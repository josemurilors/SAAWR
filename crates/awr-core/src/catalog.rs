//! The clothes catalog: a CLEO script (`extras/cleo/awr-roupas.sc`, compiled with
//! `scripts/build-cleo.sh`) that sells and lets the player wear AWR clothes, hair and
//! accessories, fed by an ini the app writes. It exists in the game only while at least one
//! clothing item is installed and on.
//!
//! The script lives in a ModLoader folder (`modloader/awr-lojas/cleo`) with its menu texts; the
//! ini goes to the game's own `CLEO\AWR-Roupas`, where the script reads it (CLEO's ini opcodes
//! see the real game folder, not ModLoader's). The player's purchases are in `estado.ini`
//! there; the app never rewrites that file.

use crate::meta::Clothes;
use crate::stores::{HAIR, SHOPS};

pub const SCRIPT: &[u8] = include_bytes!("../../../extras/cleo/awr-roupas.cs");
pub const UI_FXT: &[u8] = include_bytes!("../../../extras/cleo/awr-roupas.fxt");
pub const SCRIPT_FILE: &str = "awr-roupas.cs";
pub const UI_FXT_FILE: &str = "awr-roupas.fxt";
/// ModLoader folder with the script and its texts.
pub const CATALOG_DIR: &str = "awr-lojas";
/// Folder of the ini files, relative to the game folder.
pub const DATA_DIR: &str = "CLEO/AWR-Roupas";
pub const CONFIG_FILE: &str = "awr-roupas.ini";
pub const STATE_FILE: &str = "estado.ini";

/// Torso first, hair last, as the shops list them.
fn order(part: u8) -> u8 {
    if part == HAIR {
        255
    } else {
        part
    }
}

/// The ini the script reads. Item numbers follow the order of the lists; `List0..8` are the
/// shops of [`SHOPS`], `List9` the items no shop sells (free in the wardrobe) and `List10` all of
/// them.
pub fn config_ini(items: &[&Clothes]) -> String {
    let mut v = items.to_vec();
    v.sort_by(|a, b| (order(a.body_part), &a.texture).cmp(&(order(b.body_part), &b.texture)));
    let mut l: Vec<String> = vec![];
    l.push("; escrito pelo SA-AWR: nao edite (o estado.ini ao lado e do jogador)".into());
    l.push("[Config]".into());
    l.push(format!("Count={}", v.len()));
    for (n, s) in SHOPS.iter().enumerate() {
        l.push(format!("[Shop{n}]"));
        l.push(format!("Area={}", s.area));
        l.push(format!("X={}", s.x));
        l.push(format!("Y={}", s.y));
    }
    l.push("[Index]".into());
    for (n, c) in v.iter().enumerate() {
        l.push(format!("{}={n}", c.texture));
    }
    for (n, c) in v.iter().enumerate() {
        l.push(format!("[Item{n}]"));
        l.push(format!("Tex={}", c.texture));
        l.push(format!("Model={}", c.model));
        l.push(format!("Part={}", c.body_part));
        l.push(format!("Price={}", c.price));
        l.push(format!("Gxt={}", c.gxt));
    }
    let list = |l: &mut Vec<String>, name: String, idx: Vec<usize>| {
        l.push(format!("[{name}]"));
        l.push(format!("Count={}", idx.len()));
        for (k, i) in idx.iter().enumerate() {
            l.push(format!("I{k}={i}"));
        }
    };
    for (n, s) in SHOPS.iter().enumerate() {
        let idx = (0..v.len())
            .filter(|&i| v[i].shops.iter().any(|x| x.eq_ignore_ascii_case(s.section)))
            .collect();
        list(&mut l, format!("List{n}"), idx);
    }
    list(
        &mut l,
        "List9".into(),
        (0..v.len()).filter(|&i| v[i].shops.is_empty()).collect(),
    );
    list(&mut l, "List10".into(), (0..v.len()).collect());
    l.push(String::new());
    l.join("\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(tex: &str, part: u8, price: u32, shops: &[&str]) -> Clothes {
        Clothes {
            texture: tex.into(),
            model: "tshirt".into(),
            body_part: part,
            price,
            shops: shops.iter().map(|s| s.to_string()).collect(),
            gxt: format!("AWRR{:03}", tex.len()),
        }
    }

    /// Reads `key` of `section` the way the game's ini opcodes do.
    fn get<'a>(ini: &'a str, section: &str, key: &str) -> Option<&'a str> {
        let mut cur = "";
        for line in ini.lines() {
            if let Some(s) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                cur = s;
            } else if cur == section {
                if let Some((k, v)) = line.split_once('=') {
                    if k.eq_ignore_ascii_case(key) {
                        return Some(v);
                    }
                }
            }
        }
        None
    }

    #[test]
    fn the_script_and_its_texts_are_embedded() {
        assert_eq!(
            &SCRIPT[..3],
            [0xA4, 0x03, 0x09],
            "starts with 03A4 (script name)"
        );
        assert!(SCRIPT.windows(7).any(|w| w == b"AWRROUP"));
        let fxt = String::from_utf8(UI_FXT.to_vec()).unwrap();
        for key in [
            "AWRTIT", "AWRWRD", "AWRHLP", "AWRSHP", "AWRNXT", "AWRPRC", "AWRS0", "AWRS8",
        ] {
            assert!(fxt.contains(&format!("{key} ")), "{key}");
        }
        // GXT keys are 7 letters at most
        assert!(fxt
            .lines()
            .all(|l| l.split(' ').next().is_some_and(|k| k.len() <= 7)));
    }

    #[test]
    fn the_ini_lists_items_by_shop_and_keeps_free_ones_apart() {
        let a = item("zhat", 16, 100, &["CSchp"]);
        let b = item("shirt", 0, 150, &["Csexl", "CSchp"]);
        let c = item("cut", HAIR, 300, &["barber2"]);
        let d = item("free", 2, 0, &[]);
        let ini = config_ini(&[&a, &b, &c, &d]);
        // order: torso, legs, hat, hair
        assert_eq!(get(&ini, "Index", "shirt"), Some("0"));
        assert_eq!(get(&ini, "Index", "free"), Some("1"));
        assert_eq!(get(&ini, "Index", "zhat"), Some("2"));
        assert_eq!(get(&ini, "Index", "cut"), Some("3"));
        assert_eq!(get(&ini, "Item0", "Part"), Some("0"));
        assert_eq!(get(&ini, "Item0", "Price"), Some("150"));
        assert_eq!(get(&ini, "Item0", "Gxt"), Some("AWRR005"));
        // Binco (0): shirt and hat, in item order; Didier Sachs (5): the shirt
        assert_eq!(get(&ini, "List0", "Count"), Some("2"));
        assert_eq!(
            (get(&ini, "List0", "I0"), get(&ini, "List0", "I1")),
            (Some("0"), Some("2"))
        );
        assert_eq!(
            (get(&ini, "List5", "Count"), get(&ini, "List5", "I0")),
            (Some("1"), Some("0"))
        );
        assert_eq!(
            (get(&ini, "List7", "Count"), get(&ini, "List7", "I0")),
            (Some("1"), Some("3"))
        );
        assert_eq!(get(&ini, "List1", "Count"), Some("0"));
        assert_eq!(
            (get(&ini, "List9", "Count"), get(&ini, "List9", "I0")),
            (Some("1"), Some("1"))
        );
        assert_eq!(get(&ini, "List10", "Count"), Some("4"));
        assert_eq!(get(&ini, "Shop0", "Area"), Some("15"));
        assert_eq!(get(&ini, "Shop8", "Area"), Some("12"));
        assert!(ini.is_ascii() && ini.contains("\r\n") && !ini.replace("\r\n", "").contains('\n'));
    }
}
