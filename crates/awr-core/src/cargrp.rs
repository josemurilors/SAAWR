//! cargrp.dat traffic groups. Each data line is one group: `name, name, ... # LABEL`.

use crate::text::tokens;

/// `(names part, rest)` of a group line; None for comments and blank lines.
fn split_group(line: &[u8]) -> Option<(&[u8], &[u8])> {
    let t = line.trim_ascii();
    if t.is_empty() || t.starts_with(b"#") {
        return None;
    }
    let hash = line.iter().position(|&b| b == b'#').unwrap_or(line.len());
    Some(line.split_at(hash))
}

fn has(names: &[u8], short: &str) -> bool {
    tokens(names)
        .iter()
        .any(|&(s, e)| names[s..e].eq_ignore_ascii_case(short.as_bytes()))
}

fn map_groups(file: &[u8], f: impl Fn(&[u8], &[u8]) -> Option<Vec<u8>>) -> Vec<u8> {
    let mut out = Vec::with_capacity(file.len() + 64);
    for raw in file.split_inclusive(|&b| b == b'\n') {
        let body_len = raw.len()
            - raw
                .iter()
                .rev()
                .take_while(|b| matches!(b, b'\r' | b'\n'))
                .count();
        let (body, eol) = raw.split_at(body_len);
        match split_group(body).and_then(|(names, rest)| f(names, rest)) {
            Some(new) => {
                out.extend(new);
                out.extend_from_slice(eol);
            }
            None => out.extend_from_slice(raw),
        }
    }
    out
}

/// `short` appended to the group's names, before the trailing whitespace.
fn append(names: &[u8], rest: &[u8], short: &str) -> Option<Vec<u8>> {
    if has(names, short) {
        return None;
    }
    let end = names
        .iter()
        .rposition(|b| !b.is_ascii_whitespace() && *b != b',')?
        + 1;
    Some([&names[..end], b", ", short.as_bytes(), &names[end..], rest].concat())
}

/// Adds `short` to every group the reference is in, so the clone drives where it does.
pub fn add_like(file: &[u8], reference: &str, short: &str) -> Vec<u8> {
    map_groups(file, |names, rest| {
        if !has(names, reference) {
            return None;
        }
        append(names, rest, short)
    })
}

/// One traffic group, in file order (the order is what the game indexes by).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub index: usize,
    /// The `# LABEL` comment, e.g. `POPCYCLE_GROUP_WORKERS` or `Boats`.
    pub label: String,
    pub len: usize,
}

pub fn groups(file: &[u8]) -> Vec<Group> {
    crate::text::lines(file)
        .filter_map(split_group)
        .enumerate()
        .map(|(index, (names, rest))| Group {
            index,
            label: String::from_utf8_lossy(rest.strip_prefix(b"#").unwrap_or(rest))
                .trim()
                .to_string(),
            len: tokens(names).len(),
        })
        .collect()
}

/// Adds `short` to the group at `index` only (the player picked where it drives).
pub fn add_to(file: &[u8], index: usize, short: &str) -> Vec<u8> {
    let seen = std::cell::Cell::new(0);
    map_groups(file, |names, rest| {
        let this = seen.get();
        seen.set(this + 1);
        if this == index {
            append(names, rest, short)
        } else {
            None
        }
    })
}

/// Removes `short` from every group, with its separator.
pub fn remove(file: &[u8], short: &str) -> Vec<u8> {
    map_groups(file, |names, rest| {
        let t = tokens(names);
        let i = t
            .iter()
            .position(|&(s, e)| names[s..e].eq_ignore_ascii_case(short.as_bytes()))?;
        let (cut_start, cut_end) = match (t.get(i + 1), i.checked_sub(1)) {
            (Some(next), _) => (t[i].0, next.0),
            (None, Some(prev)) => (t[prev].1, t[i].1),
            // last member: an empty group line would break the group order
            (None, None) => return None,
        };
        Some([&names[..cut_start], &names[cut_end..], rest].concat())
    })
}

/// Largest member count of any group (FLA `Cargrp cars per group`).
pub fn max_group_len(file: &[u8]) -> u32 {
    crate::text::lines(file)
        .filter_map(split_group)
        .map(|(names, _)| tokens(names).len() as u32)
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const F: &[u8] = b"# head\r\nbuffalo, stretch, club \t# POPCYCLE_GROUP_CLUBBERS\r\npeds, bus\t# W\r\ncaddy\t# GOLFERS\r\n";

    #[test]
    fn adds_only_where_the_reference_drives() {
        let out = add_like(F, "stretch", "gtr34");
        assert_eq!(
            out,
            b"# head\r\nbuffalo, stretch, club, gtr34 \t# POPCYCLE_GROUP_CLUBBERS\r\npeds, bus\t# W\r\ncaddy\t# GOLFERS\r\n"
        );
        assert_eq!(add_like(&out, "stretch", "gtr34"), out);
    }

    #[test]
    fn lists_groups_in_order_and_adds_to_one() {
        let g = groups(F);
        let view: Vec<_> = g
            .iter()
            .map(|g| (g.index, g.label.as_str(), g.len))
            .collect();
        assert_eq!(
            view,
            [
                (0, "POPCYCLE_GROUP_CLUBBERS", 3),
                (1, "W", 2),
                (2, "GOLFERS", 1)
            ]
        );
        let out = add_to(F, 1, "gtr34");
        assert_eq!(
            out,
            b"# head\r\nbuffalo, stretch, club \t# POPCYCLE_GROUP_CLUBBERS\r\npeds, bus, gtr34\t# W\r\ncaddy\t# GOLFERS\r\n"
        );
        assert_eq!(remove(&out, "gtr34"), F);
        assert_eq!(add_to(F, 9, "gtr34"), F);
    }

    #[test]
    fn remove_restores_bytes_exactly() {
        assert_eq!(remove(&add_like(F, "stretch", "gtr34"), "gtr34"), F);
        assert_eq!(remove(F, "buffalo"), b"# head\r\nstretch, club \t# POPCYCLE_GROUP_CLUBBERS\r\npeds, bus\t# W\r\ncaddy\t# GOLFERS\r\n");
        assert_eq!(remove(F, "caddy"), F);
    }

    #[test]
    fn real_file_round_trip_and_group_size() {
        if !crate::testutil::have_game_fixtures() {
            return;
        }
        let f = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/cargrp.dat"
        ))
        .unwrap();
        let added = add_like(&f, "landstal", "gtr34");
        assert_ne!(added, f);
        assert_eq!(remove(&added, "gtr34"), f);
        assert!(max_group_len(&f) >= 20);
    }
}
