//! Batch import: a folder of sub-folders, one vehicle pack each. The folder name
//! becomes the vehicle name, and the reference defaults to the model the pack replaces
//! (the `.dff` name), so most packs need no questions.

use crate::pack::{self, PackFiles};
use crate::references::VehicleRef;
use crate::validators;
use crate::weapons::WeaponRef;
use std::{collections::BTreeSet, fs, path::Path, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub folder: PathBuf,
    pub name: String,
    pub short: String,
    /// Original model the pack was made to replace, if the `.dff` name says so.
    pub reference: Option<String>,
    /// `reference` is a weapon name (`weapon.dat`), not a vehicle model.
    pub weapon: bool,
    pub files: PackFiles,
}

/// `.dff` named after an original vehicle (`sanchez.dff`) means "clone that one".
pub fn guess_reference(files: &PackFiles, refs: &[VehicleRef]) -> Option<String> {
    let stem = files
        .dff
        .as_ref()?
        .file_stem()?
        .to_string_lossy()
        .to_lowercase();
    refs.iter()
        .find(|r| r.model == stem)
        .map(|r| r.model.clone())
}

/// `.dff` named after an original weapon model (`gun_dildo1.dff`) means "clone that one".
pub fn guess_weapon(files: &PackFiles, weapons: &[WeaponRef]) -> Option<String> {
    let stem = files
        .dff
        .as_ref()?
        .file_stem()?
        .to_string_lossy()
        .to_lowercase();
    weapons
        .iter()
        .find(|w| w.model.to_lowercase() == stem)
        .map(|w| w.name.clone())
}

/// First free short: the name's own letters, then with a number (`launch` -> `launc2`).
pub fn suggest_short(name: &str, taken: &BTreeSet<String>) -> String {
    let base = match validators::short_from(name) {
        s if s.is_empty() || !s.as_bytes()[0].is_ascii_lowercase() => format!("car{}", s),
        s => s,
    };
    free_name(&base, validators::SHORT_MAX, taken)
}

/// Same for weapons: up to 15 characters, `_` allowed (`Desert Eagle Gold` -> `desert_eagle_go`).
pub fn suggest_weapon_name(name: &str, taken: &BTreeSet<String>) -> String {
    let words: Vec<String> = crate::text::fold_ascii(name)
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect();
    let base = words.join("_");
    let base = match base.as_bytes().first() {
        Some(b) if b.is_ascii_lowercase() => base,
        _ => format!("wpn{base}"),
    };
    free_name(&base, 15, taken)
}

fn free_name(base: &str, max: usize, taken: &BTreeSet<String>) -> String {
    let base: String = base.chars().take(max).collect();
    let base = base.trim_end_matches('_').to_string();
    if !taken.contains(&base) {
        return base;
    }
    (2..100)
        .map(|n| {
            let tail = n.to_string();
            let keep = max - tail.len();
            format!("{}{tail}", base.chars().take(keep).collect::<String>())
        })
        .find(|s| !taken.contains(s))
        .unwrap_or(base)
}

fn candidate(
    folder: &Path,
    refs: &[VehicleRef],
    taken: &mut BTreeSet<String>,
) -> Option<Candidate> {
    let files = pack::discover(folder, "");
    files.dff.as_ref()?;
    files.txd.as_ref()?;
    let name = folder.file_name()?.to_string_lossy().into_owned();
    let short = suggest_short(&name, taken);
    taken.insert(short.clone());
    Some(Candidate {
        folder: folder.to_path_buf(),
        name,
        short,
        reference: guess_reference(&files, refs),
        weapon: false,
        files,
    })
}

/// A pack folder itself gives one candidate; otherwise every sub-folder that holds a
/// `.dff` and a `.txd`. `taken` are shorts already used (game models, installed mods).
pub fn scan(dir: &Path, refs: &[VehicleRef], taken: &BTreeSet<String>) -> Vec<Candidate> {
    let mut taken = taken.clone();
    if let Some(c) = candidate(dir, refs, &mut taken) {
        return vec![c];
    }
    let mut subs: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    subs.sort();
    subs.iter()
        .filter_map(|s| candidate(s, refs, &mut taken))
        .collect()
}

/// Everything the user dropped or picked: pack folders, folders of packs, loose `.dff` +
/// `.txd` pairs. A `.dff` named after an original weapon model becomes a weapon.
/// `taken` grows with the shorts handed out, so a whole drop gets distinct names.
pub fn from_paths(
    paths: &[PathBuf],
    refs: &[VehicleRef],
    weapons: &[WeaponRef],
    taken: &mut BTreeSet<String>,
) -> Vec<Candidate> {
    let mut found: Vec<Candidate> = vec![];
    for p in paths.iter().filter(|p| p.is_dir()) {
        let c = scan(p, refs, taken);
        taken.extend(c.iter().map(|c| c.short.clone()));
        found.extend(c);
    }
    let is = |p: &Path, ext: &str| p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext));
    for dff in paths.iter().filter(|p| is(p, "dff")) {
        let Some(txd) = paths
            .iter()
            .find(|p| is(p, "txd") && p.file_stem() == dff.file_stem())
        else {
            continue;
        };
        let name = dff
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let files = PackFiles {
            dff: Some(dff.clone()),
            txd: Some(txd.clone()),
            ..Default::default()
        };
        let short = suggest_short(&name, taken);
        taken.insert(short.clone());
        found.push(Candidate {
            folder: dff.parent().map(Path::to_path_buf).unwrap_or_default(),
            reference: guess_reference(&files, refs),
            weapon: false,
            name,
            short,
            files,
        });
    }
    for c in found.iter_mut().filter(|c| c.reference.is_none()) {
        if let Some(w) = guess_weapon(&c.files, weapons) {
            c.reference = Some(w);
            c.weapon = true;
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn dff_named_after_a_weapon_model_picks_that_weapon() {
        let w = |name: &str, model: &str| WeaponRef {
            name: name.into(),
            melee: true,
            model_id: 321,
            model: model.into(),
            dat_lines: vec![],
            ide_line: vec![],
            object_line: None,
            config_line: vec![],
            associated: vec![],
        };
        let weapons = [w("DILDO1", "gun_dildo1"), w("KATANA", "katana")];
        let files = |dff: &str| PackFiles {
            dff: Some(PathBuf::from(dff)),
            ..Default::default()
        };
        assert_eq!(
            guess_weapon(&files("x/GUN_DILDO1.dff"), &weapons).as_deref(),
            Some("DILDO1")
        );
        assert_eq!(guess_weapon(&files("x/luger.dff"), &weapons), None);
    }

    #[test]
    fn shorts_avoid_taken_names() {
        assert_eq!(suggest_short("GT R34 Skyline", &set(&[])), "gtr34sk");
        assert_eq!(
            suggest_short("Launch", &set(&["launch"])),
            "launch2".chars().take(7).collect::<String>()
        );
        assert_eq!(
            suggest_short("Launch", &set(&["launch", "launch2"])),
            "launch3"
        );
        assert_eq!(
            suggest_short("Marinha do Brasil", &set(&["marinha"])),
            "marinh2"
        );
        assert_eq!(suggest_short("800 GS", &set(&[])), "car800g");
        assert_eq!(suggest_short("???", &set(&[])), "car");
    }

    #[test]
    fn weapon_names_allow_underscores_and_fifteen_characters() {
        assert_eq!(
            suggest_weapon_name("Desert Eagle Gold", &set(&[])),
            "desert_eagle_go"
        );
        assert_eq!(
            suggest_weapon_name("Pistola Parabéllum", &set(&["pistola_parabel"])),
            "pistola_parabe2"
        );
        assert_eq!(suggest_weapon_name("9mm", &set(&[])), "wpn9mm");
    }

    #[test]
    fn folder_of_packs_gets_one_candidate_per_subfolder() {
        let d = tempfile::tempdir().unwrap();
        for f in [
            "Moto GS/sanchez.dff",
            "Moto GS/sanchez.txd",
            "Barco/launch.dff",
            "Barco/launch.txd",
            "Vazia/readme.txt",
        ] {
            let p = d.path().join(f);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, b"").unwrap();
        }
        let c = scan(d.path(), &[], &set(&["barco"]));
        let view: Vec<_> = c
            .iter()
            .map(|c| (c.name.as_str(), c.short.as_str(), c.reference.clone()))
            .collect();
        assert_eq!(
            view,
            [("Barco", "barco2", None), ("Moto GS", "motogs", None)]
        );
    }
}
