//! "Create Weapon Pickups" (Um Geek) integration: weapon pickups placed in the world by a CLEO
//! script that reads `pick-ups.dat`. Our lines live between two marker comments inside the
//! file's `{ ... }` block; everything else is left alone.
//!
//! Each line is `type ammo x y z` (type = the weapon type, not the model ID).

const BEGIN: &str = "# AWR-BEGIN (managed by SA-AWR, do not edit)";
const END: &str = "# AWR-END";

/// Normalizes one pickup typed as `x y z [ammo]` into `x y z ammo` (what `meta.json` keeps and the
/// editor shows); `sync` turns it into the mod's `type ammo x y z`.
/// `default_ammo` is used when the ammo is left out.
pub fn normalize(text: &str, default_ammo: u32) -> std::result::Result<String, String> {
    let t: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || c == ';')
        .filter(|s| !s.is_empty())
        .collect();
    if t.len() != 3 && t.len() != 4 {
        return Err(format!("pickup '{text}': use 'x y z' ou 'x y z munição'"));
    }
    let mut c = [0f64; 3];
    for (i, v) in t[..3].iter().enumerate() {
        c[i] = v
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite() && n.abs() < 100_000.0)
            .ok_or_else(|| format!("pickup '{text}': '{v}' não é uma coordenada"))?;
    }
    let ammo = match t.get(3) {
        Some(a) => a
            .parse::<u32>()
            .map_err(|_| format!("pickup '{text}': a munição precisa ser um número inteiro"))?,
        None => default_ammo,
    };
    Ok(format!("{} {} {} {ammo}", c[0], c[1], c[2]))
}

/// `(weapon type, normalized pickups)` of one enabled weapon.
pub type Entry = (u32, Vec<String>);

/// Rewrites `text` so our block lists exactly `entries`. A file without a closing `}` line is
/// not the mod's format: it is returned as it is.
pub fn sync(text: &str, entries: &[Entry]) -> String {
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let (kept, had_block) = crate::text::outside_block(text, BEGIN, END);
    let Some(close) = kept.iter().rposition(|l| l.trim() == "}") else {
        return text.to_string();
    };
    let mut block = Vec::new();
    for (type_id, list) in entries {
        for p in list {
            let t: Vec<&str> = p.split_whitespace().collect();
            if let [x, y, z, ammo] = t[..] {
                block.push(format!("{type_id}\t{ammo}\t{x} {y} {z}"));
            }
        }
    }
    if block.is_empty() && !had_block {
        return text.to_string();
    }
    let mut out: Vec<String> = kept[..close].iter().map(|l| l.to_string()).collect();
    if !block.is_empty() {
        out.push(BEGIN.into());
        out.extend(block);
        out.push(END.into());
    }
    out.extend(kept[close..].iter().map(|l| l.to_string()));
    let mut s = out.join(eol);
    s.push_str(eol);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAT: &str = "# Autor: Um Geek\r\n{\r\n17\t20\t2492.0297 -1678.5623 13.3363\r\n43\t15\t2488.7011 -1646.6979 14.0703\r\n}";

    #[test]
    fn normalizes_coordinates_and_ammo() {
        assert_eq!(normalize("10.5 20 3", 30).unwrap(), "10.5 20 3 30");
        assert_eq!(normalize("1,5 2 3 12", 30).unwrap(), "1.5 2 3 12");
        assert!(normalize("1 2", 30).is_err());
        assert!(normalize("a b c", 30).is_err());
        assert!(normalize("1 2 3 -5", 30).is_err());
    }

    #[test]
    fn the_block_sits_inside_the_braces_and_goes_away() {
        let one = sync(
            DAT,
            &[
                (70, vec!["1 2 3 30".into()]),
                (71, vec!["4 5 6 5".into(), "7 8 9 5".into()]),
            ],
        );
        assert!(one.contains("\r\n70\t30\t1 2 3\r\n"), "{one}");
        assert!(
            one.find("AWR-END").unwrap() < one.rfind('}').unwrap(),
            "{one}"
        );
        assert!(one.starts_with("# Autor: Um Geek\r\n{\r\n17\t20"));
        assert_eq!(
            sync(
                &one,
                &[
                    (70, vec!["1 2 3 30".into()]),
                    (71, vec!["4 5 6 5".into(), "7 8 9 5".into()])
                ]
            ),
            one
        );
        assert_eq!(sync(&one, &[]), format!("{DAT}\r\n"));
    }

    #[test]
    fn a_file_without_braces_is_left_alone() {
        assert_eq!(
            sync("nothing here", &[(70, vec!["1 2 3 4".into()])]),
            "nothing here"
        );
    }
}
