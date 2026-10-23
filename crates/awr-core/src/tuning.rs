//! Tuning parts, paintjobs and IVF colors of a vehicle pack, renamed so they belong to
//! the new model and never touch the reference car's own parts.

use crate::text::{lines, replace_spans, tokens};

/// Model names stay unique per car: `exh_a_u` -> `exh_a_u_beetle`.
/// The `_l_` / `_r_` pair the game looks up by name keeps working.
pub fn rename(stem: &str, short: &str) -> String {
    format!("{}_{short}", stem.to_ascii_lowercase())
}

fn token_is(line: &[u8], at: usize, name: &str) -> bool {
    tokens(line)
        .get(at)
        .is_some_and(|&(s, e)| line[s..e].eq_ignore_ascii_case(name.as_bytes()))
}

/// The reference car's line in `carmods.dat`.
pub fn carmods_line<'a>(carmods: &'a [u8], model: &str) -> Option<&'a [u8]> {
    lines(carmods).find(|l| !l.starts_with(b"#") && token_is(l, 0, model))
}

/// Same part list under `short`; parts without a renamed file (nitro, wheels) stay vanilla.
pub fn clone_carmods(line: &[u8], short: &str, renames: &[(String, String)]) -> Vec<u8> {
    let t = tokens(line);
    let mut edits: Vec<((usize, usize), &[u8])> = vec![(t[0], short.as_bytes())];
    for &(s, e) in &t[1..] {
        if let Some((_, new)) = renames
            .iter()
            .find(|(old, _)| line[s..e].eq_ignore_ascii_case(old.as_bytes()))
        {
            edits.push(((s, e), new.as_bytes()));
        }
    }
    replace_spans(line, &edits)
}

/// `id, name, txd, draw distance, flags` for a renamed part: the reference's line with
/// the ID and name swapped, and its own TXD (`ref_txd`) pointed at the new car's.
pub fn part_ide(
    veh_mods: Option<&[u8]>,
    stem: &str,
    id: u32,
    name: &str,
    ref_txd: &str,
    short: &str,
) -> Vec<u8> {
    let line =
        veh_mods.and_then(|f| lines(f).find(|l| !l.starts_with(b"#") && token_is(l, 1, stem)));
    let Some(line) = line else {
        return format!("{id}, {name}, vehicle, 70, 0").into_bytes();
    };
    let t = tokens(line);
    let id = id.to_string();
    let mut edits: Vec<((usize, usize), &[u8])> =
        vec![(t[0], id.as_bytes()), (t[1], name.as_bytes())];
    if token_is(line, 2, ref_txd) {
        edits.push((t[2], short.as_bytes()));
    }
    replace_spans(line, &edits)
}

/// `uranus3.txd` -> `beetle3.txd`; None for files that are not this car's paintjobs.
pub fn paintjob_name(file: &str, ref_model: &str, short: &str) -> Option<String> {
    let lower = file.to_ascii_lowercase();
    let n = lower.strip_prefix(ref_model)?.strip_suffix(".txd")?;
    (!n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())).then(|| format!("{short}{n}.txd"))
}

/// IVF color files are keyed by `vehicle_id`; point it at the new model ID.
pub fn retarget_ivfc(bytes: &[u8], id: u32) -> Vec<u8> {
    let id = id.to_string();
    bytes
        .split_inclusive(|&b| b == b'\n')
        .flat_map(|l| {
            let t = tokens(l);
            match t.as_slice() {
                [(s, e), v, ..] if l[*s..*e].eq_ignore_ascii_case(b"vehicle_id") => {
                    replace_spans(l, &[(*v, id.as_bytes())])
                }
                _ => l.to_vec(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODS: &[u8] =
        b"objs\r\n1090, wg_l_a_u, uranus, 100, 2097152\r\n1048, nto_b_l, vehicle, 70, 0\r\nend\r\n";

    #[test]
    fn carmods_keeps_vanilla_parts_and_renames_ours() {
        let cm = b"# c\r\nuranus, exh_a_u, nto_b_l, wg_l_a_u\r\n";
        let l = carmods_line(cm, "uranus").unwrap();
        let renames = [("wg_l_a_u".to_string(), rename("wg_l_a_u", "beetle"))];
        assert_eq!(
            clone_carmods(l, "beetle", &renames),
            b"beetle, exh_a_u, nto_b_l, wg_l_a_u_beetle"
        );
        assert!(carmods_line(cm, "infernus").is_none());
    }

    #[test]
    fn part_ide_swaps_id_name_and_own_txd_only() {
        let own = part_ide(
            Some(MODS),
            "wg_l_a_u",
            12503,
            "wg_l_a_u_beetle",
            "uranus",
            "beetle",
        );
        assert_eq!(own, b"12503, wg_l_a_u_beetle, beetle, 100, 2097152");
        let shared = part_ide(Some(MODS), "nto_b_l", 12504, "x", "uranus", "beetle");
        assert_eq!(shared, b"12504, x, vehicle, 70, 0");
        let unknown = part_ide(None, "zzz", 12505, "zzz_b", "uranus", "beetle");
        assert_eq!(unknown, b"12505, zzz_b, vehicle, 70, 0");
    }

    #[test]
    fn paintjobs_follow_the_reference_name() {
        assert_eq!(
            paintjob_name("uranus3.txd", "uranus", "beetle").unwrap(),
            "beetle3.txd"
        );
        assert_eq!(
            paintjob_name("URANUS12.TXD", "uranus", "beetle").unwrap(),
            "beetle12.txd"
        );
        for no in ["uranus.txd", "uranusx.txd", "other1.txd", "uranus1.dff"] {
            assert!(paintjob_name(no, "uranus", "beetle").is_none(), "{no}");
        }
    }

    #[test]
    fn ivfc_vehicle_id_is_retargeted() {
        let f = b"#c\r\nvehicle_id 558\r\nnum_colors 9\r\n";
        assert_eq!(
            retarget_ivfc(f, 12502),
            b"#c\r\nvehicle_id 12502\r\nnum_colors 9\r\n"
        );
    }
}
