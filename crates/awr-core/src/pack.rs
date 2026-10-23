//! Finds the parts of a downloaded vehicle pack by its usual layout:
//! `{name}.dff` + `{name}.txd` at the top, `Paintjobs/`, `Tuning Parts/`, `IVF Colors/ImVehFt/colors/*.ivfc`.

use crate::readme::{self, FoundHandling, FoundWeapon};
use std::{fs, path::Path, path::PathBuf};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PackFiles {
    pub dff: Option<PathBuf>,
    pub txd: Option<PathBuf>,
    pub paintjobs: Option<PathBuf>,
    pub parts: Option<PathBuf>,
    pub ivfc: Option<PathBuf>,
    /// Handling the pack ships in its readme, if any.
    pub handling: Option<FoundHandling>,
    /// Weapon lines the pack ships in its readme, if any.
    pub weapon: Option<FoundWeapon>,
}

/// Files of `dir` (not recursive) with the given extension, sorted.
fn with_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)))
        .collect();
    v.sort();
    v
}

/// The `{reference}.ext` file if present, else the first file with that extension.
fn pick(dir: &Path, reference: &str, ext: &str) -> Option<PathBuf> {
    let all = with_ext(dir, ext);
    let named = all.iter().find(|p| {
        p.file_stem()
            .is_some_and(|s| s.eq_ignore_ascii_case(reference))
    });
    named.or(all.first()).cloned()
}

fn sub_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir() && p.file_name().is_some_and(|n| n.eq_ignore_ascii_case(name)))
}

pub fn discover(dir: &Path, reference: &str) -> PackFiles {
    let ivfc = sub_dir(dir, "IVF Colors")
        .and_then(|d| sub_dir(&d, "ImVehFt"))
        .and_then(|d| sub_dir(&d, "colors"))
        .and_then(|d| with_ext(&d, "ivfc").into_iter().next());
    let dff = pick(dir, reference, "dff");
    let stem = dff
        .as_ref()
        .and_then(|d| d.file_stem())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    PackFiles {
        handling: readme::find_handling(dir, &stem),
        weapon: readme::find_weapon(dir),
        dff,
        txd: pick(dir, reference, "txd"),
        paintjobs: sub_dir(dir, "Paintjobs"),
        parts: sub_dir(dir, "Tuning Parts"),
        ivfc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(p: PathBuf) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, b"").unwrap();
    }

    #[test]
    fn finds_the_usual_layout() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        for f in [
            "uranus.dff",
            "uranus.txd",
            "Paintjobs/uranus1.txd",
            "Tuning Parts/exh_a_u.dff",
            "IVF Colors/ImVehFt/colors/vw.ivfc",
        ] {
            touch(r.join(f));
        }
        let p = discover(r, "uranus");
        assert_eq!(p.dff, Some(r.join("uranus.dff")));
        assert_eq!(p.txd, Some(r.join("uranus.txd")));
        assert_eq!(p.paintjobs, Some(r.join("Paintjobs")));
        assert_eq!(p.parts, Some(r.join("Tuning Parts")));
        assert_eq!(p.ivfc, Some(r.join("IVF Colors/ImVehFt/colors/vw.ivfc")));
    }

    #[test]
    fn falls_back_to_the_only_dff_and_reports_missing_extras() {
        let d = tempfile::tempdir().unwrap();
        touch(d.path().join("bmw.dff"));
        touch(d.path().join("bmw.txd"));
        let p = discover(d.path(), "sanchez");
        assert_eq!(p.dff, Some(d.path().join("bmw.dff")));
        assert_eq!((p.paintjobs, p.parts, p.ivfc), (None, None, None));
    }
}
