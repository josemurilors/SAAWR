//! `meta.json`: the tool's record inside each AWR mod folder (spec §7).
//! Enabled/foreign are NOT stored here: they come from the folder itself.

use crate::error::{AwrError, Result};
use crate::fsutil;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const FILE: &str = "meta.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModKind {
    Vehicle,
    Weapon,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Files {
    pub dff: String,
    pub txd: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fxt: Option<String>,
}

/// A renamed tuning part owned by the car (model name + its own model ID).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    pub id: u32,
}

/// How a weapon is sold by the Custom Ammu-Nation mod (see `shop.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shop {
    /// 0 pistols .. 10 melee, as in the mod's `ammun.dat`.
    pub category: u8,
    /// 0..=10: which mission unlocks it.
    pub unlock: u8,
    /// Shop clerk animation: 0 one hand, 1 two hands, 2 rifle.
    pub anim: u8,
    pub price: u32,
    pub ammo: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meta {
    pub v: u32,
    pub kind: ModKind,
    pub id: u32,
    pub weapon_type_id: Option<u32>,
    pub name: String,
    pub short: String,
    /// Type column of the reference's IDE line (car, bike, boat, plane, heli, ...).
    pub vehicle_type: Option<String>,
    pub reference: String,
    pub traffic_group: Option<String>,
    pub audio: Option<String>,
    pub created: String,
    pub files: Files,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<Part>,
    /// Handling came from the pack's readme instead of a copy of the reference.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pack_handling: bool,
    /// Weapons: the exact lines added to `gtasa_weapon_config.dat` (CP-1252 as text), so
    /// turning the mod back on needs nothing but this file.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub weapon_config: Vec<String>,
    /// Weapons: sold in the Ammu-Nation (needs the Custom Ammu-Nation mod).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shop: Option<Shop>,
}

/// UTC timestamp for `created`, from Unix seconds (civil-from-days, no date crate).
pub fn rfc3339(secs: u64) -> String {
    let (days, rem) = ((secs / 86400) as i64, secs % 86400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let (h, m, s) = (rem / 3600, rem % 3600 / 60, rem % 60);
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
}

pub fn read(dir: &Path) -> Result<Meta> {
    let path = dir.join(FILE);
    let bytes = fsutil::read(&path)?;
    serde_json::from_slice(&bytes).map_err(|e| AwrError::Meta {
        path,
        msg: e.to_string(),
    })
}

pub fn write(dir: &Path, meta: &Meta) -> Result<()> {
    let json = serde_json::to_vec(meta).expect("meta always serializes");
    fsutil::write_atomic(&dir.join(FILE), &json)
}

#[cfg(test)]
pub(crate) fn sample(id: u32, short: &str) -> Meta {
    Meta {
        v: 1,
        kind: ModKind::Vehicle,
        id,
        weapon_type_id: None,
        name: short.to_uppercase(),
        short: short.into(),
        vehicle_type: Some("car".into()),
        reference: "LANDSTAL".into(),
        traffic_group: None,
        audio: Some("clone:LANDSTAL".into()),
        created: "2026-10-23T00:00:00Z".into(),
        files: Files {
            dff: format!("{short}.dff"),
            txd: format!("{short}.txd"),
            fxt: None,
        },
        parts: vec![],
        pack_handling: false,
        weapon_config: vec![],
        shop: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_unix_seconds() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(951_782_400 + 3661), "2000-02-29T01:01:01Z");
    }

    #[test]
    fn round_trips() {
        let d = tempfile::tempdir().unwrap();
        let m = sample(12500, "gtr34");
        write(d.path(), &m).unwrap();
        assert_eq!(read(d.path()).unwrap(), m);
    }

    #[test]
    fn broken_json_is_a_meta_error() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join(FILE), b"{").unwrap();
        assert!(matches!(read(d.path()), Err(AwrError::Meta { .. })));
    }
}
