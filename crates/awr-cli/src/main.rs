use awr_core::{
    batch,
    checker::{self, Severity},
    config::AppSettings,
    injector::{InjectVehicle, Injector, Traffic},
    inventory, meta, pack,
    paths::{self, InstallInfo},
    references, validators,
    weapons::{self, InjectWeapon, Special},
};
use std::{path::Path, path::PathBuf, process::ExitCode, time::SystemTime};

const USAGE: &str = "uso: awr-cli <comando> [--gta <pasta do GTA SA>]
  status
  doctor
  inject --name <nome> --ref <modelo, ex. uranus> (--pack <pasta do mod> | --dff <arq> --txd <arq>)
         [--paintjobs <pasta>] [--parts <pasta>] [--ivfc <arq>] [--short <nome curto>] [--traffic auto|none|<n° do grupo>] [--no-audio] [--no-pack-handling] [--cleo]
  groups
  weapons
  inject-weapon --short <nome> --ref <ARMA, ex. DESERT_EAGLE> (--pack <pasta> | --dff <arq> --txd <arq>) [--no-pack-lines] [--special silencer|jetpack|binoculars]
  batch <pasta com subpastas> [--ref <modelo padrão>] [--yes]
  enable <pasta do mod>
  disable <pasta do mod>
  remove <pasta do mod>
  repair";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("erro: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Value after `--flag`.
fn opt<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let i = args.iter().position(|a| a == flag)?;
    args.get(i + 1).map(String::as_str)
}

fn need<'a>(args: &'a [String], flag: &str) -> Result<&'a str, String> {
    opt(args, flag).ok_or_else(|| format!("falta {flag}\n{USAGE}"))
}

fn run(args: &[String]) -> Result<(), String> {
    let settings = AppSettings::load();
    let gta = opt(args, "--gta")
        .map(PathBuf::from)
        .or_else(|| settings.gta_path.clone())
        .ok_or("informe --gta <pasta>")?;
    let info = paths::detect_install(&gta).map_err(|e| e.to_string())?;
    let cmd = args.first().map(String::as_str).unwrap_or_default();
    if cmd == "status" {
        return status(&info);
    }
    if cmd == "doctor" {
        return doctor(&info);
    }
    if cmd == "groups" {
        let file =
            awr_core::fsutil::read(&info.data().join("cargrp.dat")).map_err(|e| e.to_string())?;
        for g in awr_core::cargrp::groups(&file) {
            println!("{:>3}  {}  ({} carros)", g.index, g.label, g.len);
        }
        return Ok(());
    }
    let refs = references::load_vehicles(&info.data(), info.fla_data().as_deref())
        .map_err(|e| e.to_string())?;
    let weapons = info
        .fla_data()
        .and_then(|d| weapons::load_weapons(&info.data(), &d).ok())
        .unwrap_or_default();
    let inj = Injector {
        info: &info,
        refs: &refs,
        weapons: &weapons,
        id_floor: settings.id_floor_vehicle,
    };
    let folder = || {
        args.get(1)
            .filter(|a| !a.starts_with("--"))
            .map(PathBuf::from)
            .ok_or_else(|| format!("informe a pasta do mod\n{USAGE}"))
    };
    match cmd {
        "inject" => {
            let name = need(args, "--name")?;
            let reference = need(args, "--ref")?;
            let short = opt(args, "--short")
                .map(String::from)
                .unwrap_or_else(|| validators::short_from(name));
            let secs = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            // --pack <folder> finds the car and its extras by the usual pack layout
            let found = opt(args, "--pack")
                .map(|p| pack::discover(Path::new(p), reference))
                .unwrap_or_default();
            let flag = |f: &str| opt(args, f).map(PathBuf::from);
            let dff = flag("--dff").or(found.dff).ok_or("falta --dff ou --pack")?;
            let txd = flag("--txd").or(found.txd).ok_or("falta --txd ou --pack")?;
            let paintjobs = flag("--paintjobs").or(found.paintjobs);
            let parts = flag("--parts").or(found.parts);
            let ivfc = flag("--ivfc").or(found.ivfc);
            let found_cleo = found.cleo;
            let no_pack_handling = args.iter().any(|a| a == "--no-pack-handling");
            let own = found.handling.filter(|_| !no_pack_handling);
            let key = inj
                .inject_vehicle(&InjectVehicle {
                    name,
                    short: &short,
                    dff: &dff,
                    txd: &txd,
                    reference,
                    traffic: traffic(opt(args, "--traffic"))?,
                    audio: !args.iter().any(|a| a == "--no-audio"),
                    created: &meta::rfc3339(secs),
                    paintjobs: paintjobs.as_deref(),
                    parts: parts.as_deref(),
                    ivfc: ivfc.as_deref(),
                    handling: own.as_ref(),
                    cleo: found_cleo
                        .as_deref()
                        .filter(|_| args.iter().any(|a| a == "--cleo")),
                })
                .map_err(|e| e.to_string())?;
            println!("injetado: {}", key.display());
        }
        "batch" => {
            let dir = folder()?;
            let fallback = opt(args, "--ref");
            let found = batch::scan(&dir, &refs, &inj.taken_shorts());
            let yes = args.iter().any(|a| a == "--yes");
            let yes_own_off = args.iter().any(|a| a == "--no-pack-handling");
            let secs = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            for c in &found {
                let Some(reference) = c.reference.as_deref().or(fallback) else {
                    println!("pulado (sem referência, use --ref): {}", c.name);
                    continue;
                };
                println!("{} -> {} (ref {reference})", c.name, c.short);
                if !yes {
                    continue;
                }
                let (Some(dff), Some(txd)) = (&c.files.dff, &c.files.txd) else {
                    continue;
                };
                let r = inj.inject_vehicle(&InjectVehicle {
                    name: &c.name,
                    short: &c.short,
                    dff,
                    txd,
                    reference,
                    traffic: Traffic::Auto,
                    audio: true,
                    created: &meta::rfc3339(secs),
                    paintjobs: c.files.paintjobs.as_deref(),
                    parts: c.files.parts.as_deref(),
                    ivfc: c.files.ivfc.as_deref(),
                    handling: c.files.handling.as_ref().filter(|_| !yes_own_off),
                    cleo: None,
                });
                match r {
                    Ok(k) => println!("  injetado: {}", k.display()),
                    Err(e) => println!("  erro: {e}"),
                }
            }
            if !yes {
                println!("(simulação; use --yes para injetar)");
            }
        }
        "weapons" => {
            for w in &weapons {
                let kind = if w.melee { "corpo a corpo" } else { "fogo" };
                println!("{:<18} {kind}  (modelo {})", w.name, w.model);
            }
        }
        "inject-weapon" => {
            let short = need(args, "--short")?;
            let reference = need(args, "--ref")?;
            let found = opt(args, "--pack")
                .map(|p| pack::discover(Path::new(p), ""))
                .unwrap_or_default();
            let flag = |f: &str| opt(args, f).map(PathBuf::from);
            let dff = flag("--dff").or(found.dff).ok_or("falta --dff ou --pack")?;
            let txd = flag("--txd").or(found.txd).ok_or("falta --txd ou --pack")?;
            let secs = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            let key = inj
                .inject_weapon(&InjectWeapon {
                    name: opt(args, "--name").unwrap_or(short),
                    short,
                    dff: &dff,
                    txd: &txd,
                    reference,
                    created: &meta::rfc3339(secs),
                    special: Special::parse(opt(args, "--special").unwrap_or_default()),
                    own: found
                        .weapon
                        .as_ref()
                        .filter(|_| !args.iter().any(|a| a == "--no-pack-lines")),
                })
                .map_err(|e| e.to_string())?;
            println!("injetada: {}", key.display());
            if !weapons::has_icon(&key.join(format!("{short}.txd")), short) {
                println!("aviso: o .txd não tem a textura '{short}icon' (ícone do HUD); renomeie a textura do ícone no Magic TXD");
            }
        }
        "enable" | "disable" => {
            let key = inj
                .set_enabled(&folder()?, cmd == "enable")
                .map_err(|e| e.to_string())?;
            println!("{}", key.display());
        }
        "remove" => {
            inj.remove(&folder()?).map_err(|e| e.to_string())?;
            println!("removido");
        }
        "repair" => {
            let n = inj.repair().map_err(|e| e.to_string())?;
            println!("reparado ({n} injeção(ões) interrompida(s) limpa(s))");
        }
        _ => return Err(USAGE.to_string()),
    }
    Ok(())
}

/// `auto` (default), `none`, or the group number from `awr-cli groups`.
fn traffic(v: Option<&str>) -> Result<Traffic, String> {
    match v {
        None | Some("auto") => Ok(Traffic::Auto),
        Some("none") => Ok(Traffic::None),
        Some(n) => n
            .parse()
            .map(Traffic::Group)
            .map_err(|_| format!("--traffic inválido: {n}")),
    }
}

fn doctor(info: &InstallInfo) -> Result<(), String> {
    let findings = checker::check(info);
    for f in &findings {
        let tag = match f.severity {
            Severity::Error => "ERRO",
            Severity::Warn => "aviso",
        };
        println!("{tag}: {}", f.msg);
    }
    if checker::has_errors(&findings) {
        return Err("a instalação tem erros".into());
    }
    if findings.is_empty() {
        println!("tudo certo");
    }
    Ok(())
}

fn status(info: &InstallInfo) -> Result<(), String> {
    let show = |label: &str, p: Option<PathBuf>| match p {
        Some(p) => println!("✓ {label}: {}", p.display()),
        None => println!("✗ {label}: não encontrado"),
    };
    println!("GTA SA: {}", info.root.display());
    show("ModLoader", info.modloader.clone());
    show("FLA", info.fla_dir.clone());
    show("FLA .ini", info.fla_ini());
    show("FLA data", info.fla_data());
    show("OLA", info.ola_asi.clone());

    let Some(ml) = &info.modloader else {
        return Ok(());
    };
    let mods = inventory::scan(ml);
    println!("\nInventário: {} mod(s)", mods.len());
    for m in mods {
        let id = m.id.map_or("-".into(), |i| i.to_string());
        let name = m.meta.as_ref().map_or("(estrangeiro)", |x| x.name.as_str());
        let state = if m.enabled { "ON " } else { "OFF" };
        println!("  {id:>6}  {state}  {name}  [{}]", m.key.display());
    }
    Ok(())
}
