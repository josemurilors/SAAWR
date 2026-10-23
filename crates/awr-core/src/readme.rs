//! Handling lines that a vehicle pack ships in its readme. Authors tune them for the
//! model, so the injector prefers them over a copy of the reference's line.

use crate::text::{lines, tokens};
use std::{fs, path::Path, path::PathBuf};

/// A standard handling line (and the `!`/`%`/`$` line of bikes, boats and aircraft).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundHandling {
    pub standard: Vec<u8>,
    pub special: Option<Vec<u8>>,
    pub source: PathBuf,
}

/// The weapon lines a pack ships: `weapon.dat` (one per skill level), the `weap` IDE line and
/// the `gtasa_weapon_config.dat` line. Authors write `ID` where the model/type ID goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundWeapon {
    pub name: String,
    pub dat_lines: Vec<Vec<u8>>,
    pub ide_line: Option<Vec<u8>>,
    pub config_line: Option<Vec<u8>>,
    pub source: PathBuf,
}

/// Tokens of a standard `handling.cfg` line: name + 35 values.
const STANDARD_TOKENS: usize = 36;

fn is_float(t: &[u8]) -> bool {
    std::str::from_utf8(t).is_ok_and(|s| s.parse::<f64>().is_ok())
}

fn is_hex(t: &[u8]) -> bool {
    !t.is_empty() && t.iter().all(u8::is_ascii_hexdigit)
}

fn is_name(t: &[u8]) -> bool {
    (1..=20).contains(&t.len())
        && t[0].is_ascii_alphabetic()
        && t.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_')
}

/// `NAME mass turn drag ... drive engine ... modelflags handlingflags ... anim group`
fn standard_name(line: &[u8]) -> Option<&[u8]> {
    let t: Vec<&[u8]> = tokens(line).iter().map(|&(s, e)| &line[s..e]).collect();
    let ok = t.len() == STANDARD_TOKENS
        && is_name(t[0])
        && (1..=14).all(|i| is_float(t[i]))
        && matches!(t[15], b"F" | b"R" | b"4")
        && matches!(t[16], b"P" | b"D" | b"E")
        && (17..=30).all(|i| is_float(t[i]))
        && is_hex(t[31])
        && is_hex(t[32])
        && (33..=35).all(|i| t[i].iter().all(u8::is_ascii_digit));
    ok.then(|| t[0])
}

/// `! NAME values...` (`%` boats, `$` aircraft): returns the prefix and the name.
fn special_name(line: &[u8]) -> Option<(u8, &[u8])> {
    let t: Vec<&[u8]> = tokens(line).iter().map(|&(s, e)| &line[s..e]).collect();
    let prefix = match t.first()? {
        [p @ (b'!' | b'%' | b'$')] => *p,
        _ => return None,
    };
    let ok = t.len() >= 10 && is_name(t[1]) && t[2..].iter().all(|v| is_float(v));
    ok.then(|| (prefix, t[1]))
}

fn upper_word(t: &[u8]) -> bool {
    !t.is_empty()
        && t.iter()
            .all(|b| b.is_ascii_uppercase() || *b == b'_' || b.is_ascii_digit())
}

fn id_or_number(t: &[u8]) -> bool {
    t.eq_ignore_ascii_case(b"ID") || (!t.is_empty() && t.iter().all(u8::is_ascii_digit))
}

/// `$ NAME FIRE_TYPE range range model ...` (or `\xA3` for melee): returns the weapon name.
fn weapon_dat_name(line: &[u8]) -> Option<&[u8]> {
    let t: Vec<&[u8]> = tokens(line).iter().map(|&(s, e)| &line[s..e]).collect();
    let ok =
        t.len() >= 20 && matches!(t[0], b"$" | b"\xA3") && upper_word(t[1]) && upper_word(t[2]);
    ok.then(|| t[1])
}

/// `ID, model, txd, ifp, n, n, n`
fn weapon_ide(line: &[u8]) -> bool {
    let t: Vec<&[u8]> = tokens(line).iter().map(|&(s, e)| &line[s..e]).collect();
    t.len() == 7
        && id_or_number(t[0])
        && t[1..4].iter().all(|x| is_name(x))
        && t[4..].iter().all(|x| x.iter().all(u8::is_ascii_digit))
}

/// `type NAME stat second ammoUsed ammoStreet r g b float [parent]`
fn weapon_config(line: &[u8], name: &[u8]) -> bool {
    let t: Vec<&[u8]> = tokens(line).iter().map(|&(s, e)| &line[s..e]).collect();
    t.len() >= 10
        && (id_or_number(t[0]) || t[0] == b"-1")
        && t[1].eq_ignore_ascii_case(name)
        && is_float(t[9])
}

/// Readmes of a pack: `.txt` files at the top and one level down, in name order.
fn readmes(dir: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![dir.to_path_buf()];
    let skip = ["screenshots", "paintjobs", "tuning parts"];
    dirs.extend(
        fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .filter(|p| {
                let n = p.file_name().map(|n| n.to_string_lossy().to_lowercase());
                n.is_some_and(|n| !skip.contains(&n.as_str()))
            }),
    );
    let mut out: Vec<PathBuf> = dirs
        .iter()
        .flat_map(|d| fs::read_dir(d).into_iter().flatten().flatten())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("txt")))
        .collect();
    out.sort();
    out
}

/// The handling the pack ships, from the first readme that has one. With several
/// vehicles listed, the line named `prefer` (the model name) wins, then the first.
pub fn find_handling(dir: &Path, prefer: &str) -> Option<FoundHandling> {
    for path in readmes(dir) {
        let Ok(bytes) = fs::read(&path) else { continue };
        let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
        let all: Vec<&[u8]> = lines(bytes).collect();
        let standards: Vec<&[u8]> = all
            .iter()
            .copied()
            .filter(|l| standard_name(l).is_some())
            .collect();
        let chosen = standards
            .iter()
            .find(|l| standard_name(l).is_some_and(|n| n.eq_ignore_ascii_case(prefer.as_bytes())))
            .or(standards.first());
        let Some(chosen) = chosen else { continue };
        let name = standard_name(chosen).unwrap_or_default();
        let special = all
            .iter()
            .find(|l| special_name(l).is_some_and(|(_, n)| n.eq_ignore_ascii_case(name)));
        return Some(FoundHandling {
            standard: chosen.trim_ascii().to_vec(),
            special: special.map(|l| l.trim_ascii().to_vec()),
            source: path,
        });
    }
    None
}

/// The weapon the pack describes, from the first readme with `weapon.dat` lines.
pub fn find_weapon(dir: &Path) -> Option<FoundWeapon> {
    for path in readmes(dir) {
        let Ok(bytes) = fs::read(&path) else { continue };
        let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
        let all: Vec<&[u8]> = lines(bytes).collect();
        let Some(name) = all.iter().find_map(|l| weapon_dat_name(l)) else {
            continue;
        };
        let dat_lines: Vec<Vec<u8>> = all
            .iter()
            .filter(|l| weapon_dat_name(l) == Some(name))
            .map(|l| l.trim_ascii().to_vec())
            .collect();
        let pick =
            |f: &dyn Fn(&[u8]) -> bool| all.iter().find(|l| f(l)).map(|l| l.trim_ascii().to_vec());
        return Some(FoundWeapon {
            name: String::from_utf8_lossy(name).into_owned(),
            dat_lines,
            ide_line: pick(&weapon_ide),
            config_line: pick(&|l| weapon_config(l, name)),
            source: path,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIRT: &str = "DIRTBIKE 500.0 195.0 5.0 0.0 0.05 -0.09 103 1.6 0.9 0.48 5 190.0 50.0 5.0 R P 14.0 0.50 0 35.0 0.85 0.15 0.0 0.15 -0.16 0.5 0.0 0.0 0.15 10000 01000000 00000000 1 1 7";
    const URANUS: &str = "URANUS       1315.0    3000.0   2.5    0.0 0.1 -0.3  75  0.80 0.85 0.47  \t5 185.0 18.0 7.0  F P \t8.0  0.45 0 30.0  \t1.3  0.15  0.0   0.28 -0.10 0.5  0.3\t\t0.25 0.60 35000 \tc0000800\t4000001\t\t1  1\t0";
    const SPECIAL: &str = "!\tDIRTBIKE\t0.40\t0.40\t0.30\t0.08\t48.0\t43.0\t0.92\t0.60\t1.0\t0.1\t45.0\t-35.0\t-0.010\t0.5\t0.5";

    fn pack(files: &[(&str, String)]) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        for (name, body) in files {
            let p = d.path().join(name);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, body).unwrap();
        }
        d
    }

    #[test]
    fn recognizes_real_world_lines_and_rejects_prose() {
        assert_eq!(standard_name(DIRT.as_bytes()), Some(&b"DIRTBIKE"[..]));
        assert_eq!(standard_name(URANUS.as_bytes()), Some(&b"URANUS"[..]));
        assert_eq!(special_name(SPECIAL.as_bytes()).map(|s| s.0), Some(b'!'));
        for no in [
            "Handling",
            "Created by Yochi 1999 Volkswagen New Beetle 2026",
            "URANUS 1315.0 3000.0",
        ] {
            assert!(standard_name(no.as_bytes()).is_none(), "{no}");
        }
    }

    #[test]
    fn finds_the_weapon_lines_of_a_pack_readme() {
        let text = "weapon lines :\n\n$ LUGER   INSTANT_HIT   25.0 30.0   ID   -1   2   silenced   8   40   0.24  0.05  0.11   0  0\t\t1.1  1.0 \t 6 18  6\t 6 18  6  40\t7001\n$ LUGER   INSTANT_HIT   30.0 35.0   ID   -1   2   silenced   8   40   0.24  0.05  0.11   1  500\t1.2  1.0 \t 6 17  6\t 6 17  6  40\t7011\n\nweapon.ide lines :\n\nID, luger, luger, silenced, 1, 30, 0\n\ngtasa_weapon_config.dat\n\nID    LUGER\t\t\t\t69         1                          0                       24                      149  194   24   1.0\tPISTOL\n";
        let d = pack(&[("(ReadMe).txt", text.into())]);
        let w = find_weapon(d.path()).unwrap();
        assert_eq!((w.name.as_str(), w.dat_lines.len()), ("LUGER", 2));
        assert_eq!(
            w.ide_line.as_deref(),
            Some(&b"ID, luger, luger, silenced, 1, 30, 0"[..])
        );
        assert!(w.config_line.unwrap().starts_with(b"ID    LUGER"));
        // a vehicle readme has no weapon lines
        assert!(find_weapon(pack(&[("a.txt", DIRT.into())]).path()).is_none());
    }

    #[test]
    fn picks_the_line_and_its_special_pair() {
        let text = format!("\u{feff}Handling\r\n\r\n{DIRT}\r\n{SPECIAL}\r\n");
        let d = pack(&[("Leia.txt", text)]);
        let f = find_handling(d.path(), "sanchez").unwrap();
        assert_eq!(f.standard, DIRT.as_bytes());
        assert_eq!(f.special.as_deref(), Some(SPECIAL.as_bytes()));
        assert!(f.source.ends_with("Leia.txt"));
    }

    #[test]
    fn prefers_the_named_vehicle_and_looks_one_folder_down() {
        let both = format!("{DIRT}\n{URANUS}\n");
        let d = pack(&[
            ("docs/readme.txt", both),
            ("Screenshots/x.txt", DIRT.into()),
        ]);
        let f = find_handling(d.path(), "uranus").unwrap();
        assert_eq!(f.standard, URANUS.trim().as_bytes());
        assert!(f.special.is_none());
        assert!(find_handling(pack(&[("a.txt", "nada".into())]).path(), "x").is_none());
    }
}
