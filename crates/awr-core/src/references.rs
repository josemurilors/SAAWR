//! Reference vehicles, read from the game's original data files (not a hand table):
//! `data\vehicles.ide`, `data\handling.cfg`, `data\carcols.dat` and the FLA audio cfg.

use crate::error::Result;
use crate::fsutil;
use crate::text::{lines, tokens};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VehicleRef {
    pub id: u32,
    /// IDE model name, e.g. `landstal`.
    pub model: String,
    /// handling.cfg name, e.g. `LANDSTAL`.
    pub handling_id: String,
    /// IDE type column: car, bike, bmx, boat, plane, heli, quad, mtruck, trailer, train.
    pub kind: String,
    pub ide_line: Vec<u8>,
    pub handling_line: Vec<u8>,
    /// The `!` / `%` / `$` line bikes, boats and aircraft also need.
    pub special_line: Option<Vec<u8>>,
    pub carcols: Option<CarcolsLine>,
    pub audio_line: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarcolsLine {
    /// From the `car4` section (four colours) instead of `car`.
    pub four_colors: bool,
    pub line: Vec<u8>,
}

impl VehicleRef {
    pub fn special_prefix(&self) -> Option<u8> {
        match self.kind.as_str() {
            "bike" | "bmx" => Some(b'!'),
            "boat" => Some(b'%'),
            "plane" | "heli" => Some(b'$'),
            _ => None,
        }
    }
}

pub fn load_vehicles(game_data: &Path, fla_data: Option<&Path>) -> Result<Vec<VehicleRef>> {
    let ide = fsutil::read(&game_data.join("vehicles.ide"))?;
    let handling = fsutil::read(&game_data.join("handling.cfg"))?;
    let carcols = fsutil::read(&game_data.join("carcols.dat"))?;
    let audio = match fla_data {
        Some(d) => Some(fsutil::read(&d.join("gtasa_vehicleAudioSettings.cfg"))?),
        None => None,
    };
    Ok(parse_vehicles(&ide, &handling, &carcols, audio.as_deref()))
}

fn tok<'a>(line: &'a [u8], spans: &[(usize, usize)], i: usize) -> Option<&'a [u8]> {
    spans.get(i).map(|&(s, e)| &line[s..e])
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// Lines of `section` (`cars`, `car4`, ...) in an IDE-like file with `name` / `end` blocks.
fn section<'a>(file: &'a [u8], name: &str) -> Vec<&'a [u8]> {
    let mut inside = false;
    let mut out = vec![];
    for l in lines(file) {
        let t = l.trim_ascii();
        if inside && t == b"end" {
            inside = false;
        } else if !inside && t == name.as_bytes() {
            inside = true;
        } else if inside && !t.is_empty() && !t.starts_with(b"#") {
            out.push(l);
        }
    }
    out
}

/// First line whose token `at` matches `name` (ASCII case-insensitive).
fn line_named<'a>(
    candidates: impl IntoIterator<Item = &'a [u8]>,
    at: usize,
    name: &str,
) -> Option<&'a [u8]> {
    candidates.into_iter().find(|l| {
        let t = tokens(l);
        tok(l, &t, at).is_some_and(|n| n.eq_ignore_ascii_case(name.as_bytes()))
    })
}

pub fn parse_vehicles(
    ide: &[u8],
    handling: &[u8],
    carcols: &[u8],
    audio: Option<&[u8]>,
) -> Vec<VehicleRef> {
    let handling_lines: Vec<&[u8]> = lines(handling).collect();
    let starts_with = |l: &&[u8], c: u8| l.first() == Some(&c);
    let standard = || {
        handling_lines
            .iter()
            .copied()
            .filter(|l| !matches!(l.first(), Some(b';' | b'!' | b'$' | b'%' | b'^') | None))
    };
    let car = section(carcols, "car");
    let car4 = section(carcols, "car4");
    let audio_lines: Vec<&[u8]> = audio
        .map(|a| lines(a).filter(|l| !l.starts_with(b";")).collect())
        .unwrap_or_default();

    section(ide, "cars")
        .into_iter()
        .filter_map(|l| {
            let t = tokens(l);
            let id = text(tok(l, &t, 0)?).parse().ok()?;
            let model = text(tok(l, &t, 1)?);
            let kind = text(tok(l, &t, 3)?).to_ascii_lowercase();
            let handling_id = text(tok(l, &t, 4)?);
            let handling_line = line_named(standard(), 0, &handling_id)?.to_vec();
            let mut r = VehicleRef {
                id,
                ide_line: l.to_vec(),
                handling_line,
                special_line: None,
                carcols: line_named(car4.iter().copied(), 0, &model)
                    .map(|line| CarcolsLine {
                        four_colors: true,
                        line: line.to_vec(),
                    })
                    .or_else(|| {
                        line_named(car.iter().copied(), 0, &model).map(|line| CarcolsLine {
                            four_colors: false,
                            line: line.to_vec(),
                        })
                    }),
                audio_line: line_named(audio_lines.iter().copied(), 0, &model).map(<[u8]>::to_vec),
                model,
                handling_id,
                kind,
            };
            if let Some(p) = r.special_prefix() {
                r.special_line = line_named(
                    handling_lines.iter().copied().filter(|l| starts_with(l, p)),
                    1,
                    &r.handling_id,
                )
                .map(<[u8]>::to_vec);
            }
            Some(r)
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn refs() -> Vec<VehicleRef> {
        let f = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures"));
        let read = |n: &str| std::fs::read(f.join(n)).unwrap();
        parse_vehicles(
            &read("vehicles.ide"),
            &read("handling.cfg"),
            &read("carcols.dat"),
            Some(&read("gtasa_vehicleAudioSettings.cfg")),
        )
    }

    pub(crate) fn get<'a>(r: &'a [VehicleRef], model: &str) -> &'a VehicleRef {
        r.iter().find(|v| v.model == model).unwrap()
    }

    #[test]
    fn all_212_original_vehicles_have_a_handling_line() {
        assert_eq!(refs().len(), 212);
    }

    #[test]
    fn landstal_is_a_car_with_all_lines() {
        let r = refs();
        let v = get(&r, "landstal");
        assert_eq!(
            (v.id, v.kind.as_str(), v.handling_id.as_str()),
            (400, "car", "LANDSTAL")
        );
        assert!(v.handling_line.starts_with(b"LANDSTAL "));
        assert!(v.special_line.is_none());
        assert!(v.carcols.as_ref().unwrap().line.starts_with(b"landstal,"));
        assert!(v.audio_line.as_ref().unwrap().starts_with(b"landstal "));
    }

    #[test]
    fn bikes_boats_and_aircraft_bring_their_special_line() {
        let r = refs();
        for (model, prefix) in [
            ("pcj600", b'!'),
            ("predator", b'%'),
            ("dodo", b'$'),
            ("maverick", b'$'),
        ] {
            let v = get(&r, model);
            assert_eq!(v.special_line.as_ref().unwrap()[0], prefix, "{model}");
        }
    }

    #[test]
    fn malformed_ide_lines_still_parse() {
        let r = refs();
        assert_eq!(get(&r, "emperor").kind, "car");
        assert_eq!(get(&r, "dodo").kind, "plane");
    }
}
