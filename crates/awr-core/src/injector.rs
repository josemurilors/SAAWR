//! Vehicle injection (spec §8). Order is fixed because files have no joint commit:
//! build the folder in staging, edit the global files, then one rename promotes it.
//! A crash in between leaves orphan global lines plus a staging folder, which
//! `repair` knows how to clean (the staging `meta.json` names the orphan short).

use crate::checker::{self, Severity};
use crate::error::{AwrError, NotFoundKind, Result};
use crate::fla::{self, Needs};
use crate::inventory::{self, ModKey, CARS_DIR, DISABLED_PREFIX};
use crate::meta::{self, Files, Meta, ModKind};
use crate::paths::InstallInfo;
use crate::references::VehicleRef;
use crate::{audio, cargrp, fsutil, handling, ide, ids, validators};
use std::{fs, io, path::Path, path::PathBuf};

/// `. ` prefix: ModLoader ignores it, so a half-built mod is never loaded by accident.
const STAGING: &str = ". awr-staging";
/// Holds the one `cargrp.dat` ModLoader merges over the game's (spec §4.3).
pub const TRAFFIC_DIR: &str = "awr-transito";
const AUDIO_CFG: &str = "gtasa_vehicleAudioSettings.cfg";

pub struct Injector<'a> {
    pub info: &'a InstallInfo,
    pub refs: &'a [VehicleRef],
    pub id_floor: u32,
}

pub struct InjectVehicle<'a> {
    /// Friendly name shown in the HUD.
    pub name: &'a str,
    pub short: &'a str,
    pub dff: &'a Path,
    pub txd: &'a Path,
    /// IDE model name of the original vehicle to clone, e.g. `landstal`.
    pub reference: &'a str,
    /// Same traffic groups as the reference.
    pub traffic: bool,
    /// Clone the reference's engine sound.
    pub audio: bool,
    pub created: &'a str,
}

fn io_at<T>(path: &Path, r: io::Result<T>) -> Result<T> {
    r.map_err(|e| AwrError::io(path, e))
}

impl Injector<'_> {
    pub fn inject_vehicle(&self, req: &InjectVehicle) -> Result<ModKey> {
        let health = checker::check(self.info);
        if let Some(e) = health.iter().find(|f| f.severity == Severity::Error) {
            return Err(AwrError::Unhealthy(e.msg.clone()));
        }
        self.not_running()?;
        let ml = self.modloader()?;
        self.info
            .fla_ini()
            .ok_or(AwrError::NotFound(NotFoundKind::FlaIni))?;
        let r = self.reference(req.reference)?;
        let installed: Vec<String> = inventory::scan(&ml)
            .into_iter()
            .filter_map(|m| m.meta.map(|m| m.short))
            .collect();
        validators::check_short(req.short, |s| {
            installed.iter().any(|i| i == s)
                || self.refs.iter().any(|v| {
                    v.model.eq_ignore_ascii_case(s) || v.handling_id.eq_ignore_ascii_case(s)
                })
        })?;
        let used = ids::collect_used_ids(self.refs.iter().map(|v| v.id), &ml);
        let id = ids::allocate_vehicle(&used, self.id_floor)?;

        let folder = format!("{id}-{}", req.short);
        let dest = ml.join(CARS_DIR).join(&folder);
        if dest.exists() {
            return Err(AwrError::AlreadyExists { path: dest });
        }
        let m = Meta {
            v: 1,
            kind: ModKind::Vehicle,
            id,
            weapon_type_id: None,
            name: req.name.into(),
            short: req.short.into(),
            vehicle_type: Some(r.kind.clone()),
            reference: r.model.clone(),
            traffic_group: req.traffic.then(|| "auto".to_string()),
            audio: (req.audio && r.audio_line.is_some()).then(|| format!("clone:{}", r.model)),
            created: req.created.into(),
            files: Files {
                dff: format!("{}.dff", req.short),
                txd: format!("{}.txd", req.short),
                fxt: Some(format!("{}.fxt", req.short)),
            },
        };

        let stage = ml.join(STAGING).join(&folder);
        io_at(&stage, fs::create_dir_all(&stage))?;
        for (src, name) in [(req.dff, &m.files.dff), (req.txd, &m.files.txd)] {
            io_at(src, fs::copy(src, stage.join(name)))?;
        }
        let eol = b"\r\n";
        let fxt = [&ide::fxt_line(req.short, req.name)[..], eol].concat();
        fsutil::write_atomic(
            &stage.join(m.files.fxt.as_deref().unwrap_or_default()),
            &fxt,
        )?;
        fsutil::write_atomic(
            &stage.join("linhas.txt"),
            &self.lines_file(r, id, req.short),
        )?;
        meta::write(&stage, &m)?;

        self.globals(&m, true)?;
        io_at(&dest, fs::create_dir_all(dest.parent().unwrap_or(&ml)))?;
        io_at(&dest, fs::rename(&stage, &dest))?;
        let _ = fs::remove_dir(ml.join(STAGING));
        self.sync_fla()?;
        Ok(dest)
    }

    /// Folder rename (`. ` prefix) plus the mod's lines in the global files.
    pub fn set_enabled(&self, key: &ModKey, enabled: bool) -> Result<ModKey> {
        self.not_running()?;
        let m = self.own_meta(key)?;
        let (now, bare) = inventory::split_disabled(key);
        if now == enabled {
            return Ok(key.clone());
        }
        let name = if enabled {
            bare
        } else {
            format!("{DISABLED_PREFIX}{bare}")
        };
        let new = key.with_file_name(name);
        io_at(&new, fs::rename(key, &new))?;
        self.globals(&m, enabled)?;
        self.sync_fla()?;
        Ok(new)
    }

    pub fn remove(&self, key: &ModKey) -> Result<()> {
        self.not_running()?;
        let m = self.own_meta(key)?;
        io_at(key, fs::remove_dir_all(key))?;
        self.globals(&m, false)?;
        self.sync_fla()
    }

    /// Cleans staging leftovers and makes the global files match the folders:
    /// enabled mods have their lines, disabled and half-installed ones do not.
    /// Returns how many staging folders were dropped.
    pub fn repair(&self) -> Result<usize> {
        self.not_running()?;
        let ml = self.modloader()?;
        let staging = ml.join(STAGING);
        let mut dropped = 0;
        for dir in inventory::read_dirs(&staging) {
            if let Ok(m) = meta::read(&dir) {
                self.globals(&m, false)?;
            }
            io_at(&dir, fs::remove_dir_all(&dir))?;
            dropped += 1;
        }
        let _ = fs::remove_dir(&staging);
        for m in inventory::scan(&ml) {
            if let Some(meta) = m.meta.filter(|m| m.kind == ModKind::Vehicle) {
                self.globals(&meta, m.enabled)?;
            }
        }
        self.sync_fla()?;
        Ok(dropped)
    }

    fn not_running(&self) -> Result<()> {
        if checker::game_running(&self.info.root) {
            return Err(AwrError::LockedByGame);
        }
        Ok(())
    }

    fn modloader(&self) -> Result<PathBuf> {
        self.info
            .modloader
            .clone()
            .ok_or(AwrError::NotFound(NotFoundKind::ModLoader))
    }

    fn reference(&self, model: &str) -> Result<&VehicleRef> {
        self.refs
            .iter()
            .find(|v| v.model.eq_ignore_ascii_case(model))
            .ok_or_else(|| AwrError::InvalidName(format!("referência desconhecida: {model}")))
    }

    fn own_meta(&self, key: &ModKey) -> Result<Meta> {
        meta::read(key).map_err(|_| AwrError::ForeignMod { path: key.clone() })
    }

    /// Everything ModLoader should match by content: handling, IDE and carcols.
    fn lines_file(&self, r: &VehicleRef, id: u32, short: &str) -> Vec<u8> {
        let (standard, special) = handling::clone_lines(r, short);
        [
            Some(standard),
            special,
            Some(ide::clone_ide(r, id, short)),
            ide::clone_carcols(r, short),
        ]
        .into_iter()
        .flatten()
        .flat_map(|l| [l, b"\r\n".to_vec()].concat())
        .collect()
    }

    /// Adds (or removes) the mod's lines in the audio cfg and the traffic cargrp.
    fn globals(&self, m: &Meta, add: bool) -> Result<()> {
        let audio_cfg = self
            .info
            .fla_data()
            .ok_or(AwrError::NotFound(NotFoundKind::FlaIni))?
            .join(AUDIO_CFG);
        let reference = if add {
            Some(self.reference(&m.reference)?)
        } else {
            None
        };
        edit_global(&audio_cfg, |f| {
            match reference.and_then(|r| r.audio_line.as_deref()) {
                Some(line) if m.audio.is_some() => audio::add_clone(f, line, &m.short),
                _ if add => f.to_vec(),
                _ => audio::remove(f, &m.short),
            }
        })?;
        if add && m.traffic_group.is_none() {
            return Ok(());
        }
        self.edit_traffic(add, |f| {
            if add {
                cargrp::add_like(f, &m.reference, &m.short)
            } else {
                cargrp::remove(f, &m.short)
            }
        })
    }

    /// The traffic file starts as a copy of the game's and goes away once it is equal again.
    fn edit_traffic(&self, create: bool, f: impl FnOnce(&[u8]) -> Vec<u8>) -> Result<()> {
        let dir = self.modloader()?.join(TRAFFIC_DIR);
        let path = dir.join("cargrp.dat");
        let game = self.info.data().join("cargrp.dat");
        let old = if path.is_file() {
            fsutil::read(&path)?
        } else if create {
            fsutil::read(&game)?
        } else {
            return Ok(());
        };
        let new = f(&old);
        if new == fsutil::read(&game)? {
            if path.is_file() {
                io_at(&path, fs::remove_file(&path))?;
                let _ = fs::remove_dir(&dir);
            }
        } else if new != old || !path.is_file() {
            io_at(&dir, fs::create_dir_all(&dir))?;
            fsutil::write_atomic(&path, &new)?;
        }
        Ok(())
    }

    /// Raises the managed FLA limits to what the enabled AWR vehicles need.
    fn sync_fla(&self) -> Result<()> {
        let ini = self
            .info
            .fla_ini()
            .ok_or(AwrError::NotFound(NotFoundKind::FlaIni))?;
        let ml = self.modloader()?;
        let mut needs = Needs::default();
        for m in inventory::scan(&ml).into_iter().filter(|m| m.enabled) {
            let Some(m) = m.meta.filter(|m| m.kind == ModKind::Vehicle) else {
                continue;
            };
            needs.max_model_id = needs.max_model_id.max(Some(m.id));
            match m.vehicle_type.as_deref() {
                Some("bike" | "bmx") => needs.bikes += 1,
                Some("boat") => needs.boats += 1,
                Some("plane" | "heli") => needs.flying += 1,
                _ => needs.cars += 1,
            }
        }
        let traffic = ml.join(TRAFFIC_DIR).join("cargrp.dat");
        let file = if traffic.is_file() {
            traffic
        } else {
            self.info.data().join("cargrp.dat")
        };
        needs.max_cargrp_group_len = cargrp::max_group_len(&fsutil::read(&file)?);
        edit_global(&ini, |f| fla::sync(f, &fla::plan(&needs)).0)
    }
}

/// Game-owned global file: `.bak` of the pre-AWR original, atomic write, only if it changed.
fn edit_global(path: &Path, f: impl FnOnce(&[u8]) -> Vec<u8>) -> Result<()> {
    let old = fsutil::read(path)?;
    let new = f(&old);
    if new != old {
        fsutil::backup_once(path)?;
        fsutil::write_atomic(path, &new)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{paths::detect_install, references::load_vehicles, testutil::have_game_fixtures};

    const FIX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");

    /// A throwaway game install built from the fixtures.
    fn install() -> (tempfile::TempDir, InstallInfo, Vec<VehicleRef>) {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        let put = |to: PathBuf, from: &str| {
            fs::create_dir_all(to.parent().unwrap()).unwrap();
            fs::copy(Path::new(FIX).join(from), to).unwrap();
        };
        for f in ["handling.cfg", "vehicles.ide", "carcols.dat", "cargrp.dat"] {
            put(r.join("data").join(f), f);
        }
        let fla = r.join("modloader/$fastman92 limit adjuster");
        fs::create_dir_all(fla.join("data")).unwrap();
        fs::write(r.join("gta_sa.exe"), b"").unwrap();
        fs::write(fla.join("$fastman92limitAdjuster.asi"), b"").unwrap();
        fs::create_dir_all(r.join("modloader/OLA")).unwrap();
        fs::write(r.join("modloader/OLA/III.VC.SA.LimitAdjuster.asi"), b"").unwrap();
        put(fla.join(crate::paths::FLA_INI), "fla_original.ini");
        put(fla.join("data").join(AUDIO_CFG), AUDIO_CFG);
        let info = detect_install(r).unwrap();
        let refs = load_vehicles(&info.data(), info.fla_data().as_deref()).unwrap();
        (d, info, refs)
    }

    fn req<'a>(short: &'a str, reference: &'a str, files: &'a Path) -> InjectVehicle<'a> {
        InjectVehicle {
            name: "GT R34",
            short,
            dff: files,
            txd: files,
            reference,
            traffic: true,
            audio: true,
            created: "2026-10-23T00:00:00Z",
        }
    }

    fn read(p: &Path) -> Vec<u8> {
        fs::read(p).unwrap()
    }

    #[test]
    fn inject_disable_enable_remove_round_trip() {
        if !have_game_fixtures() {
            return;
        }
        let (d, info, refs) = install();
        let inj = Injector {
            info: &info,
            refs: &refs,
            id_floor: 12500,
        };
        let ml = info.modloader.clone().unwrap();
        let audio = info.fla_data().unwrap().join(AUDIO_CFG);
        let ini = info.fla_ini().unwrap();
        let traffic = ml.join(TRAFFIC_DIR).join("cargrp.dat");
        let game_data: Vec<_> = ["handling.cfg", "vehicles.ide", "carcols.dat", "cargrp.dat"]
            .map(|f| read(&info.data().join(f)))
            .into();
        let (audio0, ini0) = (read(&audio), read(&ini));
        let model = d.path().join("gta_sa.exe");

        let key = inj
            .inject_vehicle(&req("gtr34", "landstal", &model))
            .unwrap();
        assert_eq!(key, ml.join("awr-cars/12500-gtr34"));
        for f in [
            "gtr34.dff",
            "gtr34.txd",
            "gtr34.fxt",
            "linhas.txt",
            "meta.json",
        ] {
            assert!(key.join(f).is_file(), "{f}");
        }
        assert!(!ml.join(STAGING).exists());
        let (audio1, traffic1) = (read(&audio), read(&traffic));
        assert_ne!(audio1, audio0);
        assert_ne!(read(&ini), ini0, "FLA limits must follow the new ID");

        let off = inj.set_enabled(&key, false).unwrap();
        assert_eq!(off, ml.join("awr-cars/. 12500-gtr34"));
        assert_eq!(read(&audio), audio0);
        assert!(!traffic.exists() && !traffic.parent().unwrap().exists());

        let on = inj.set_enabled(&off, true).unwrap();
        assert_eq!(on, key);
        assert_eq!((read(&audio), read(&traffic)), (audio1, traffic1));

        inj.remove(&on).unwrap();
        assert_eq!(read(&audio), audio0);
        assert!(!traffic.exists() && !key.exists());
        let after: Vec<_> = ["handling.cfg", "vehicles.ide", "carcols.dat", "cargrp.dat"]
            .map(|f| read(&info.data().join(f)))
            .into();
        assert_eq!(after, game_data, "game data/ must stay untouched");
    }

    #[test]
    fn bike_lines_and_next_free_id() {
        if !have_game_fixtures() {
            return;
        }
        let (d, info, refs) = install();
        let inj = Injector {
            info: &info,
            refs: &refs,
            id_floor: 12500,
        };
        let f = d.path().join("gta_sa.exe");
        let car = inj.inject_vehicle(&req("gtr34", "landstal", &f)).unwrap();
        inj.set_enabled(&car, false).unwrap();
        let bike = inj.inject_vehicle(&req("mybike", "pcj600", &f)).unwrap();
        assert!(
            bike.ends_with("12501-mybike"),
            "disabled mods keep their ID"
        );
        let lines = read(&bike.join("linhas.txt"));
        assert!(lines.starts_with(b"MYBIKE"));
        assert!(lines.windows(3).any(|w| w == b"\r\n!"), "special bike line");
        assert!(lines.ends_with(b"\r\n"));
    }

    #[test]
    fn rejects_taken_shorts_and_unknown_references() {
        if !have_game_fixtures() {
            return;
        }
        let (d, info, refs) = install();
        let inj = Injector {
            info: &info,
            refs: &refs,
            id_floor: 12500,
        };
        let f = d.path().join("gta_sa.exe");
        inj.inject_vehicle(&req("gtr34", "landstal", &f)).unwrap();
        for short in ["gtr34", "landstal"] {
            let e = inj.inject_vehicle(&req(short, "landstal", &f)).unwrap_err();
            assert!(
                matches!(
                    e,
                    AwrError::DuplicateShort { .. } | AwrError::InvalidName(_)
                ),
                "{short}: {e}"
            );
        }
        let e = inj.inject_vehicle(&req("abc", "nope", &f)).unwrap_err();
        assert!(matches!(e, AwrError::InvalidName(_)));
    }

    #[test]
    fn repair_drops_a_crashed_injection() {
        if !have_game_fixtures() {
            return;
        }
        let (d, info, refs) = install();
        let inj = Injector {
            info: &info,
            refs: &refs,
            id_floor: 12500,
        };
        let ml = info.modloader.clone().unwrap();
        let audio = info.fla_data().unwrap().join(AUDIO_CFG);
        let audio0 = read(&audio);
        let f = d.path().join("gta_sa.exe");
        let key = inj.inject_vehicle(&req("gtr34", "landstal", &f)).unwrap();
        // simulate a crash after the global edits and before the final rename
        let stage = ml.join(STAGING).join("12500-gtr34");
        fs::create_dir_all(stage.parent().unwrap()).unwrap();
        fs::rename(&key, &stage).unwrap();
        assert_ne!(read(&audio), audio0);

        assert_eq!(inj.repair().unwrap(), 1);
        assert_eq!(read(&audio), audio0);
        assert!(!ml.join(STAGING).exists());
        assert!(!ml.join(TRAFFIC_DIR).exists());
    }

    #[test]
    fn foreign_folders_are_never_touched() {
        if !have_game_fixtures() {
            return;
        }
        let (_d, info, refs) = install();
        let inj = Injector {
            info: &info,
            refs: &refs,
            id_floor: 12500,
        };
        let foreign = info.modloader.clone().unwrap().join("awr-cars/12500-x");
        fs::create_dir_all(&foreign).unwrap();
        assert!(matches!(
            inj.remove(&foreign),
            Err(AwrError::ForeignMod { .. })
        ));
        assert!(foreign.exists());
    }
}
