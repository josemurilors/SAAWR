//! Rebuilds the mod list from disk; there is no other database.

use crate::meta::{self, Meta};
use std::{fs, path::Path, path::PathBuf};

pub const CARS_DIR: &str = "awr-cars";
pub const WEAPONS_DIR: &str = "awr-weapons";
/// ModLoader skips folders whose name starts with dot + space.
pub const DISABLED_PREFIX: &str = ". ";

/// A mod is identified by its folder, never by its ID (IDs can collide).
pub type ModKey = PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledMod {
    pub key: ModKey,
    /// `{id}` from the folder name `{id}-{short}`, if it has one.
    pub id: Option<u32>,
    pub enabled: bool,
    /// None = foreign: no (valid) meta.json, listed but never touched.
    pub meta: Option<Meta>,
}

impl InstalledMod {
    pub fn is_foreign(&self) -> bool {
        self.meta.is_none()
    }
}

pub fn scan(modloader: &Path) -> Vec<InstalledMod> {
    let mut out: Vec<InstalledMod> = [CARS_DIR, WEAPONS_DIR]
        .iter()
        .flat_map(|d| read_dirs(&modloader.join(d)))
        .map(|key| {
            let (enabled, bare) = split_disabled(&key);
            let meta = match meta::read(&key) {
                Ok(m) => Some(m),
                Err(e) => {
                    if key.join(meta::FILE).exists() {
                        tracing::warn!("{e}; tratando como estrangeiro");
                    }
                    None
                }
            };
            InstalledMod {
                id: folder_id(&bare),
                enabled,
                meta,
                key,
            }
        })
        .collect();
    out.sort_by_key(|m| (m.id, m.key.clone()));
    out
}

/// `(enabled, name without the disabled prefix)`.
pub fn split_disabled(dir: &Path) -> (bool, String) {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    match name.strip_prefix(DISABLED_PREFIX) {
        Some(bare) => (false, bare.to_string()),
        None => (true, name),
    }
}

/// `12500-gtr34` -> 12500.
pub fn folder_id(name: &str) -> Option<u32> {
    name.split_once('-')?.0.parse().ok()
}

pub(crate) fn read_dirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(dir) else {
        return vec![];
    };
    rd.flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_awr_disabled_and_foreign_folders() {
        let d = tempfile::tempdir().unwrap();
        let cars = d.path().join(CARS_DIR);
        for name in [
            "12500-gtr34",
            ". 12501-supra",
            "12502-alien",
            "staging-junk",
        ] {
            fs::create_dir_all(cars.join(name)).unwrap();
        }
        meta::write(&cars.join("12500-gtr34"), &meta::sample(12500, "gtr34")).unwrap();
        meta::write(&cars.join(". 12501-supra"), &meta::sample(12501, "supra")).unwrap();
        fs::write(cars.join("12502-alien").join(meta::FILE), b"not json").unwrap();

        let inv = scan(d.path());
        let view: Vec<_> = inv
            .iter()
            .map(|m| (m.id, m.enabled, m.is_foreign()))
            .collect();
        assert_eq!(
            view,
            [
                (None, true, true),
                (Some(12500), true, false),
                (Some(12501), false, false),
                (Some(12502), true, true)
            ]
        );
    }

    #[test]
    fn missing_modloader_dirs_mean_empty_inventory() {
        let d = tempfile::tempdir().unwrap();
        assert!(scan(d.path()).is_empty());
    }
}
