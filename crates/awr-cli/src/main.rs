use awr_core::{
    checker::{self, Severity},
    config::AppSettings,
    injector::{InjectVehicle, Injector},
    inventory, meta,
    paths::{self, InstallInfo},
    references, validators,
};
use std::{path::PathBuf, process::ExitCode, time::SystemTime};

const USAGE: &str = "uso: awr-cli <comando> [--gta <pasta do GTA SA>]
  status
  doctor
  inject --name <nome> --ref <modelo, ex. uranus> (--pack <pasta do mod> | --dff <arq> --txd <arq>)
         [--paintjobs <pasta>] [--parts <pasta>] [--ivfc <arq>] [--short <nome curto>] [--no-traffic] [--no-audio]
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
    let refs = references::load_vehicles(&info.data(), info.fla_data().as_deref())
        .map_err(|e| e.to_string())?;
    let inj = Injector {
        info: &info,
        refs: &refs,
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
            let pack = opt(args, "--pack").map(PathBuf::from);
            let find = |flag: &str, rel: String| {
                opt(args, flag)
                    .map(PathBuf::from)
                    .or_else(|| pack.as_ref().map(|p| p.join(rel)).filter(|p| p.exists()))
            };
            let dff = find("--dff", format!("{reference}.dff")).ok_or("falta --dff ou --pack")?;
            let txd = find("--txd", format!("{reference}.txd")).ok_or("falta --txd ou --pack")?;
            let paintjobs = find("--paintjobs", "Paintjobs".into());
            let parts = find("--parts", "Tuning Parts".into());
            let ivfc = find("--ivfc", "IVF Colors/ImVehFt/colors".into()).and_then(|p| {
                if p.is_dir() {
                    std::fs::read_dir(p)
                        .ok()?
                        .flatten()
                        .map(|e| e.path())
                        .find(|f| {
                            f.extension()
                                .is_some_and(|e| e.eq_ignore_ascii_case("ivfc"))
                        })
                } else {
                    Some(p)
                }
            });
            let key = inj
                .inject_vehicle(&InjectVehicle {
                    name,
                    short: &short,
                    dff: &dff,
                    txd: &txd,
                    reference,
                    traffic: !args.iter().any(|a| a == "--no-traffic"),
                    audio: !args.iter().any(|a| a == "--no-audio"),
                    created: &meta::rfc3339(secs),
                    paintjobs: paintjobs.as_deref(),
                    parts: parts.as_deref(),
                    ivfc: ivfc.as_deref(),
                })
                .map_err(|e| e.to_string())?;
            println!("injetado: {}", key.display());
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
