//! fastman92 Limit Adjuster `.ini` sync (spec §4.2).
//!
//! Works on raw bytes: only the numeric token of a managed key changes, plus its
//! leading `#` (the tutorials say to remove it before editing). Everything else,
//! including CP-1252 comments, CRLF and the missing final newline, stays byte-exact.
//! `Vehicle Models` / `Weapon Models` are never in the managed list: OLA owns them.

/// Keys OLA manages; they must stay `#`-disabled.
pub const OLA_KEYS: [(&str, &str); 2] = [
    ("IDE LIMITS", "Vehicle Models"),
    ("IDE LIMITS", "Weapon Models"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Want {
    pub section: &'static str,
    pub key: &'static str,
    /// Final value is `max(current, target)`: limits only grow.
    pub target: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub key: &'static str,
    pub from: Option<u64>,
    pub to: u64,
    pub was_disabled: bool,
    pub inserted: bool,
}

/// What the installed AWR mods need from the FLA.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Needs {
    pub max_model_id: Option<u32>,
    pub cars: u32,
    pub bikes: u32,
    pub boats: u32,
    pub flying: u32,
    pub weapons: u32,
    /// Weapon pickups placed by the Create Weapon Pickups mod.
    pub pickups: u32,
    pub max_weapon_type: Option<u32>,
    pub max_cargrp_group_len: u32,
}

const HEADROOM: u64 = 16;

fn round50(n: u64) -> u64 {
    n.div_ceil(50) * 50
}

/// count limit = original + AWR mods + headroom, never below the tutorial floor.
fn count(original: u64, added: u32, floor: u64) -> u64 {
    round50(original + u64::from(added) + HEADROOM).max(floor)
}

/// Turns the inventory's needs into managed keys and target values.
pub fn plan(n: &Needs) -> Vec<Want> {
    let w = |section, key, target| Want {
        section,
        key,
        target,
    };
    let vehicles = n.cars + n.bikes + n.boats + n.flying;
    let mut out = vec![];
    if let Some(max) = n.max_model_id {
        out.push(w(
            "ID LIMITS",
            "Count of killable model IDs",
            round50(u64::from(max) + HEADROOM),
        ));
    }
    if vehicles > 0 {
        out.extend([
            w("HANDLING.CFG LIMITS", "Apply handling.cfg patch", 1),
            // floors from the MixMods car tutorial: standard >= 300, bike ~20-30
            w(
                "HANDLING.CFG LIMITS",
                "Number of standard lines",
                count(210, vehicles, 300),
            ),
            w(
                "HANDLING.CFG LIMITS",
                "Number of bike lines",
                count(13, n.bikes, 20),
            ),
            w(
                "HANDLING.CFG LIMITS",
                "Number of flying lines",
                count(24, n.flying, 0),
            ),
            w(
                "HANDLING.CFG LIMITS",
                "Number of boat lines",
                count(12, n.boats, 0),
            ),
            w("ADDONS", "Enable vehicle audio loader", 1),
            w("SPECIAL", "Make paintjobs work for any ID", 1),
            // not managed: ID limit + special feature loader crashed with OLA in the spike
        ]);
    }
    if n.max_cargrp_group_len > 23 {
        out.push(w(
            "CAR STREAMING",
            "Cargrp cars per group",
            u64::from(n.max_cargrp_group_len) + 4,
        ));
    }
    if n.pickups > 0 {
        out.extend([
            w("IPL", "Enable pickup limit patch", 1),
            w("IPL", "Pickups", count(620, n.pickups, 0)),
        ]);
    }
    if n.weapons > 0 {
        let types = n.max_weapon_type.map_or(70, |t| u64::from(t) + 1);
        out.extend([
            w("WEAPON LIMITS", "Enable weapon type loader", 1),
            w(
                "WEAPON LIMITS",
                "Weapon type loader, number of type IDs",
                round50(types + HEADROOM).max(80),
            ),
            w(
                "OTHER LIMITS",
                "Object info entries",
                count(160, n.weapons, 0),
            ),
        ]);
    }
    out
}

/// One `key = value` line, possibly `#`-disabled.
struct Entry {
    line: usize,
    disabled: bool,
    /// Byte span of the numeric token inside the line.
    value: (usize, usize),
}

struct Parsed<'a> {
    lines: Vec<&'a [u8]>,
    /// Section name of each line (None before the first header).
    section_of: Vec<Option<String>>,
    header_line: Vec<(String, usize)>,
}

fn parse(ini: &[u8]) -> Parsed<'_> {
    // split keeping terminators so concatenation is the original file
    let lines: Vec<&[u8]> = ini.split_inclusive(|&b| b == b'\n').collect();
    let mut section = None;
    let mut section_of = Vec::with_capacity(lines.len());
    let mut header_line = vec![];
    for (i, l) in lines.iter().enumerate() {
        let t = trim(l);
        if t.first() == Some(&b'[') && t.last() == Some(&b']') {
            let name = String::from_utf8_lossy(&t[1..t.len() - 1])
                .trim()
                .to_string();
            header_line.push((name.clone(), i));
            section = Some(name);
        }
        section_of.push(section.clone());
    }
    Parsed {
        lines,
        section_of,
        header_line,
    }
}

fn trim(b: &[u8]) -> &[u8] {
    let ws = |c: &u8| c.is_ascii_whitespace();
    let start = b.iter().position(|c| !ws(c)).unwrap_or(b.len());
    let end = b.iter().rposition(|c| !ws(c)).map_or(start, |e| e + 1);
    &b[start..end]
}

fn entry(p: &Parsed, i: usize, section: &str, key: &str) -> Option<Entry> {
    if p.section_of[i].as_deref() != Some(section) {
        return None;
    }
    let l = p.lines[i];
    let lead = l.iter().take_while(|c| **c == b' ' || **c == b'\t').count();
    let disabled = l.get(lead) == Some(&b'#');
    let body_start = lead + usize::from(disabled);
    let eq = body_start + l[body_start..].iter().position(|&c| c == b'=')?;
    if trim(&l[body_start..eq]) != key.as_bytes() {
        return None;
    }
    let vstart = eq
        + 1
        + l[eq + 1..]
            .iter()
            .take_while(|c| **c == b' ' || **c == b'\t')
            .count();
    // no inline comments exist in the official file, but stop at ; and # anyway
    let vlen = l[vstart..]
        .iter()
        .take_while(|c| !c.is_ascii_whitespace() && **c != b';' && **c != b'#')
        .count();
    Some(Entry {
        line: i,
        disabled,
        value: (vstart, vstart + vlen),
    })
}

/// Active entry wins over a disabled one; the first of each kind is used.
fn find(p: &Parsed, section: &str, key: &str) -> Option<Entry> {
    let all: Vec<Entry> = (0..p.lines.len())
        .filter_map(|i| entry(p, i, section, key))
        .collect();
    let pos = all.iter().position(|e| !e.disabled).unwrap_or(0);
    all.into_iter().nth(pos)
}

fn number(bytes: &[u8]) -> Option<u64> {
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

/// Value of an active (not `#`) key.
#[cfg(test)]
fn active_value(ini: &[u8], section: &str, key: &str) -> Option<u64> {
    let p = parse(ini);
    let e = find(&p, section, key).filter(|e| !e.disabled)?;
    number(&p.lines[e.line][e.value.0..e.value.1])
}

/// OLA keys the player left active (they cap OLA's unlimited setting).
pub fn active_ola_keys(ini: &[u8]) -> Vec<&'static str> {
    let p = parse(ini);
    OLA_KEYS
        .iter()
        .filter(|(s, k)| find(&p, s, k).is_some_and(|e| !e.disabled))
        .map(|(_, k)| *k)
        .collect()
}

/// Applies `wants`; returns the new file and what changed (empty = untouched).
pub fn sync(ini: &[u8], wants: &[Want]) -> (Vec<u8>, Vec<Change>) {
    let p = parse(ini);
    let eol: &[u8] = if ini.windows(2).any(|w| w == b"\r\n") {
        b"\r\n"
    } else {
        b"\n"
    };
    let mut replaced: Vec<Option<Vec<u8>>> = vec![None; p.lines.len()];
    // lines to insert right after a section header, or new sections at EOF
    let mut after_header: Vec<Vec<u8>> = vec![vec![]; p.lines.len()];
    let mut new_sections: Vec<(&str, Vec<u8>)> = vec![];
    let mut changes = vec![];

    for w in wants {
        match find(&p, w.section, w.key) {
            Some(e) => {
                let l = p.lines[e.line];
                let from = number(&l[e.value.0..e.value.1]);
                let to = from.unwrap_or(0).max(w.target);
                if !e.disabled && from == Some(to) {
                    continue;
                }
                let mut out = Vec::with_capacity(l.len() + 4);
                let hash = l.iter().position(|&c| c == b'#').filter(|_| e.disabled);
                for (i, &c) in l[..e.value.0].iter().enumerate() {
                    if Some(i) != hash {
                        out.push(c);
                    }
                }
                out.extend_from_slice(to.to_string().as_bytes());
                out.extend_from_slice(&l[e.value.1..]);
                replaced[e.line] = Some(out);
                changes.push(Change {
                    key: w.key,
                    from,
                    to,
                    was_disabled: e.disabled,
                    inserted: false,
                });
            }
            None => {
                let line = [
                    w.key.as_bytes(),
                    b" = ",
                    w.target.to_string().as_bytes(),
                    eol,
                ]
                .concat();
                match p.header_line.iter().find(|(s, _)| s == w.section) {
                    Some((_, h)) => after_header[*h].extend(line),
                    None => match new_sections.iter_mut().find(|(s, _)| *s == w.section) {
                        Some((_, body)) => body.extend(line),
                        None => new_sections.push((w.section, line)),
                    },
                }
                changes.push(Change {
                    key: w.key,
                    from: None,
                    to: w.target,
                    was_disabled: false,
                    inserted: true,
                });
            }
        }
    }

    let mut out = Vec::with_capacity(ini.len() + 256);
    for (i, l) in p.lines.iter().enumerate() {
        out.extend_from_slice(replaced[i].as_deref().unwrap_or(l));
        if !after_header[i].is_empty() {
            if !l.ends_with(b"\n") {
                out.extend_from_slice(eol);
            }
            out.extend_from_slice(&after_header[i]);
        }
    }
    for (section, body) in new_sections {
        if !out.is_empty() && !out.ends_with(b"\n") {
            out.extend_from_slice(eol);
        }
        out.extend_from_slice(format!("[{section}]").as_bytes());
        out.extend_from_slice(eol);
        out.extend_from_slice(&body);
    }
    (out, changes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/");
        std::fs::read(format!("{dir}{name}")).unwrap()
    }

    fn one_car() -> Needs {
        Needs {
            max_model_id: Some(12500),
            cars: 1,
            ..Needs::default()
        }
    }

    #[test]
    fn pickups_raise_the_pickup_limit_and_turn_the_patch_on() {
        let none = plan(&one_car());
        assert!(none.iter().all(|w| w.section != "IPL"));
        let with = plan(&Needs {
            pickups: 3,
            ..one_car()
        });
        let ipl: Vec<_> = with
            .iter()
            .filter(|w| w.section == "IPL")
            .map(|w| (w.key, w.target))
            .collect();
        assert_eq!(ipl, [("Enable pickup limit patch", 1), ("Pickups", 650)]);
    }

    #[test]
    fn golden_one_car_sync() {
        if !crate::testutil::have_game_fixtures() {
            return;
        }
        let (out, _) = sync(&fixture("fla_original.ini"), &plan(&one_car()));
        assert_eq!(out, fixture("fla_expected_sync_one_car.ini"));
    }

    #[test]
    fn ola_keys_stay_disabled_and_byte_exact() {
        if !crate::testutil::have_game_fixtures() {
            return;
        }
        let original = fixture("fla_original.ini");
        let needs = Needs {
            weapons: 1,
            max_weapon_type: Some(70),
            ..one_car()
        };
        let (out, changes) = sync(&original, &plan(&needs));
        assert!(changes
            .iter()
            .all(|c| !["Vehicle Models", "Weapon Models"].contains(&c.key)));
        assert!(active_ola_keys(&out).is_empty());
        for line in ["#Vehicle Models = 212\r\n", "#Weapon Models = 51\r\n"] {
            assert!(out.windows(line.len()).any(|w| w == line.as_bytes()));
        }
        assert!(out
            .windows(31)
            .any(|w| w == b"Enable weapon type loader  = 1\r"));
    }

    #[test]
    fn second_sync_is_a_no_op_and_limits_never_shrink() {
        if !crate::testutil::have_game_fixtures() {
            return;
        }
        let (once, _) = sync(&fixture("fla_original.ini"), &plan(&one_car()));
        let (twice, changes) = sync(&once, &plan(&one_car()));
        assert_eq!(twice, once);
        assert!(changes.is_empty());
        let smaller = Needs {
            max_model_id: Some(900),
            ..one_car()
        };
        let (third, _) = sync(&once, &plan(&smaller));
        assert_eq!(
            active_value(&third, "ID LIMITS", "Count of killable model IDs"),
            Some(12550)
        );
    }

    #[test]
    fn missing_key_goes_into_its_section_not_eof() {
        let ini = b"[A]\r\nx = 1\r\n[ID LIMITS]\r\n; doc\r\n[B]\r\ny = 2";
        let want = [Want {
            section: "ID LIMITS",
            key: "Count of killable model IDs",
            target: 12550,
        }];
        let (out, changes) = sync(ini, &want);
        assert_eq!(
            out,
            b"[A]\r\nx = 1\r\n[ID LIMITS]\r\nCount of killable model IDs = 12550\r\n; doc\r\n[B]\r\ny = 2"
        );
        assert!(changes[0].inserted);
    }

    #[test]
    fn same_key_name_in_another_section_is_not_touched() {
        let ini =
            b"[X]\r\n#Apply ID limit patch = 0\r\n[ID LIMITS]\r\n#Apply ID limit patch = 0\r\n";
        let want = [Want {
            section: "ID LIMITS",
            key: "Apply ID limit patch",
            target: 1,
        }];
        let (out, _) = sync(ini, &want);
        assert_eq!(
            out,
            b"[X]\r\n#Apply ID limit patch = 0\r\n[ID LIMITS]\r\nApply ID limit patch = 1\r\n"
        );
    }

    #[test]
    fn cp1252_comments_survive() {
        let poisoned = fixture("poisoned_cp1252.ini");
        let want = [Want {
            section: "TEST",
            key: "Active key",
            target: 5,
        }];
        let (out, _) = sync(&poisoned, &want);
        assert_eq!(out, [&poisoned[..poisoned.len() - 3], b"5\r\n"].concat());
    }

    #[test]
    fn plan_uses_tutorial_floors() {
        let p = plan(&one_car());
        let get = |k| p.iter().find(|w| w.key == k).map(|w| w.target);
        assert_eq!(get("Count of killable model IDs"), Some(12550));
        assert_eq!(get("Number of standard lines"), Some(300));
        assert_eq!(get("Number of bike lines"), Some(50));
        assert_eq!(get("Enable weapon type loader"), None);
        assert!(plan(&Needs::default()).is_empty());
    }
}
