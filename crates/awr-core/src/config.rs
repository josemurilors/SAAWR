use crate::error::{AwrError, Result};
use crate::fsutil;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub gta_path: Option<PathBuf>,
    pub id_floor_vehicle: u32,
    pub id_floor_weapon: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        // MixMods convention: cars from 12500, weapons 12400-12499
        Self {
            gta_path: None,
            id_floor_vehicle: 12500,
            id_floor_weapon: 12400,
        }
    }
}

pub fn settings_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("br", "saawr", "sa-awr")
        .map(|d| d.config_dir().join("sa-awr.json"))
}

impl AppSettings {
    /// Missing or unreadable file means defaults: settings are a convenience, not state.
    pub fn load() -> Self {
        settings_path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = settings_path()
            .ok_or_else(|| AwrError::io("config_dir", std::io::ErrorKind::NotFound.into()))?;
        let dir = path.parent().expect("config file has a parent");
        std::fs::create_dir_all(dir).map_err(|e| AwrError::io(dir, e))?;
        let json = serde_json::to_vec_pretty(self).expect("settings always serialize");
        fsutil::write_atomic(&path, &json)
    }
}
