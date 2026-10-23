//! Health checks (spec §4.4). `Error` blocks injection; `Warn` is shown, never blocks.

use crate::fla;
use crate::fsutil;
use crate::inventory::{read_dirs, split_disabled};
use crate::paths::InstallInfo;
use std::{fs, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    pub msg: String,
}

pub fn has_errors(findings: &[Finding]) -> bool {
    findings.iter().any(|f| f.severity == Severity::Error)
}

pub fn check(info: &InstallInfo) -> Vec<Finding> {
    let mut out = vec![];
    let mut add = |severity, msg: String| out.push(Finding { severity, msg });
    let Some(ml) = &info.modloader else {
        add(
            Severity::Error,
            "ModLoader não encontrado (pasta modloader ausente)".into(),
        );
        return out;
    };
    if info.fla_dir.is_none() {
        add(
            Severity::Error,
            "fastman92 Limit Adjuster (.asi) não encontrado".into(),
        );
    } else if let Some(ini) = info.fla_ini() {
        // with OLA these must stay `#`-disabled, OLA owns the model limits
        let bytes = fsutil::read(&ini).unwrap_or_default();
        for key in fla::active_ola_keys(&bytes) {
            add(
                Severity::Warn,
                format!("'{key}' está ativa no .ini do FLA; com o OLA deve ficar com #"),
            );
        }
    } else {
        add(
            Severity::Error,
            "fastman92limitAdjuster_GTASA.ini não encontrado".into(),
        );
    }
    if info.ola_asi.is_none() {
        add(
            Severity::Error,
            "Open Limit Adjuster (III.VC.SA.LimitAdjuster.asi) não encontrado; instale-o antes de injetar".into(),
        );
    }
    for other in other_cargrps(ml) {
        add(
            Severity::Warn,
            format!(
                "outro cargrp.dat no modloader ({}); o tráfego do AWR só funciona sem ele",
                other.display()
            ),
        );
    }
    if game_running(&info.root) {
        add(
            Severity::Warn,
            "o GTA SA parece estar aberto; feche-o antes de alterar mods".into(),
        );
    }
    out
}

/// `cargrp.dat` files ModLoader would load besides the AWR traffic copy (two levels deep).
fn other_cargrps(modloader: &Path) -> Vec<std::path::PathBuf> {
    let mut dirs = vec![modloader.to_path_buf()];
    for top in read_dirs(modloader) {
        dirs.push(top.clone());
        dirs.extend(read_dirs(&top));
    }
    dirs.into_iter()
        .filter(|d| {
            let (enabled, _) = split_disabled(d);
            enabled
                && d.file_name()
                    .is_none_or(|n| n != crate::injector::TRAFFIC_DIR)
        })
        .map(|d| d.join("cargrp.dat"))
        .filter(|p| p.is_file())
        .collect()
}

/// Windows refuses write access to a running executable.
/// shortcut: a read-only gta_sa.exe also reads as "running"; upgrade if it shows up.
pub fn game_running(root: &Path) -> bool {
    fs::OpenOptions::new()
        .write(true)
        .open(root.join("gta_sa.exe"))
        .is_err()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::detect_install;

    fn touch(p: &Path, body: &[u8]) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }

    /// Minimal healthy install; returns it plus the FLA .ini path.
    fn healthy(r: &Path, ini: &[u8]) -> InstallInfo {
        touch(&r.join("gta_sa.exe"), b"");
        let fla = r.join("modloader/FLA");
        touch(&fla.join("$fastman92limitAdjuster.asi"), b"");
        touch(&fla.join(crate::paths::FLA_INI), ini);
        touch(&r.join("modloader/OLA/III.VC.SA.LimitAdjuster.asi"), b"");
        detect_install(r).unwrap()
    }

    #[test]
    fn healthy_install_has_no_findings() {
        let d = tempfile::tempdir().unwrap();
        let info = healthy(d.path(), b"[IDE LIMITS]\r\n#Vehicle Models = 212\r\n");
        assert_eq!(check(&info), []);
    }

    #[test]
    fn missing_parts_are_errors() {
        let d = tempfile::tempdir().unwrap();
        touch(&d.path().join("gta_sa.exe"), b"");
        let info = detect_install(d.path()).unwrap();
        assert!(has_errors(&check(&info)));

        touch(&d.path().join("modloader/x"), b"");
        let msgs: Vec<_> = check(&detect_install(d.path()).unwrap())
            .into_iter()
            .map(|f| f.msg)
            .collect();
        assert!(msgs.iter().any(|m| m.contains("fastman92")));
        assert!(msgs.iter().any(|m| m.contains("Open Limit Adjuster")));
    }

    #[test]
    fn warns_about_active_model_limits_and_foreign_cargrp() {
        let d = tempfile::tempdir().unwrap();
        let info = healthy(d.path(), b"[IDE LIMITS]\r\nVehicle Models = 212\r\n");
        touch(&d.path().join("modloader/other/cargrp.dat"), b"");
        touch(&d.path().join("modloader/awr-transito/cargrp.dat"), b"");
        touch(&d.path().join("modloader/. off/cargrp.dat"), b"");
        let f = check(&info);
        assert!(!has_errors(&f));
        assert_eq!(f.len(), 2, "{f:?}");
        assert!(f.iter().any(|f| f.msg.contains("Vehicle Models")));
        assert!(f.iter().any(|f| f.msg.contains("other")));
    }
}
