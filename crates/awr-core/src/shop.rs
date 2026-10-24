//! Custom Ammu-Nation Mod (cjfan) integration: lists the enabled AWR weapons in its
//! `data/ammun.dat`. Our lines live between two marker comments; everything else in the file
//! is left alone. Each weapon's name comes from a GXT key (`AWR{type}`) that the weapon's own
//! `.fxt` defines.

use crate::{error::Result, fsutil, meta::Shop, text::tokens};
use std::path::Path;

const BEGIN: &str = "# AWR-BEGIN (managed by SA-AWR, do not edit)";
const END: &str = "# AWR-END";
/// The mod's own limit per category.
const MAX_PER_CATEGORY: usize = 24;
/// Flags column: available in every store.
const FLAGS: &str = "111111";

/// GXT key of a weapon type (keys are 7 characters at most: `AWR` + up to 4 digits).
pub fn gxt_key(type_id: u32) -> String {
    format!("AWR{type_id}")
}

/// Sensible shop settings for a clone of `reference` (a `weapon.dat` name), from the mod's
/// own default `ammun.dat`.
pub fn defaults(reference: &str) -> Shop {
    // (category, anim, price, ammo)
    let (category, anim, price, ammo) = match reference.to_ascii_uppercase().as_str() {
        "PISTOL" | "PISTOL_SILENCED" => (0, 0, 300, 30),
        "DESERT_EAGLE" => (0, 0, 1200, 15),
        "MICRO_UZI" | "TEC9" => (1, 0, 400, 60),
        "SHOTGUN" | "SAWNOFF" | "SPAS12" => (2, 1, 700, 15),
        "GRENADE" | "TEARGAS" | "MOLOTOV" => (3, 0, 300, 5),
        "MP5" => (5, 0, 2000, 90),
        "COUNTRYRIFLE" | "SNIPERRIFLE" => (6, 1, 3000, 15),
        "AK47" | "M4" => (7, 1, 4000, 120),
        "RLAUNCHER" | "RLAUNCHER_HS" | "FLAME" | "MINIGUN" | "SATCHEL" => (8, 1, 6000, 5),
        "UNARMED" | "BRASSKNUCKLE" | "GOLFCLUB" | "NIGHTSTICK" | "KNIFE" | "BASEBALLBAT"
        | "SHOVEL" | "POOLCUE" | "KATANA" | "CHAINSAW" | "DILDO1" | "DILDO2" | "VIBE1"
        | "VIBE2" | "FLOWERS" | "CANE" => (10, 0, 200, 1),
        _ => (9, 0, 1000, 30),
    };
    Shop {
        category,
        unlock: 0,
        anim,
        price,
        ammo,
    }
}

/// `(weapon type, settings)` of one enabled weapon that is for sale.
pub type Entry = (u32, Shop);

fn tok(line: &[u8], i: usize) -> Option<String> {
    let t = tokens(line);
    t.get(i)
        .map(|&(s, e)| String::from_utf8_lossy(&line[s..e]).into_owned())
}

/// Rewrites `text` so our block lists exactly `entries` (those that fit the mod's limits).
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
    let data: Vec<&[u8]> = kept
        .iter()
        .map(|l| l.as_bytes())
        .filter(|l| !matches!(l.trim_ascii().first(), None | Some(b'#')))
        .collect();
    let manual_types: Vec<u32> = data
        .iter()
        .filter_map(|l| tok(l, 0)?.parse().ok())
        .collect();
    let mut per_category = [0usize; 256];
    for l in &data {
        if let Some(c) = tok(l, 3).and_then(|c| c.parse::<usize>().ok()) {
            per_category[c.min(255)] += 1;
        }
    }
    let mut block = Vec::new();
    for (type_id, s) in entries {
        let cat = usize::from(s.category);
        if manual_types.contains(type_id) || per_category[cat] >= MAX_PER_CATEGORY {
            tracing::warn!(
                "Ammu-Nation: arma {type_id} fora da lista (tipo repetido ou categoria cheia)"
            );
            continue;
        }
        per_category[cat] += 1;
        block.push(format!(
            "{type_id}\t{}\t{}\t{}\t{}\t{}\t{}\t{FLAGS}",
            gxt_key(*type_id),
            s.anim,
            s.category,
            s.unlock,
            s.price,
            s.ammo
        ));
    }
    let mut out: Vec<String> = kept.iter().map(|l| l.to_string()).collect();
    if !block.is_empty() {
        out.push(String::new());
        out.push(BEGIN.into());
        out.extend(block);
        out.push(END.into());
    }
    let mut s = out.join(eol);
    s.push_str(eol);
    s
}

/// Writes the file if the result differs; a missing file means no shop mod, so nothing to do.
pub fn sync_file(path: &Path, entries: &[Entry]) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    let bytes = fsutil::read(path)?;
    // the mod's own file is CP-1252 / ASCII; our block is ASCII, so bytes outside it stay as read
    let text: String = bytes.iter().map(|&b| char::from(b)).collect();
    let new = sync(&text, entries);
    let new_bytes: Vec<u8> = new.chars().map(|c| c as u8).collect();
    if new_bytes != bytes {
        fsutil::backup_once(path)?;
        fsutil::write_atomic(path, &new_bytes)?;
    }
    Ok(())
}

/// How many of `lines` are data lines (used by tests and the CLI report).
#[cfg(test)]
fn data_lines(text: &str) -> usize {
    crate::text::lines(text.as_bytes())
        .filter(|l| !matches!(l.trim_ascii().first(), None | Some(b'#')))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAT: &str = "# header\r\n#id\tname\r\n22\tPISTOL\t0\t0\t0\t200\t30\t111111\r\n24\tDEAGLE\t0\t0\t9\t1200\t15\t111111";

    fn shop(category: u8) -> Shop {
        Shop {
            category,
            unlock: 2,
            anim: 1,
            price: 900,
            ammo: 20,
        }
    }

    #[test]
    fn adds_a_block_and_takes_it_out_again() {
        let one = sync(DAT, &[(70, shop(7))]);
        assert!(one.starts_with(DAT), "{one}");
        assert!(
            one.contains("\r\n70\tAWR70\t1\t7\t2\t900\t20\t111111\r\n"),
            "{one}"
        );
        assert_eq!(data_lines(&one), 3);
        assert_eq!(sync(&one, &[(70, shop(7))]), one, "idempotent");
        assert_eq!(data_lines(&sync(&one, &[])), 2);
        assert!(!sync(&one, &[]).contains("AWR"));
    }

    #[test]
    fn a_manual_line_with_the_same_type_wins_and_categories_are_capped() {
        assert!(!sync(DAT, &[(22, shop(0))]).contains("AWR"));
        let many: Vec<Entry> = (70..100).map(|t| (t, shop(8))).collect();
        assert_eq!(data_lines(&sync(DAT, &many)), 2 + 24);
    }

    #[test]
    fn defaults_follow_the_weapon_family() {
        assert_eq!(defaults("ak47").category, 7);
        assert_eq!(defaults("KNIFE").category, 10);
        assert_eq!(defaults("JETPACK").category, 9);
        assert_eq!(gxt_key(4095).len(), 7);
    }
}
