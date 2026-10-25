//! New CJ clothes in the game's shops. One `shopping.dat`, rebuilt from the game's own, lists
//! every enabled AWR clothing item; ModLoader merges it over the game's file.
//!
//! ModLoader merges this file line by line *by position*, so a section another mod also
//! changes can lose lines (and an item without a price crashes the shop). We only add lines at
//! the end of sections that no other mod touches ([`touched`]).

use crate::meta::Clothes;

/// Body parts a shop sells: torso, legs, shoes, necklace, watch, glasses, hat.
pub const BODY_PARTS: [u8; 7] = [0, 2, 3, 13, 14, 15, 16];
/// The shop scripts show at most 12 items of one body part (no original shop has more).
pub const MAX_PER_PART: usize = 12;
/// `CShopping` holds 560 price lines for the whole file (gta-reversed `NUM_ITEMS`).
pub const MAX_PRICES: usize = 560;
/// The price section the clothes live in.
pub const CLOTHES: &str = "Clothes";

/// A section inside `section prices` or `section shops`: `body` are its line indices, `end`
/// the index of its `end` line.
#[derive(Debug)]
struct Section<'a> {
    parent: &'a str,
    name: &'a str,
    body: std::ops::Range<usize>,
}

fn words(line: &str) -> Vec<&str> {
    let line = line.split('#').next().unwrap_or_default();
    line.split(|c: char| c.is_ascii_whitespace() || c == ',')
        .filter(|w| !w.is_empty())
        .collect()
}

fn sections<'a>(lines: &[&'a str]) -> Vec<Section<'a>> {
    let mut out = vec![];
    let mut stack: Vec<(&str, usize)> = vec![];
    for (i, l) in lines.iter().enumerate() {
        match words(l)[..] {
            ["section", name, ..] => stack.push((name, i + 1)),
            ["end", ..] => {
                if let Some((name, start)) = stack.pop() {
                    if let Some(&(parent, _)) = stack.last() {
                        out.push(Section {
                            parent,
                            name,
                            body: start..i,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn find<'s, 'a>(secs: &'s [Section<'a>], parent: &str, name: &str) -> Option<&'s Section<'a>> {
    secs.iter()
        .find(|s| s.parent.eq_ignore_ascii_case(parent) && s.name.eq_ignore_ascii_case(name))
}

/// `(texture, model, body part)` of every clothing price line.
pub fn originals(text: &str) -> Vec<(String, String, u8)> {
    let lines: Vec<&str> = text.lines().collect();
    let secs = sections(&lines);
    let Some(s) = find(&secs, "prices", CLOTHES) else {
        return vec![];
    };
    lines[s.body.clone()]
        .iter()
        .filter_map(|l| match words(l)[..] {
            [tex, _, model, part, ..] => {
                Some((tex.to_lowercase(), model.to_lowercase(), part.parse().ok()?))
            }
            _ => None,
        })
        .collect()
}

/// Section names of the shops that sell clothes (`type clothes`), in file order. `uniform` is
/// the missions' outfit list, not a shop.
pub fn shops(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    sections(&lines)
        .iter()
        .filter(|s| {
            s.parent.eq_ignore_ascii_case("shops") && !s.name.eq_ignore_ascii_case("uniform")
        })
        .filter(|s| {
            lines[s.body.clone()]
                .iter()
                .any(|l| words(l)[..] == ["type", "clothes"])
        })
        .map(|s| s.name.to_string())
        .collect()
}

/// How many items of `part` the shop lists.
pub fn count(text: &str, shop: &str, part: u8) -> usize {
    let parts: std::collections::HashMap<String, u8> = originals(text)
        .into_iter()
        .map(|(t, _, p)| (t, p))
        .collect();
    let lines: Vec<&str> = text.lines().collect();
    let secs = sections(&lines);
    let Some(s) = find(&secs, "shops", shop) else {
        return 0;
    };
    lines[s.body.clone()]
        .iter()
        .filter(|l| match words(l)[..] {
            ["item", tex, ..] => parts.get(&tex.to_lowercase()) == Some(&part),
            _ => false,
        })
        .count()
}

/// Price lines in the whole file (the game's 560 limit counts all of them).
pub fn prices(text: &str) -> usize {
    let lines: Vec<&str> = text.lines().collect();
    sections(&lines)
        .iter()
        .filter(|s| s.parent.eq_ignore_ascii_case("prices"))
        .map(|s| {
            lines[s.body.clone()]
                .iter()
                .filter(|l| !words(l).is_empty())
                .count()
        })
        .sum()
}

/// Sections whose lines differ between the game's file and another mod's copy.
pub fn touched(game: &str, other: &str) -> Vec<String> {
    let body = |text: &str| -> Vec<(String, Vec<String>)> {
        let lines: Vec<&str> = text.lines().collect();
        sections(&lines)
            .iter()
            .map(|s| {
                let key = format!("{}/{}", s.parent, s.name).to_lowercase();
                let rows = lines[s.body.clone()]
                    .iter()
                    .map(|l| words(l).join(" ").to_lowercase())
                    .filter(|l| !l.is_empty())
                    .collect();
                (key, rows)
            })
            .collect()
    };
    let (a, b) = (body(game), body(other));
    let mut out: Vec<String> = b
        .iter()
        .filter(|(k, rows)| a.iter().find(|(ka, _)| ka == k).map(|(_, r)| r) != Some(rows))
        .map(|(k, _)| k.rsplit('/').next().unwrap_or(k).to_string())
        .collect();
    out.dedup();
    out
}

fn price_line(c: &Clothes) -> String {
    format!(
        "\t\t{}\t\t{}\t\t{}\t\t{}\t\trespect 0 \tsexy 0\t\t{}",
        c.texture, c.gxt, c.model, c.body_part, c.price
    )
}

/// The game's `shopping.dat` with each item's price line at the end of the clothes prices
/// and an `item` line at the end of each shop that sells it. No items: the game's file as is.
pub fn build(game: &str, items: &[&Clothes]) -> String {
    if items.is_empty() {
        return game.to_string();
    }
    let eol = if game.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = game.lines().collect();
    let secs = sections(&lines);
    // (index of the section's `end` line, lines to put before it)
    let mut inserts: Vec<(usize, Vec<String>)> = vec![];
    if let Some(s) = find(&secs, "prices", CLOTHES) {
        inserts.push((s.body.end, items.iter().map(|c| price_line(c)).collect()));
    }
    for s in secs
        .iter()
        .filter(|s| s.parent.eq_ignore_ascii_case("shops"))
    {
        let new: Vec<String> = items
            .iter()
            .filter(|c| c.shops.iter().any(|n| n.eq_ignore_ascii_case(s.name)))
            .map(|c| format!("\t\titem {}", c.texture))
            .collect();
        if !new.is_empty() {
            inserts.push((s.body.end, new));
        }
    }
    let mut out: Vec<String> = Vec::with_capacity(lines.len() + 16);
    for (i, l) in lines.iter().enumerate() {
        for (_, new) in inserts.iter().filter(|(at, _)| *at == i) {
            out.extend(new.iter().cloned());
        }
        out.push(l.to_string());
    }
    let mut s = out.join(eol);
    s.push_str(eol);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAT: &str = "section prices\r\n\tsection CarMods\r\n\t\tbnt_b_sc_l\t\tBMBLBS\t\trespect 0 \tsexy 0\t\t150\r\n\tend\r\n\tsection Clothes\r\n\t\t# texture nametag model type\r\n\t\tvest\t\tVESTWH\t\tvest\t\t0\t\trespect 2 \tsexy 3\t\t10\r\n\t\tlegsblack\t\tLEGSBL\t\tlegs\t\t2\t\trespect 0 \tsexy 0\t\t10\r\n\tend\r\nend\r\nsection shops\r\n\tsection CSchp # BINCO\r\n\t\ttype clothes\r\n\t\titem vest\r\n\t\titem legsblack\r\n\tend\r\n\tsection carmod1\r\n\ttype CarMods\r\n\t\titem bnt_b_sc_l\r\n\tend\r\nend\r\n";

    fn shirt(shops: &[&str]) -> Clothes {
        Clothes {
            texture: "awrtee".into(),
            model: "vest".into(),
            body_part: 0,
            price: 7,
            shops: shops.iter().map(|s| s.to_string()).collect(),
            gxt: "AWRR001".into(),
        }
    }

    #[test]
    fn reads_clothes_shops_and_counts() {
        assert_eq!(
            originals(DAT),
            [
                ("vest".into(), "vest".into(), 0),
                ("legsblack".into(), "legs".into(), 2)
            ]
        );
        assert_eq!(shops(DAT), ["CSchp"]);
        assert_eq!((count(DAT, "CSchp", 0), count(DAT, "csCHP", 2)), (1, 1));
        assert_eq!(prices(DAT), 3);
    }

    #[test]
    fn adds_lines_at_the_end_of_sections_only() {
        let c = shirt(&["CSchp"]);
        let out = build(DAT, &[&c]);
        assert!(out.contains("sexy 0\t\t10\r\n\t\tawrtee\t\tAWRR001\t\tvest\t\t0\t\trespect 0 \tsexy 0\t\t7\r\n\tend"), "{out}");
        assert!(
            out.contains("\t\titem legsblack\r\n\t\titem awrtee\r\n\tend"),
            "{out}"
        );
        assert_eq!((count(&out, "CSchp", 0), prices(&out)), (2, 4));
        // every original line keeps its position, so ModLoader's merge only sees additions
        let orig: Vec<&str> = DAT.lines().collect();
        assert_eq!(
            out.lines()
                .filter(|l| !l.contains("awrtee"))
                .collect::<Vec<_>>(),
            orig
        );
        assert_eq!(build(DAT, &[]), DAT);
        assert_eq!(touched(DAT, &out), ["clothes", "cschp"]);
        assert!(touched(DAT, &DAT.replace("\r\n", "\n")).is_empty());
    }

    #[test]
    fn real_file_has_six_full_torso_racks_but_room_elsewhere() {
        if !crate::testutil::have_game_fixtures() {
            return;
        }
        let f = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/shopping.dat"
        ))
        .unwrap();
        let t: String = f.iter().map(|&b| char::from(b)).collect();
        assert_eq!(shops(&t).len(), 6);
        assert_eq!(prices(&t), 544);
        assert_eq!(count(&t, "CSchp", 0), MAX_PER_PART);
        assert_eq!(count(&t, "Csexl", 0), 7);
        assert_eq!(build(&t, &[]), t);
        let c = shirt(&["Csexl"]);
        assert_eq!(touched(&t, &build(&t, &[&c])), ["clothes", "csexl"]);
    }
}
