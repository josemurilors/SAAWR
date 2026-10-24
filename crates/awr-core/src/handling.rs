//! handling.cfg line cloning. SA handling lines have no model ID column:
//! only the name changes, every separator of the reference line is kept.

use crate::readme::FoundHandling;
use crate::references::VehicleRef;
use crate::text::{replace_spans, tokens};

/// Standard line, plus the `!`/`%`/`$` line when the reference has one.
/// The handling name must be uppercase (MixMods tutorial).
pub fn clone_lines(r: &VehicleRef, short: &str) -> (Vec<u8>, Option<Vec<u8>>) {
    let name = short.to_ascii_uppercase();
    let rename =
        |line: &[u8], at: usize| replace_spans(line, &[(tokens(line)[at], name.as_bytes())]);
    let standard = rename(&r.handling_line, 0);
    let special = r.special_line.as_deref().map(|l| rename(l, 1));
    (standard, special)
}

/// The pack author's own lines, renamed to `short`. The special line is used only if it
/// is of the kind the reference needs (bike `!`, boat `%`, aircraft `$`); otherwise the
/// reference's own special line stays, renamed.
pub fn from_pack(r: &VehicleRef, h: &FoundHandling, short: &str) -> (Vec<u8>, Option<Vec<u8>>) {
    let name = short.to_ascii_uppercase();
    let rename =
        |line: &[u8], at: usize| replace_spans(line, &[(tokens(line)[at], name.as_bytes())]);
    let special = match (&h.special, r.special_prefix()) {
        (Some(s), Some(p)) if s.first() == Some(&p) => Some(rename(s, 1)),
        _ => clone_lines(r, short).1,
    };
    (rename(&h.standard, 0), special)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::references::tests::{get, refs};

    #[test]
    fn landstal_clone_keeps_tabs_and_spaces() {
        if !crate::testutil::have_game_fixtures() {
            return;
        }
        let r = refs();
        let (line, special) = clone_lines(get(&r, "landstal"), "gtr34");
        // "LANDSTAL" is 8 bytes; everything after it must be untouched
        let expected = [b"GTR34", &get(&r, "landstal").handling_line[8..]].concat();
        assert_eq!(line, expected);
        assert!(special.is_none());
    }

    fn bare(kind: &str, special: Option<&[u8]>) -> VehicleRef {
        VehicleRef {
            id: 468,
            model: "sanchez".into(),
            handling_id: "SANCHEZ".into(),
            kind: kind.into(),
            ide_line: vec![],
            handling_line: b"SANCHEZ 1 2".to_vec(),
            special_line: special.map(<[u8]>::to_vec),
            carcols: None,
            audio_line: None,
        }
    }

    #[test]
    fn pack_lines_win_but_the_special_line_must_fit_the_vehicle() {
        let h = FoundHandling {
            standard: b"DIRTBIKE 500.0 195.0".to_vec(),
            special: Some(b"!	DIRTBIKE	0.4	0.4".to_vec()),
            carcols: None,
            source: Default::default(),
        };
        let bike = bare("bike", Some(b"!	SANCHEZ	0.9"));
        let (std, sp) = from_pack(&bike, &h, "bmw");
        assert_eq!(std, b"BMW 500.0 195.0");
        assert_eq!(sp.unwrap(), b"!	BMW	0.4	0.4");

        // pack ships no special line: the reference's, renamed
        let none = FoundHandling {
            special: None,
            ..h.clone()
        };
        assert_eq!(from_pack(&bike, &none, "bmw").1.unwrap(), b"!	BMW	0.9");
        // a car never gets a special line, even if the readme lists one
        assert!(from_pack(&bare("car", None), &h, "bmw").1.is_none());
    }

    #[test]
    fn bike_clone_renames_both_lines() {
        if !crate::testutil::have_game_fixtures() {
            return;
        }
        let r = refs();
        let (line, special) = clone_lines(get(&r, "pcj600"), "mybike");
        assert!(line.starts_with(b"MYBIKE"));
        let special = special.unwrap();
        assert!(special.starts_with(b"!"));
        assert_eq!(
            tokens(&special).len(),
            tokens(get(&r, "pcj600").special_line.as_ref().unwrap()).len()
        );
        let t = tokens(&special)[1];
        assert_eq!(&special[t.0..t.1], b"MYBIKE");
    }
}
