//! vehicles.ide, carcols.dat and .fxt lines for a new vehicle.

use crate::references::VehicleRef;
use crate::text::{replace_spans, tokens};

/// IDE columns: 0 id, 1 model, 2 txd, 3 type, 4 handling id, 5 game name (fxt key).
pub fn clone_ide(r: &VehicleRef, id: u32, short: &str) -> Vec<u8> {
    let t = tokens(&r.ide_line);
    let id = id.to_string();
    let upper = short.to_ascii_uppercase();
    replace_spans(
        &r.ide_line,
        &[
            (t[0], id.as_bytes()),
            (t[1], short.as_bytes()),
            (t[2], short.as_bytes()),
            (t[4], upper.as_bytes()),
            (t[5], upper.as_bytes()),
        ],
    )
}

/// Same colours as the reference; None if the reference has no carcols line.
pub fn clone_carcols(r: &VehicleRef, short: &str) -> Option<Vec<u8>> {
    let c = r.carcols.as_ref()?;
    Some(replace_spans(
        &c.line,
        &[(tokens(&c.line)[0], short.as_bytes())],
    ))
}

/// `{SHORT} {friendly name}`: the HUD name shown when entering the vehicle.
pub fn fxt_line(short: &str, name: &str) -> Vec<u8> {
    let shown = hud_name(name);
    let shown = if shown.is_empty() {
        short.to_ascii_uppercase()
    } else {
        shown
    };
    [
        short.to_ascii_uppercase().as_bytes(),
        b" ",
        shown.as_bytes(),
    ]
    .concat()
}

/// Longest name the HUD shows without clipping.
pub const HUD_NAME_MAX: usize = 24;

/// What the game can show: the font has no accents of its own for ModLoader's `.fxt`, `~`
/// opens color/control codes and other symbols print as boxes. So: accents folded to ASCII,
/// only letters, digits, space and `. ' ( ) & / + -` kept, spaces collapsed, cut at 24.
pub fn hud_name(name: &str) -> String {
    let kept: String = crate::text::fold_ascii(name)
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || " .'()&/+-".contains(c) {
                c
            } else {
                ' '
            }
        })
        .collect();
    let words: Vec<&str> = kept.split_whitespace().collect();
    let mut out = words.join(" ");
    out.truncate(HUD_NAME_MAX);
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::references::tests::{get, refs};

    #[test]
    fn ide_clone_changes_only_the_named_columns() {
        if !crate::testutil::have_game_fixtures() {
            return;
        }
        let r = refs();
        let line = clone_ide(get(&r, "landstal"), 12500, "gtr34");
        assert_eq!(
            line,
            b"12500,\tgtr34, \tgtr34, \tcar, \t\tGTR34, \tGTR34, \tnull,\tnormal, \t10,\t0,\t0,\t\t-1, 0.768, 0.768,\t0"
        );
    }

    #[test]
    fn malformed_reference_line_still_clones() {
        if !crate::testutil::have_game_fixtures() {
            return;
        }
        let r = refs();
        let line = clone_ide(get(&r, "emperor"), 12501, "emp2");
        assert!(line.starts_with(b"12501,\temp2\t\temp2, \tcar, \t\tEMP2, \tEMP2,"));
    }

    #[test]
    fn carcols_and_fxt() {
        if !crate::testutil::have_game_fixtures() {
            return;
        }
        let r = refs();
        let c = clone_carcols(get(&r, "landstal"), "gtr34").unwrap();
        assert_eq!(
            c,
            b"gtr34, 4,1, 123,1, 113,1, 101,1, 75,1, 62,1, 40,1, 36,1"
        );
        assert_eq!(fxt_line("gtr34", "GT R34 Ação"), b"GTR34 GT R34 Acao");
        assert_eq!(fxt_line("x", "~~~"), b"X X");
        assert_eq!(fxt_line("x", "~r~Red"), b"X r Red");
    }
}
