//! Gives a `meta.json` back to a mod folder that lost it (or never had one), so it can be
//! enabled, edited and removed like any other. Nothing in the game files is written here:
//! the lines the mod needs must already be there, and the notes say what was not recovered.

use crate::{
    error::{AwrError, NotFoundKind, Result},
    fsutil,
    injector::{AUDIO_CFG, TRAFFIC_DIR},
    meta::{Files, Meta, ModKind},
    paths::InstallInfo,
    references::VehicleRef,
    text::{lines, tokens},
    weapons::CONFIG,
};
use std::{fs, path::Path};

pub struct Proposal {
    pub meta: Meta,
    /// What could not be recovered; shown to the user before adopting.
    pub notes: Vec<String>,
}

fn bad(msg: impl Into<String>) -> AwrError {
    AwrError::InvalidName(msg.into())
}

fn tok(line: &[u8], i: usize) -> Option<String> {
    let t = tokens(line);
    t.get(i)
        .map(|&(s, e)| String::from_utf8_lossy(&line[s..e]).into_owned())
}

/// First data line of `section` (`cars`, `weap`) in any `.ide` of the folder.
fn ide_line(dir: &Path, section: &str) -> Option<Vec<u8>> {
    for e in fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ide")) {
            continue;
        }
        let Ok(bytes) = fs::read(&p) else { continue };
        let mut inside = false;
        for l in lines(&bytes) {
            let t = l.trim_ascii();
            if inside && t == b"end" {
                break;
            } else if !inside && t.eq_ignore_ascii_case(section.as_bytes()) {
                inside = true;
            } else if inside && !t.is_empty() && !t.starts_with(b"#") {
                return Some(t.to_vec());
            }
        }
    }
    None
}

/// The `cars` IDE line inside `linhas.txt`, where this tool (and ModLoader tutorials) keep
/// the car's lines together: `id, model, txd, type, handling, ...` (15 columns).
fn linhas_line(dir: &Path) -> Option<Vec<u8>> {
    let bytes = fs::read(dir.join("linhas.txt")).ok()?;
    let found = lines(&bytes).map(<[u8]>::trim_ascii).find(|l| {
        tokens(l).len() == 15
            && tok(l, 0).is_some_and(|x| x.parse::<u32>().is_ok())
            && tok(l, 3).is_some_and(|x| x.chars().all(|c| c.is_ascii_alphabetic()))
    });
    found.map(<[u8]>::to_vec)
}

/// True if `short` is one of the names on a non-comment line (text after `#` is ignored).
fn names_in(file: &[u8], short: &str) -> bool {
    lines(file)
        .filter(|l| !matches!(l.trim_ascii().first(), Some(b';' | b'#')))
        .any(|l| {
            let body = l.split(|&b| b == b'#').next().unwrap_or(l);
            tokens(body)
                .iter()
                .any(|&(s, e)| body[s..e].eq_ignore_ascii_case(short.as_bytes()))
        })
}

fn file_named(dir: &Path, name: &str) -> bool {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
}

/// Rebuilds the record of a folder. `reference` is the original vehicle (IDE model name) or
/// weapon (`weapon.dat` name) it was cloned from; weapons can usually tell it from their parent.
pub fn analyze(
    dir: &Path,
    info: &InstallInfo,
    refs: &[VehicleRef],
    reference: Option<&str>,
) -> Result<Proposal> {
    if let Some(l) = ide_line(dir, "weap") {
        return weapon(&l, info, reference);
    }
    let Some(l) = ide_line(dir, "cars").or_else(|| linhas_line(dir)) else {
        return Err(bad(
            "nenhum .ide com carro ('cars') ou arma ('weap') na pasta",
        ));
    };
    let (Some(id), Some(model), Some(txd), Some(kind)) =
        (tok(&l, 0), tok(&l, 1), tok(&l, 2), tok(&l, 3))
    else {
        return Err(bad("linha 'cars' do .ide incompleta"));
    };
    let id: u32 = id.parse().map_err(|_| bad("ID do veículo ilegível"))?;
    let reference = reference.ok_or_else(|| bad("informe o veículo de referência (--ref)"))?;
    if !refs.iter().any(|r| r.model.eq_ignore_ascii_case(reference)) {
        return Err(bad(format!("referência desconhecida: {reference}")));
    }
    let fxt = fs::read_dir(dir).ok().and_then(|rd| {
        rd.flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .find(|n| n.to_ascii_lowercase().ends_with(".fxt"))
    });
    let hud_name = |f: &String| {
        let bytes = fs::read(dir.join(f)).ok()?;
        let line = String::from_utf8_lossy(lines(&bytes).next()?.trim_ascii()).into_owned();
        line.split_once(' ').map(|(_, n)| n.trim().to_string())
    };
    let name = fxt
        .as_ref()
        .and_then(hud_name)
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| model.clone());
    let audio = info
        .fla_data()
        .and_then(|d| fs::read(d.join(AUDIO_CFG)).ok())
        .filter(|f| names_in(f, &model))
        .map(|_| format!("clone:{reference}"));
    let traffic_file = dir
        .parent()
        .and_then(Path::parent)
        .map(|ml| ml.join(TRAFFIC_DIR).join("cargrp.dat"));
    let traffic = traffic_file
        .and_then(|p| fs::read(p).ok())
        .filter(|f| names_in(f, &model))
        .map(|_| "auto".to_string());
    let mut notes = vec![
        "peças de tuning e paintjobs não são reconhecidas".to_string(),
        "o tráfego é registrado como 'auto' se o carro já está no cargrp".to_string(),
    ];
    if !file_named(dir, &format!("{model}.dff")) || !file_named(dir, &format!("{txd}.txd")) {
        notes.push("faltam o .dff ou o .txd com o nome do .ide".into());
    }
    Ok(Proposal {
        meta: Meta {
            v: 1,
            kind: ModKind::Vehicle,
            id,
            weapon_type_id: None,
            name,
            short: model.clone(),
            vehicle_type: Some(kind.to_ascii_lowercase()),
            reference: reference.to_string(),
            traffic_group: traffic,
            audio,
            created: String::new(),
            files: Files {
                dff: format!("{model}.dff"),
                txd: format!("{txd}.txd"),
                fxt,
            },
            parts: vec![],
            pack_handling: false,
            weapon_config: vec![],
            shop: None,
        },
        notes,
    })
}

fn weapon(ide: &[u8], info: &InstallInfo, reference: Option<&str>) -> Result<Proposal> {
    let (Some(id), Some(model), Some(txd)) = (tok(ide, 0), tok(ide, 1), tok(ide, 2)) else {
        return Err(bad("linha 'weap' do .ide incompleta"));
    };
    let id: u32 = id
        .parse()
        .map_err(|_| bad("ID do modelo da arma ilegível"))?;
    let name = model.to_ascii_uppercase();
    let cfg_path = info
        .fla_data()
        .ok_or(AwrError::NotFound(NotFoundKind::FlaIni))?
        .join(CONFIG);
    let cfg = fsutil::read(&cfg_path)?;
    let found: Vec<&[u8]> = lines(&cfg)
        .filter(|l| {
            tok(l, 1).is_some_and(|n| n.eq_ignore_ascii_case(&name))
                && tok(l, 0).is_some_and(|t| t.parse::<u32>().is_ok())
        })
        .collect();
    let main = found.first().ok_or_else(|| {
        bad(format!(
            "{name} não está no {CONFIG}; só dá para adotar armas já registradas"
        ))
    })?;
    let type_id: u32 = tok(main, 0).and_then(|t| t.parse().ok()).unwrap_or(0);
    let reference = reference
        .map(str::to_string)
        .or_else(|| tok(main, 10))
        .ok_or_else(|| bad("informe a arma de referência"))?;
    Ok(Proposal {
        meta: Meta {
            v: 1,
            kind: ModKind::Weapon,
            id,
            weapon_type_id: Some(type_id),
            name: model.clone(),
            short: model.clone(),
            vehicle_type: None,
            reference,
            traffic_group: None,
            audio: None,
            created: String::new(),
            files: Files {
                dff: format!("{model}.dff"),
                txd: format!("{txd}.txd"),
                fxt: None,
            },
            parts: vec![],
            pack_handling: false,
            weapon_config: found.iter().map(|l| fsutil::cp1252_to_string(l)).collect(),
            shop: None,
        },
        notes: vec!["a linha da arma no weapon_config é guardada como está".into()],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install(root: &Path) -> InstallInfo {
        let fla = root.join("modloader/fla");
        fs::create_dir_all(fla.join("data")).unwrap();
        fs::write(root.join("gta_sa.exe"), b"").unwrap();
        fs::write(fla.join("fastman92limitAdjuster.asi"), b"").unwrap();
        let cfg = "# cfg\r\n70    LUGER   69 1 0 24 149 194 24 1.0 PISTOL\r\n";
        fs::write(fla.join("data").join(CONFIG), cfg).unwrap();
        fs::write(fla.join("data").join(AUDIO_CFG), "; audio\r\nmycar 0 1\r\n").unwrap();
        crate::paths::detect_install(root).unwrap()
    }

    fn landstal() -> VehicleRef {
        VehicleRef {
            id: 400,
            model: "landstal".into(),
            handling_id: "LANDSTAL".into(),
            kind: "car".into(),
            ide_line: vec![],
            handling_line: vec![],
            special_line: None,
            carcols: None,
            audio_line: None,
        }
    }

    #[test]
    fn recovers_a_weapon_from_its_ide_and_the_config_line() {
        let d = tempfile::tempdir().unwrap();
        let info = install(d.path());
        let dir = d.path().join("modloader/awr-weapons/12400-luger");
        fs::create_dir_all(&dir).unwrap();
        let ide = "weap\r\n12400, luger, luger, pistol, 1, 30, 0\r\nend\r\n";
        fs::write(dir.join("luger.ide"), ide).unwrap();
        let m = analyze(&dir, &info, &[], None).unwrap().meta;
        assert_eq!(
            (m.kind, m.id, m.weapon_type_id),
            (ModKind::Weapon, 12400, Some(70))
        );
        assert_eq!(
            (m.reference.as_str(), m.short.as_str()),
            ("PISTOL", "luger")
        );
        assert!(m.weapon_config[0].starts_with("70    LUGER"));
    }

    #[test]
    fn recovers_a_car_but_needs_the_reference() {
        let d = tempfile::tempdir().unwrap();
        let info = install(d.path());
        let dir = d.path().join("modloader/awr-cars/12500-mycar");
        fs::create_dir_all(&dir).unwrap();
        let ide = "cars\r\n12500, mycar, mycar, car, MYCAR, MYCARK, null, normal, 10, 0, 0, -1, 0.7, 0.7, 0\r\nend\r\n";
        fs::write(dir.join("mycar.ide"), ide).unwrap();
        fs::write(dir.join("mycar.fxt"), "MYCAR Meu Carro\r\n").unwrap();
        assert!(analyze(&dir, &info, &[landstal()], None).is_err());
        assert!(analyze(&dir, &info, &[landstal()], Some("nope")).is_err());
        let m = analyze(&dir, &info, &[landstal()], Some("landstal"))
            .unwrap()
            .meta;
        assert_eq!((m.id, m.name.as_str()), (12500, "Meu Carro"));
        assert_eq!(m.audio.as_deref(), Some("clone:landstal"));
        assert_eq!(m.vehicle_type.as_deref(), Some("car"));
    }
}
