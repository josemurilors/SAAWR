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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
