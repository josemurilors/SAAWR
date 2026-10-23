//! Vehicle injection (spec §8). Order is fixed because files have no joint commit:
//! build the folder in staging, edit the global files, then one rename promotes it.
//! A crash in between leaves orphan global lines plus a staging folder, which
//! `repair` knows how to clean (the staging `meta.json` names the orphan short).

use crate::checker::{self, Severity};
use crate::error::{AwrError, NotFoundKind, Result};
use crate::fla::{self, Needs};
use crate::inventory::{self, ModKey, CARS_DIR, DISABLED_PREFIX};
use crate::meta::{self, Files, Meta, ModKind, Part};
use crate::paths::InstallInfo;
use crate::readme::FoundHandling;
use crate::references::VehicleRef;
use crate::weapons::WeaponRef;
use crate::{audio, cargrp, fsutil, handling, ide, ids, tuning, validators};
use std::{collections::BTreeSet, fs, io, path::Path, path::PathBuf};

/// `. ` prefix: ModLoader ignores it, so a half-built mod is never loaded by accident.
pub(crate) const STAGING: &str = ". awr-staging";
/// Holds the one `cargrp.dat` ModLoader merges over the game's (spec §4.3).
pub const TRAFFIC_DIR: &str = "awr-transito";
pub(crate) const AUDIO_CFG: &str = "gtasa_vehicleAudioSettings.cfg";

pub struct Injector<'a> {
    pub info: &'a InstallInfo,
    pub refs: &'a [VehicleRef],
    pub weapons: &'a [WeaponRef],
    pub id_floor: u32,
}

/// What can be changed on an installed mod without reinjecting it.
pub struct EditMod<'a> {
    /// In-game name (vehicles: rewritten in the `.fxt`) or display name (weapons).
    pub name: &'a str,
    /// Vehicles only: where it shows up in traffic.
    pub traffic: Option<Traffic>,
    /// Vehicles only: clone the reference's engine sound (or not).
    pub audio: Option<bool>,
}

/// Where the new car shows up in traffic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Traffic {
    /// The same groups as the reference car.
    Auto,
    /// One group, by its index in `cargrp.dat` (see `cargrp::groups`).
    Group(usize),
    None,
}

impl Traffic {
    fn to_meta(self) -> Option<String> {
        match self {
            Traffic::Auto => Some("auto".into()),
            Traffic::Group(i) => Some(format!("group:{i}")),
            Traffic::None => None,
        }
    }
}

pub struct InjectVehicle<'a> {
    /// Friendly name shown in the HUD.
    pub name: &'a str,
    pub short: &'a str,
    pub dff: &'a Path,
    pub txd: &'a Path,
    /// IDE model name of the original vehicle to clone, e.g. `landstal`.
    pub reference: &'a str,
    pub traffic: Traffic,
    /// Clone the reference's engine sound.
    pub audio: bool,
    pub created: &'a str,
    /// Folder of `{ref}N.txd` paintjobs, renamed to `{short}N.txd`.
    pub paintjobs: Option<&'a Path>,
    /// Folder of tuning part `.dff`s, renamed per car and given their own model IDs.
    pub parts: Option<&'a Path>,
    /// ImVehFt color file; its `vehicle_id` is pointed at the new ID.
    pub ivfc: Option<&'a Path>,
    /// The pack author's handling lines, used instead of a copy of the reference's.
    pub handling: Option<&'a FoundHandling>,
    /// `CLEO` folder whose files are copied into the mod's own `cleo/` folder.
    pub cleo: Option<&'a Path>,
}

/// Files of a vehicle pack beyond the car itself, already renamed.
struct Pack {
    /// `(original stem, source file, renamed part)`.
    parts: Vec<(String, PathBuf, Part)>,
    /// `(source file, new file name)`.
    paintjobs: Vec<(PathBuf, String)>,
}

/// Files of `dir` with the given extension, sorted so IDs are deterministic.
fn files_with(dir: &Path, ext: &str) -> Result<Vec<(String, PathBuf)>> {
    let mut out: Vec<(String, PathBuf)> = io_at(dir, fs::read_dir(dir))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)))
        .filter_map(|p| Some((p.file_stem()?.to_string_lossy().into_owned(), p)))
        .collect();
    out.sort();
    Ok(out)
}

pub(crate) fn io_at<T>(path: &Path, r: io::Result<T>) -> Result<T> {
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
        let installed: Vec<Meta> = inventory::scan(&ml)
            .into_iter()
            .filter_map(|m| m.meta)
            .collect();
        let taken = self.taken_shorts();
        validators::check_short(req.short, |s| taken.contains(s))?;
        let mut used = ids::collect_used_ids(self.refs.iter().map(|v| v.id), &ml);
        used.extend(installed.iter().flat_map(|m| m.parts.iter().map(|p| p.id)));
        let id = ids::allocate_vehicle(&used, self.id_floor)?;
        used.insert(id);
        let pack = self.pack(req, r, &mut used)?;

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
            traffic_group: req.traffic.to_meta(),
            audio: (req.audio && r.audio_line.is_some()).then(|| format!("clone:{}", r.model)),
            created: req.created.into(),
            files: Files {
                dff: format!("{}.dff", req.short),
                txd: format!("{}.txd", req.short),
                fxt: Some(format!("{}.fxt", req.short)),
            },
            parts: pack.parts.iter().map(|p| p.2.clone()).collect(),
            pack_handling: req.handling.is_some(),
            weapon_config: vec![],
        };

        let stage = ml.join(STAGING).join(&folder);
        io_at(&stage, fs::create_dir_all(&stage))?;
        for (src, name) in [(req.dff, &m.files.dff), (req.txd, &m.files.txd)] {
            io_at(src, fs::copy(src, stage.join(name)))?;
        }
        for (src, name) in pack
            .parts
            .iter()
            .map(|p| (&p.1, format!("{}.dff", p.2.name)))
        {
            io_at(src, fs::copy(src, stage.join(name)))?;
        }
        for (src, name) in &pack.paintjobs {
            io_at(src, fs::copy(src, stage.join(name)))?;
        }
        if let Some(src) = req.ivfc {
            let dir = stage.join("ImVehFt").join("colors");
            io_at(&dir, fs::create_dir_all(&dir))?;
            let bytes = tuning::retarget_ivfc(&fsutil::read(src)?, id);
            fsutil::write_atomic(&dir.join(format!("{}.ivfc", req.short)), &bytes)?;
        }
        if let Some(dir) = req.cleo {
            let to = stage.join("cleo");
            io_at(&to, fs::create_dir_all(&to))?;
            for e in io_at(dir, fs::read_dir(dir))?
                .flatten()
                .filter(|e| e.path().is_file())
            {
                io_at(&e.path(), fs::copy(e.path(), to.join(e.file_name())))?;
            }
        }
        let eol = b"\r\n";
        let fxt = [&ide::fxt_line(req.short, req.name)[..], eol].concat();
        fsutil::write_atomic(
            &stage.join(m.files.fxt.as_deref().unwrap_or_default()),
            &fxt,
        )?;
        fsutil::write_atomic(
            &stage.join("linhas.txt"),
            &self.lines_file(r, id, req.short, &pack, req.handling),
        )?;
        meta::write(&stage, &m)?;

        self.globals(&m, true)?;
        io_at(&dest, fs::create_dir_all(dest.parent().unwrap_or(&ml)))?;
        io_at(&dest, fs::rename(&stage, &dest))?;
        let _ = fs::remove_dir(ml.join(STAGING));
        self.sync_fla()?;
        Ok(dest)
    }

    /// Edits an installed mod in place. The global lines are taken out with the old settings and
    /// put back with the new ones (only while it is enabled; enabling later reads `meta.json`).
    pub fn edit_mod(&self, key: &ModKey, req: &EditMod) -> Result<()> {
        self.not_running()?;
        let old = self.own_meta(key)?;
        let name = req.name.trim();
        if name.is_empty() {
            return Err(AwrError::InvalidName("o nome não pode ficar vazio".into()));
        }
        let mut new = Meta {
            name: name.to_string(),
            ..old.clone()
        };
        if old.kind == ModKind::Vehicle {
            if let Some(t) = req.traffic {
                new.traffic_group = t.to_meta();
            }
            if let Some(on) = req.audio {
                let has_line = self.reference(&old.reference)?.audio_line.is_some();
                new.audio = (on && has_line).then(|| format!("clone:{}", old.reference));
            }
            if let Some(fxt) = &old.files.fxt {
                let line = [&ide::fxt_line(&old.short, name)[..], b"\r\n"].concat();
                fsutil::write_atomic(&key.join(fxt), &line)?;
            }
        }
        let enabled = inventory::split_disabled(key).0;
        if enabled {
            self.globals(&old, false)?;
        }
        meta::write(key, &new)?;
        if enabled {
            self.globals(&new, true)?;
        }
        Ok(())
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
            if let Some(meta) = m.meta {
                self.globals(&meta, m.enabled)?;
            }
        }
        self.sync_fla()?;
        Ok(dropped)
    }

    /// Shorts that cannot be reused: original models/handling names and installed mods.
    pub fn taken_shorts(&self) -> BTreeSet<String> {
        let installed = self
            .info
            .modloader
            .as_deref()
            .map(inventory::scan)
            .unwrap_or_default();
        self.refs
            .iter()
            .flat_map(|v| [v.model.to_lowercase(), v.handling_id.to_lowercase()])
            .chain(
                self.weapons
                    .iter()
                    .flat_map(|w| [w.model.to_lowercase(), w.name.to_lowercase()]),
            )
            .chain(
                installed
                    .into_iter()
                    .filter_map(|m| m.meta.map(|m| m.short)),
            )
            .collect()
    }

    pub(crate) fn not_running(&self) -> Result<()> {
        if checker::game_running(&self.info.root) {
            return Err(AwrError::LockedByGame);
        }
        Ok(())
    }

    pub(crate) fn modloader(&self) -> Result<PathBuf> {
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
    fn lines_file(
        &self,
        r: &VehicleRef,
        id: u32,
        short: &str,
        pack: &Pack,
        own: Option<&FoundHandling>,
    ) -> Vec<u8> {
        let (standard, special) = match own {
            Some(h) => handling::from_pack(r, h, short),
            None => handling::clone_lines(r, short),
        };
        let data = self.info.data();
        let carmods = fsutil::read(&data.join("carmods.dat")).ok();
        let veh_mods = fsutil::read(&data.join("maps/veh_mods/veh_mods.ide")).ok();
        let renames: Vec<(String, String)> = pack
            .parts
            .iter()
            .map(|(stem, _, p)| (stem.clone(), p.name.clone()))
            .collect();
        let ref_txd = ide_txd(&r.ide_line);
        let mut lines = vec![
            Some(standard),
            special,
            Some(ide::clone_ide(r, id, short)),
            ide::clone_carcols(r, short),
            carmods
                .as_deref()
                .and_then(|c| tuning::carmods_line(c, &r.model))
                .map(|l| tuning::clone_carmods(l, short, &renames)),
        ];
        lines.extend(pack.parts.iter().map(|(stem, _, p)| {
            Some(tuning::part_ide(
                veh_mods.as_deref(),
                stem,
                p.id,
                &p.name,
                &ref_txd,
                short,
            ))
        }));
        lines
            .into_iter()
            .flatten()
            .flat_map(|l| [l, b"\r\n".to_vec()].concat())
            .collect()
    }

    /// Renames the pack's parts and paintjobs; part IDs come right after the car's.
    fn pack(&self, req: &InjectVehicle, r: &VehicleRef, used: &mut BTreeSet<u32>) -> Result<Pack> {
        let mut pack = Pack {
            parts: vec![],
            paintjobs: vec![],
        };
        if let Some(dir) = req.parts {
            for (stem, path) in files_with(dir, "dff")? {
                let name = tuning::rename(&stem, req.short);
                validators::check_part_name(&name)?;
                let id = ids::allocate_vehicle(used, self.id_floor)?;
                used.insert(id);
                pack.parts.push((stem, path, Part { name, id }));
            }
        }
        if let Some(dir) = req.paintjobs {
            for (stem, path) in files_with(dir, "txd")? {
                let file = format!("{stem}.txd");
                if let Some(new) = tuning::paintjob_name(&file, &r.model, req.short) {
                    pack.paintjobs.push((path, new));
                }
            }
        }
        Ok(pack)
    }

    /// Adds (or removes) the mod's lines in the audio cfg and the traffic cargrp.
    fn globals(&self, m: &Meta, add: bool) -> Result<()> {
        if m.kind == ModKind::Weapon {
            return self.weapon_globals(m, add);
        }
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
        let group = m
            .traffic_group
            .as_deref()
            .and_then(|g| g.strip_prefix("group:"))
            .and_then(|i| i.parse().ok());
        self.edit_traffic(add, |f| match (add, group) {
            (false, _) => cargrp::remove(f, &m.short),
            (true, Some(i)) => cargrp::add_to(f, i, &m.short),
            (true, None) => cargrp::add_like(f, &m.reference, &m.short),
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
    pub(crate) fn sync_fla(&self) -> Result<()> {
        let ini = self
            .info
            .fla_ini()
            .ok_or(AwrError::NotFound(NotFoundKind::FlaIni))?;
        let ml = self.modloader()?;
        let mut needs = Needs::default();
        for m in inventory::scan(&ml).into_iter().filter(|m| m.enabled) {
            let Some(m) = m.meta else { continue };
            let top = m.parts.iter().map(|p| p.id).fold(m.id, u32::max);
            needs.max_model_id = needs.max_model_id.max(Some(top));
            if m.kind == ModKind::Weapon {
                needs.weapons += 1;
                needs.max_weapon_type = needs.max_weapon_type.max(m.weapon_type_id);
                continue;
            }
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

/// TXD column of an IDE vehicle line (parts of the reference car point at it).
fn ide_txd(line: &[u8]) -> String {
    let t = crate::text::tokens(line);
    t.get(2)
        .map(|&(s, e)| String::from_utf8_lossy(&line[s..e]).into_owned())
        .unwrap_or_default()
}

/// Game-owned global file: `.bak` of the pre-AWR original, atomic write, only if it changed.
pub(crate) fn edit_global(path: &Path, f: impl FnOnce(&[u8]) -> Vec<u8>) -> Result<()> {
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
        for f in [
            "handling.cfg",
            "vehicles.ide",
            "carcols.dat",
            "cargrp.dat",
            "default.ide",
            "weapon.dat",
            "object.dat",
        ] {
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
        put(
            fla.join("data").join(crate::weapons::CONFIG),
            crate::weapons::CONFIG,
        );
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
            traffic: Traffic::Auto,
            audio: true,
            created: "2026-10-23T00:00:00Z",
            paintjobs: None,
            parts: None,
            ivfc: None,
            handling: None,
            cleo: None,
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
            weapons: &[],
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
            weapons: &[],
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
    fn pack_parts_paintjobs_and_colors_are_renamed_to_the_car() {
        if !have_game_fixtures() {
            return;
        }
        let (d, info, refs) = install();
        let r = d.path();
        let put = |rel: &str, body: &[u8]| {
            let p = r.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, body).unwrap();
        };
        put(
            "data/carmods.dat",
            b"uranus, exh_a_u, nto_b_l, wg_l_a_u
",
        );
        put(
            "data/maps/veh_mods/veh_mods.ide",
            b"objs
1090, wg_l_a_u, uranus, 100, 2097152
1092, exh_a_u, uranus, 100, 2097152
end
",
        );
        for f in [
            "src/parts/wg_l_a_u.dff",
            "src/parts/exh_a_u.dff",
            "src/pj/uranus1.txd",
            "src/pj/uranus2.txd",
            "src/pj/readme.txt",
        ] {
            put(f, b"x");
        }
        put(
            "src/c.ivfc",
            b"vehicle_id 558
num_colors 1
",
        );
        let inj = Injector {
            info: &info,
            refs: &refs,
            weapons: &[],
            id_floor: 12500,
        };
        let f = r.join("gta_sa.exe");
        let key = inj
            .inject_vehicle(&InjectVehicle {
                parts: Some(&r.join("src/parts")),
                paintjobs: Some(&r.join("src/pj")),
                ivfc: Some(&r.join("src/c.ivfc")),
                ..req("beetle", "uranus", &f)
            })
            .unwrap();
        for f in [
            "exh_a_u_beetle.dff",
            "wg_l_a_u_beetle.dff",
            "beetle1.txd",
            "beetle2.txd",
            "ImVehFt/colors/beetle.ivfc",
        ] {
            assert!(key.join(f).is_file(), "{f}");
        }
        assert!(!key.join("readme.txt").exists());
        assert_eq!(
            read(&key.join("ImVehFt/colors/beetle.ivfc")),
            b"vehicle_id 12500
num_colors 1
"
        );
        let lines = String::from_utf8(read(&key.join("linhas.txt")))
            .unwrap()
            .lines()
            .collect::<Vec<_>>()
            .join("\n");
        // car 12500, then parts in file-name order: exh_a_u 12501, wg_l_a_u 12502
        assert!(
            lines.contains(
                "beetle, exh_a_u_beetle, nto_b_l, wg_l_a_u_beetle
"
            ),
            "{lines}"
        );
        assert!(lines.contains("12501, exh_a_u_beetle, beetle, 100, 2097152"));
        assert!(lines.contains("12502, wg_l_a_u_beetle, beetle, 100, 2097152"));
        let meta = meta::read(&key).unwrap();
        assert_eq!(meta.parts.len(), 2);

        // the part IDs are taken: the next car skips them
        let next = inj.inject_vehicle(&req("other", "landstal", &f)).unwrap();
        assert!(next.ends_with("12503-other"));
    }

    #[test]
    fn traffic_can_go_to_one_chosen_group_or_none() {
        if !have_game_fixtures() {
            return;
        }
        let (d, info, refs) = install();
        let inj = Injector {
            info: &info,
            refs: &refs,
            weapons: &[],
            id_floor: 12500,
        };
        let f = d.path().join("gta_sa.exe");
        let traffic = info
            .modloader
            .clone()
            .unwrap()
            .join(TRAFFIC_DIR)
            .join("cargrp.dat");
        let key = inj
            .inject_vehicle(&InjectVehicle {
                traffic: Traffic::Group(29),
                ..req("gtr34", "landstal", &f)
            })
            .unwrap();
        let groups = cargrp::groups(&read(&traffic));
        assert_eq!(groups[29].label, "Boats");
        let line = String::from_utf8_lossy(&read(&traffic)).into_owned();
        assert_eq!(line.matches("gtr34").count(), 1, "only the chosen group");
        inj.remove(&key).unwrap();
        assert!(!traffic.exists());

        inj.inject_vehicle(&InjectVehicle {
            traffic: Traffic::None,
            ..req("other", "landstal", &f)
        })
        .unwrap();
        assert!(!traffic.exists());
    }

    #[test]
    fn pack_handling_replaces_the_reference_copy() {
        if !have_game_fixtures() {
            return;
        }
        let (d, info, refs) = install();
        let inj = Injector {
            info: &info,
            refs: &refs,
            weapons: &[],
            id_floor: 12500,
        };
        let f = d.path().join("gta_sa.exe");
        let own = FoundHandling {
            standard: b"URANUS 1315.0 3000.0".to_vec(),
            special: None,
            source: Default::default(),
        };
        let key = inj
            .inject_vehicle(&InjectVehicle {
                handling: Some(&own),
                ..req("beetle", "uranus", &f)
            })
            .unwrap();
        let lines = read(&key.join("linhas.txt"));
        assert!(lines.starts_with(b"BEETLE 1315.0 3000.0"));
        assert!(meta::read(&key).unwrap().pack_handling);
    }

    #[test]
    fn weapon_round_trip_leaves_the_global_config_untouched() {
        if !have_game_fixtures() {
            return;
        }
        let (d, info, refs) = install();
        let fla_data = info.fla_data().unwrap();
        let weapons = crate::weapons::load_weapons(&info.data(), &fla_data).unwrap();
        assert!(weapons.len() >= 30, "{}", weapons.len());
        let inj = Injector {
            info: &info,
            refs: &refs,
            weapons: &weapons,
            id_floor: 12500,
        };
        let cfg = fla_data.join(crate::weapons::CONFIG);
        let cfg0 = read(&cfg);
        let f = d.path().join("gta_sa.exe");
        let req = crate::weapons::InjectWeapon {
            short: "revolver",
            dff: &f,
            txd: &f,
            reference: "DESERT_EAGLE",
            created: "2026-10-23T00:00:00Z",
            own: None,
            special: crate::weapons::Special::None,
        };
        let key = inj.inject_weapon(&req).unwrap();
        assert!(key.ends_with("12400-revolver"));
        for n in [
            "revolver.dff",
            "revolver.txd",
            "revolver.ide",
            "Loader.txt",
            "meta.json",
        ] {
            assert!(key.join(n).is_file(), "{n}");
        }
        let loader = String::from_utf8_lossy(&read(&key.join("Loader.txt"))).into_owned();
        assert!(
            loader.contains("$ REVOLVER") && loader.contains("12400"),
            "{loader}"
        );
        let cfg1 = read(&cfg);
        let added = String::from_utf8_lossy(&cfg1[cfg0.len()..]).into_owned();
        assert!(added.trim_start().starts_with("70    REVOLVER"), "{added}");
        assert!(added.contains("DESERT_EAGLE"));
        let meta = meta::read(&key).unwrap();
        assert_eq!(
            (meta.kind, meta.weapon_type_id),
            (ModKind::Weapon, Some(70))
        );

        // a second weapon takes the next model and type IDs
        let again = inj
            .inject_weapon(&crate::weapons::InjectWeapon {
                short: "revolv2",
                ..req
            })
            .unwrap();
        assert!(again.to_string_lossy().contains("12401-revolv2"));

        let off = inj.set_enabled(&key, false).unwrap();
        assert!(read(&cfg).len() < cfg1.len());
        assert!(!String::from_utf8_lossy(&read(&cfg)).contains("REVOLVER "));
        let on = inj.set_enabled(&off, true).unwrap();
        assert!(String::from_utf8_lossy(&read(&cfg)).contains("70    REVOLVER"));
        inj.remove(&on).unwrap();
        inj.remove(&again).unwrap();
        assert_eq!(read(&cfg), cfg0, "config back to the original bytes");
    }

    #[test]
    fn editing_an_installed_car_moves_its_global_lines_and_keeps_the_rest() {
        if !have_game_fixtures() {
            return;
        }
        let (d, info, refs) = install();
        let inj = Injector {
            info: &info,
            refs: &refs,
            weapons: &[],
            id_floor: 12500,
        };
        let ml = info.modloader.clone().unwrap();
        let audio = info.fla_data().unwrap().join(AUDIO_CFG);
        let traffic = ml.join(TRAFFIC_DIR).join("cargrp.dat");
        let audio0 = read(&audio);
        let f = d.path().join("gta_sa.exe");
        let key = inj.inject_vehicle(&req("gtr34", "landstal", &f)).unwrap();
        let traffic_auto = read(&traffic);

        inj.edit_mod(
            &key,
            &EditMod {
                name: "  Exército ~r~ Novo  ",
                traffic: Some(Traffic::Group(29)),
                audio: Some(false),
            },
        )
        .unwrap();
        let m = meta::read(&key).unwrap();
        assert_eq!(
            (m.name.as_str(), m.audio.clone()),
            ("Exército ~r~ Novo", None)
        );
        assert_eq!(m.traffic_group.as_deref(), Some("group:29"));
        assert_eq!(read(&audio), audio0, "audio line gone");
        assert_ne!(
            read(&traffic),
            traffic_auto,
            "traffic moved to the chosen group"
        );
        let fxt = String::from_utf8(read(&key.join("gtr34.fxt"))).unwrap();
        assert_eq!(fxt.trim(), "GTR34 Exercito Novo");

        // disabled: only meta and fxt change, the globals are untouched until it is enabled
        let off = inj.set_enabled(&key, false).unwrap();
        inj.edit_mod(
            &off,
            &EditMod {
                name: "Parado",
                traffic: Some(Traffic::Auto),
                audio: Some(true),
            },
        )
        .unwrap();
        assert_eq!(read(&audio), audio0);
        assert!(!traffic.exists());
        let on = inj.set_enabled(&off, true).unwrap();
        assert_ne!(read(&audio), audio0, "audio back, from the edited meta");
        assert_eq!(read(&traffic), traffic_auto);

        assert!(matches!(
            inj.edit_mod(
                &on,
                &EditMod {
                    name: "  ",
                    traffic: None,
                    audio: None
                }
            ),
            Err(AwrError::InvalidName(_))
        ));
        inj.remove(&on).unwrap();
        assert_eq!(read(&audio), audio0);
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
            weapons: &[],
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
            weapons: &[],
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
            weapons: &[],
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
