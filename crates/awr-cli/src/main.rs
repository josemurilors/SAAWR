use awr_core::{config::AppSettings, inventory, paths};
use std::{path::PathBuf, process::ExitCode};

const USAGE: &str = "uso: awr-cli status [--gta <pasta do GTA SA>]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let gta = args
        .iter()
        .position(|a| a == "--gta")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .or_else(|| AppSettings::load().gta_path);
    let result = match (args.first().map(String::as_str), gta) {
        (Some("status"), Some(gta)) => status(gta),
        (Some("status"), None) => Err("informe --gta <pasta>".to_string()),
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("erro: {e}");
            ExitCode::FAILURE
        }
    }
}

fn status(gta: PathBuf) -> Result<(), String> {
    let info = paths::detect_install(&gta).map_err(|e| e.to_string())?;
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
