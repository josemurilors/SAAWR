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
    let base: String = base.chars().take(validators::SHORT_MAX).collect();
    if !taken.contains(&base) {
        return base;
    }
    (2..100)
        .map(|n| {
            let tail = n.to_string();
            let keep = validators::SHORT_MAX - tail.len();
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
