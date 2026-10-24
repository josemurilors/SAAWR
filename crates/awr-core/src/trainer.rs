//! Keeps RZL-Trainer's `Add-on Weapons.ini` in step with the enabled AWR weapons.
//! Our entries live between two marker lines; everything else in the file is left alone.

use crate::{error::Result, fsutil};
use std::path::Path;

const BEGIN: &str = "; AWR-BEGIN (managed by SA-AWR, do not edit)";
const END: &str = "; AWR-END";
/// The trainer's own limit on `[n]` sections.
const MAX_ENTRIES: u32 = 4095;

/// `(weapon type, display name)` of one enabled weapon.
pub type Entry = (u32, String);

fn section_number(line: &str) -> Option<u32> {
    line.trim()
        .strip_prefix('[')?
        .strip_suffix(']')?
        .parse()
        .ok()
}

fn weapon_id(line: &str) -> Option<u32> {
    let (k, v) = line.split_once('=')?;
    (k.trim() == "WeaponID").then(|| v.trim().parse().ok())?
}

/// Rewrites `text` so our block lists exactly `entries`; returns the new text.
pub fn sync(text: &str, entries: &[Entry]) -> String {
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut kept = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        match line.trim() {
            BEGIN => inside = true,
            END => inside = false,
            _ if !inside => kept.push(line),
            _ => {}
        }
    }
    while kept.last().is_some_and(|l| l.trim().is_empty()) {
        kept.pop();
    }
    let manual_ids: Vec<u32> = kept.iter().filter_map(|l| weapon_id(l)).collect();
    let mut next = kept
        .iter()
        .filter_map(|l| section_number(l))
        .max()
        .unwrap_or(0);
    let mut out: Vec<String> = kept.iter().map(|l| l.to_string()).collect();
    let mut block = Vec::new();
    for (id, name) in entries {
        if manual_ids.contains(id) || next >= MAX_ENTRIES {
            continue;
        }
        next += 1;
        let name: String = name.chars().filter(|c| !c.is_control()).collect();
        block.push(
            format!("[{next}]{eol}WeaponID     = {id}{eol}Name         = {name}{eol}")
                .replace(eol, "\n"),
        );
    }
    if !block.is_empty() {
        out.push(String::new());
        out.push(BEGIN.into());
        out.extend(
            block
                .iter()
                .flat_map(|b| b.trim_end().split('\n').map(String::from)),
        );
        out.push(END.into());
    }
    let mut s = out.join(eol);
    s.push_str(eol);
    s
}

/// Writes the trainer file if the result differs; a missing file means no trainer, so nothing to do.
pub fn sync_file(path: &Path, entries: &[Entry]) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    let bytes = fsutil::read(path)?;
    // a hand-edited CP-1252 file must not make every inject/remove fail after it already ran
    let Ok(text) = std::str::from_utf8(&bytes) else {
        tracing::warn!(
            "{} não está em UTF-8; lista do trainer não atualizada",
            path.display()
        );
        return Ok(());
    };
    let new = sync(text, entries);
    if new.as_bytes() != bytes {
        fsutil::backup_once(path)?;
        fsutil::write_atomic(path, new.as_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANUAL: &str = "; note\r\n[1]\r\nWeaponID     = 70\r\nName         = Régua\r\n[2]\r\nWeaponID     = 71\r\nName         = Faca\r\n\r\n; etc\r\n";

    #[test]
    fn adds_a_block_after_the_manual_entries_and_removes_it_again() {
        let one = sync(MANUAL, &[(72, "Luger".into()), (73, "Ak\n".into())]);
        assert!(one.starts_with(MANUAL.trim_end()), "{one}");
        assert!(one.contains("[3]\r\nWeaponID     = 72\r\nName         = Luger\r\n"));
        assert!(one.contains("[4]\r\nWeaponID     = 73\r\nName         = Ak\r\n"));
        // idempotent
        assert_eq!(sync(&one, &[(72, "Luger".into()), (73, "Ak".into())]), one);
        // dropping all entries gives the manual file back
        assert_eq!(sync(&one, &[]), MANUAL.trim_end().to_string() + "\r\n");
    }

    #[test]
    fn a_file_that_is_not_utf8_is_left_alone_without_an_error() {
        let d = tempfile::tempdir().unwrap();
        let ini = d.path().join("Add-on Weapons.ini");
        let cp1252 = b"[1]\r\nWeaponID     = 70\r\nName         = R\xE9gua\r\n";
        std::fs::write(&ini, cp1252).unwrap();
        sync_file(&ini, &[(72, "Luger".into())]).unwrap();
        assert_eq!(std::fs::read(&ini).unwrap(), cp1252);
    }

    #[test]
    fn a_manual_entry_with_the_same_type_wins() {
        let s = sync(MANUAL, &[(70, "Outra".into())]);
        assert!(!s.contains("AWR-BEGIN"), "{s}");
    }
}
