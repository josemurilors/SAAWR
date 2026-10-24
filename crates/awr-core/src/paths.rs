//! Finds the game, ModLoader, FLA and OLA on disk.

use crate::error::{AwrError, NotFoundKind, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const FLA_INI: &str = "fastman92limitAdjuster_GTASA.ini";
const OLA_ASIS: [&str; 2] = ["III.VC.SA.LimitAdjuster.asi", "OpenLimitAdjuster.asi"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallInfo {
    pub root: PathBuf,
    pub modloader: Option<PathBuf>,
    /// Folder holding the FLA `.asi`; its `.ini` and `data\` sit next to it.
    pub fla_dir: Option<PathBuf>,
    pub ola_asi: Option<PathBuf>,
}

impl InstallInfo {
    pub fn fla_ini(&self) -> Option<PathBuf> {
        self.fla_dir
            .as_ref()
            .map(|d| d.join(FLA_INI))
            .filter(|p| p.is_file())
    }

    pub fn fla_data(&self) -> Option<PathBuf> {
        self.fla_dir
            .as_ref()
            .map(|d| d.join("data"))
            .filter(|p| p.is_dir())
    }

    /// RZL-Trainer's list of added weapons, if the trainer is installed.
    pub fn trainer_ini(&self) -> Option<PathBuf> {
        Some(self.root.join("CLEO/RZL-Trainer/Add-on Weapons.ini")).filter(|p| p.is_file())
    }

    pub fn data(&self) -> PathBuf {
        self.root.join("data")
    }
}

pub fn detect_install(root: &Path) -> Result<InstallInfo> {
    if !root.join("gta_sa.exe").is_file() {
        return Err(AwrError::NotFound(NotFoundKind::Game));
    }
    let modloader = Some(root.join("modloader")).filter(|p| p.is_dir());
    // the asi loader reads plugins from the root, and ModLoader from its first-level folders
    let mut dirs = vec![root.to_path_buf()];
    if let Some(ml) = &modloader {
        dirs.extend(subdirs(ml));
    }
    let fla_dir = dirs
        .iter()
        .find(|d| has_file(d, |n| n.ends_with("fastman92limitadjuster.asi")))
        .cloned();
    let ola_asi = dirs
        .iter()
        .flat_map(|d| OLA_ASIS.iter().map(move |n| d.join(n)))
        .find(|p| p.is_file());
    Ok(InstallInfo {
        root: root.to_path_buf(),
        modloader,
        fla_dir,
        ola_asi,
    })
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(dir) else {
        return vec![];
    };
    let mut out: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    out.sort();
    out
}

fn has_file(dir: &Path, pred: impl Fn(&str) -> bool) -> bool {
    fs::read_dir(dir).is_ok_and(|rd| {
        rd.flatten()
            .any(|e| pred(&e.file_name().to_string_lossy().to_ascii_lowercase()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(p: &Path) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, b"").unwrap();
    }

    #[test]
    fn finds_fla_and_ola_inside_modloader() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        touch(&r.join("gta_sa.exe"));
        touch(&r.join("modloader.asi"));
        touch(&r.join("modloader/$fastman92 limit adjuster/$fastman92limitAdjuster.asi"));
        touch(&r.join("modloader/$fastman92 limit adjuster").join(FLA_INI));
        fs::create_dir_all(r.join("modloader/$fastman92 limit adjuster/data")).unwrap();
        touch(&r.join("modloader/Open Limit Adjuster/III.VC.SA.LimitAdjuster.asi"));

        let info = detect_install(r).unwrap();
        assert_eq!(info.modloader, Some(r.join("modloader")));
        assert_eq!(
            info.fla_dir,
            Some(r.join("modloader/$fastman92 limit adjuster"))
        );
        assert!(info.fla_ini().is_some() && info.fla_data().is_some());
        assert_eq!(
            info.ola_asi,
            Some(r.join("modloader/Open Limit Adjuster/III.VC.SA.LimitAdjuster.asi"))
        );
    }

    #[test]
    fn finds_fla_in_game_root_and_reports_missing_parts() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        touch(&r.join("gta_sa.exe"));
        touch(&r.join("fastman92limitAdjuster.asi"));
        let info = detect_install(r).unwrap();
        assert_eq!(info.fla_dir.as_deref(), Some(r));
        assert_eq!(info.fla_ini(), None);
        assert_eq!(info.modloader, None);
        assert_eq!(info.ola_asi, None);
    }

    #[test]
    fn rejects_folder_without_game() {
        let d = tempfile::tempdir().unwrap();
        assert!(matches!(
            detect_install(d.path()),
            Err(AwrError::NotFound(NotFoundKind::Game))
        ));
    }
}
