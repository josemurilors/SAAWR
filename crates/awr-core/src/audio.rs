//! gtasa_vehicleAudioSettings.cfg (FLA vehicle audio loader). Nothing below the
//! `;the end` sentinel is read, so new lines go right above it.

use crate::text::{replace_spans, tokens};

const SENTINEL: &[u8] = b";the end";

fn is_line_of(line: &[u8], short: &str) -> bool {
    !line.starts_with(b";") && {
        let t = tokens(line);
        t.first()
            .is_some_and(|&(s, e)| line[s..e].eq_ignore_ascii_case(short.as_bytes()))
    }
}

/// Clones the reference's audio line under `short`; no-op if `short` already has one.
pub fn add_clone(file: &[u8], ref_line: &[u8], short: &str) -> Vec<u8> {
    let raw: Vec<&[u8]> = file.split_inclusive(|&b| b == b'\n').collect();
    if raw.iter().any(|l| is_line_of(l, short)) {
        return file.to_vec();
    }
    let eol: &[u8] = if file.windows(2).any(|w| w == b"\r\n") {
        b"\r\n"
    } else {
        b"\n"
    };
    let new = [
        &replace_spans(ref_line, &[(tokens(ref_line)[0], short.as_bytes())])[..],
        eol,
    ]
    .concat();
    let at = raw
        .iter()
        .position(|l| l.trim_ascii().eq_ignore_ascii_case(SENTINEL));
    let mut out = Vec::with_capacity(file.len() + new.len() + 2);
    for (i, l) in raw.iter().enumerate() {
        if Some(i) == at {
            out.extend_from_slice(&new);
        }
        out.extend_from_slice(l);
    }
    if at.is_none() {
        if !out.is_empty() && !out.ends_with(b"\n") {
            out.extend_from_slice(eol);
        }
        out.extend_from_slice(&new);
    }
    out
}

pub fn remove(file: &[u8], short: &str) -> Vec<u8> {
    file.split_inclusive(|&b| b == b'\n')
        .filter(|l| !is_line_of(l, short))
        .flatten()
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_file_insert_above_sentinel_without_adding_final_newline() {
        if !crate::testutil::have_game_fixtures() {
            return;
        }
        let f = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/gtasa_vehicleAudioSettings.cfg"
        ))
        .unwrap();
        assert!(f.ends_with(b";the end"), "fixture has no final newline");
        let line = b"landstal       0  99";
        let out = add_clone(&f, line, "gtr34");
        assert!(out.ends_with(b"gtr34       0  99\r\n;the end"));
        assert_eq!(add_clone(&out, line, "gtr34"), out);
        assert_eq!(remove(&out, "gtr34"), f);
    }

    #[test]
    fn comments_mentioning_the_name_are_kept() {
        let f = b"; gtr34 note\r\ngtr34 1\r\n;the end";
        assert_eq!(remove(f, "gtr34"), b"; gtr34 note\r\n;the end");
    }
}
