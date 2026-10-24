//! Keeps RZL-Trainer's `Add-on Weapons.ini` in step with the enabled AWR weapons.
//! Our entries live between two marker lines; everything else in the file is left alone.

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
    let (mut kept, had_block) = crate::text::outside_block(text, BEGIN, END);
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
        // ASCII like every name the game shows, so the file's own encoding does not matter
        let name = crate::ide::hud_name(name);
        block.push(
            format!("[{next}]{eol}WeaponID     = {id}{eol}Name         = {name}{eol}")
                .replace(eol, "\n"),
        );
    }
    if block.is_empty() && !had_block {
        return text.to_string();
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
    fn a_cp1252_file_keeps_its_bytes_and_gets_the_block() {
        let d = tempfile::tempdir().unwrap();
        let ini = d.path().join("Add-on Weapons.ini");
        let cp1252 = b"[1]\r\nWeaponID     = 70\r\nName         = R\xE9gua\r\n";
        std::fs::write(&ini, cp1252).unwrap();
        let entries = [(72, "Régua Nova".into())];
        crate::fsutil::sync_mod_file(&ini, |t| sync(t, &entries)).unwrap();
        let out = std::fs::read(&ini).unwrap();
        assert!(out.starts_with(cp1252));
        assert!(out.ends_with(b"WeaponID     = 72\r\nName         = Regua Nova\r\n; AWR-END\r\n"));
    }

    #[test]
    fn a_manual_entry_with_the_same_type_wins() {
        let s = sync(MANUAL, &[(70, "Outra".into())]);
        assert!(!s.contains("AWR-BEGIN"), "{s}");
    }
}
