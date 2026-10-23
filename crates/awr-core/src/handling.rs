//! handling.cfg line cloning. SA handling lines have no model ID column:
//! only the name changes, every separator of the reference line is kept.

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::references::tests::{get, refs};

    #[test]
    fn landstal_clone_keeps_tabs_and_spaces() {
        let r = refs();
        let (line, special) = clone_lines(get(&r, "landstal"), "gtr34");
        // "LANDSTAL" is 8 bytes; everything after it must be untouched
        let expected = [b"GTR34", &get(&r, "landstal").handling_line[8..]].concat();
        assert_eq!(line, expected);
        assert!(special.is_none());
    }

    #[test]
    fn bike_clone_renames_both_lines() {
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
