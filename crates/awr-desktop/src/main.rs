//! SA-AWR desktop shell: a thin Tauri layer over `awr-core`. The React front end (`ui/`)
//! talks to these commands; no business logic lives here.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use awr_core::{
    batch::{self, Candidate},
    cargrp::{self, Group},
    checker::{self, Severity},
    config::AppSettings,
    fsutil, ide,
    injector::{InjectVehicle, Injector, Traffic},
    inventory::{self, InstalledMod},
    meta::{self, ModKind},
    paths::{self, InstallInfo},
    references::{self, VehicleRef},
    validators,
    weapons::{self, InjectWeapon, Special, WeaponRef},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::SystemTime,
};
use tauri::State;

/// An opened GTA SA install plus what can be cloned from it.
struct Game {
    info: InstallInfo,
    refs: Vec<VehicleRef>,
    weapons: Vec<WeaponRef>,
    groups: Vec<Group>,
}

#[derive(Default)]
struct AppState {
    game: Mutex<Option<Game>>,
    settings: Mutex<AppSettings>,
    /// Scanned packs waiting in the queue, by the id the front end knows them by.
    queue: Mutex<HashMap<u64, Candidate>>,
    next_id: AtomicU64,
}

// ---- data sent to the front end -------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    gta_path: String,
    game: Option<GameDto>,
    mods: Vec<ModDto>,
    findings: Vec<FindingDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GameDto {
    root: String,
    modloader: bool,
    fla: bool,
    ola: bool,
    refs: Vec<RefDto>,
    weapons: Vec<WeaponDto>,
    groups: Vec<GroupDto>,
}

#[derive(Serialize)]
struct RefDto {
    model: String,
    kind: String,
}

#[derive(Serialize)]
struct WeaponDto {
    name: String,
    melee: bool,
}

#[derive(Serialize)]
struct GroupDto {
    index: usize,
    label: String,
    len: usize,
}

#[derive(Serialize)]
struct FindingDto {
    severity: &'static str,
    msg: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ModDto {
    key: String,
    id: Option<u32>,
    enabled: bool,
    foreign: bool,
    kind: Option<&'static str>,
    name: String,
    short: String,
    reference: String,
    vehicle_type: Option<String>,
    weapon_type_id: Option<u32>,
    parts: usize,
    paintjobs: usize,
    ivfc: bool,
    pack_handling: bool,
    traffic: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ItemDto {
    id: u64,
    name: String,
    short: String,
    reference: String,
    weapon: bool,
    folder: String,
    extras: Extras,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Extras {
    parts: bool,
    paintjobs: bool,
    ivfc: bool,
    handling: bool,
    weapon_lines: bool,
    cleo: bool,
}

#[derive(Serialize)]
struct Problem {
    id: u64,
    problem: Option<String>,
}

#[derive(Serialize)]
struct InjectResult {
    id: u64,
    ok: bool,
    message: String,
}

// ---- data received from the front end -------------------------------------------------

#[derive(Deserialize)]
struct Check {
    id: u64,
    short: String,
    weapon: bool,
    reference: String,
}

#[derive(Deserialize)]
struct TrafficDto {
    kind: String,
    index: Option<usize>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InjectRequest {
    id: u64,
    name: String,
    short: String,
    reference: String,
    weapon: bool,
    traffic: TrafficDto,
    audio: bool,
    use_handling: bool,
    use_cleo: bool,
    /// Weapons: "silencer", "jetpack" or "binoculars".
    special: Option<String>,
}

// ---- helpers --------------------------------------------------------------------------

fn load_game(root: &Path) -> Result<Game, String> {
    let info = paths::detect_install(root).map_err(|e| e.to_string())?;
    let refs = references::load_vehicles(&info.data(), info.fla_data().as_deref())
        .map_err(|e| e.to_string())?;
    let groups = fsutil::read(&info.data().join("cargrp.dat"))
        .map(|f| cargrp::groups(&f))
        .unwrap_or_default();
    let weapons = info
        .fla_data()
        .and_then(|d| weapons::load_weapons(&info.data(), &d).ok())
        .unwrap_or_default();
    Ok(Game {
        info,
        refs,
        weapons,
        groups,
    })
}

fn injector<'a>(g: &'a Game, settings: &AppSettings) -> Injector<'a> {
    Injector {
        info: &g.info,
        refs: &g.refs,
        weapons: &g.weapons,
        id_floor: settings.id_floor_vehicle,
    }
}

fn group_label(g: &Group) -> String {
    let l = g.label.strip_prefix("POPCYCLE_GROUP_").unwrap_or(&g.label);
    format!("{} - {}", g.index, l.replace('_', " ").to_lowercase())
}

fn mod_dto(m: &InstalledMod, groups: &[Group]) -> ModDto {
    let meta = m.meta.as_ref();
    let short = meta.map(|x| x.short.clone()).unwrap_or_default();
    let paintjobs = std::fs::read_dir(&m.key)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_lowercase();
            n.strip_prefix(&short)
                .and_then(|r| r.strip_suffix(".txd"))
                .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
        })
        .count();
    let traffic = match meta.and_then(|x| x.traffic_group.as_deref()) {
        None => "sem tráfego".to_string(),
        Some("auto") => "como a referência".to_string(),
        Some(g) => g
            .strip_prefix("group:")
            .and_then(|i| i.parse::<usize>().ok())
            .and_then(|i| groups.get(i))
            .map_or(g.to_string(), group_label),
    };
    ModDto {
        key: m.key.display().to_string(),
        id: m.id,
        enabled: m.enabled,
        foreign: meta.is_none(),
        kind: meta.map(|x| match x.kind {
            ModKind::Vehicle => "vehicle",
            ModKind::Weapon => "weapon",
        }),
        name: meta.map_or_else(
            || {
                m.key
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            },
            |x| x.name.clone(),
        ),
        short,
        reference: meta.map(|x| x.reference.clone()).unwrap_or_default(),
        vehicle_type: meta.and_then(|x| x.vehicle_type.clone()),
        weapon_type_id: meta.and_then(|x| x.weapon_type_id),
        parts: meta.map_or(0, |x| x.parts.len()),
        paintjobs,
        ivfc: m.key.join("ImVehFt").is_dir(),
        pack_handling: meta.is_some_and(|x| x.pack_handling),
        traffic,
    }
}

fn snapshot_of(state: &AppState) -> Snapshot {
    let settings = state.settings.lock().unwrap();
    let game = state.game.lock().unwrap();
    let gta_path = settings
        .gta_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let Some(g) = game.as_ref() else {
        return Snapshot {
            gta_path,
            game: None,
            mods: vec![],
            findings: vec![],
        };
    };
    let mods = g
        .info
        .modloader
        .as_deref()
        .map(inventory::scan)
        .unwrap_or_default()
        .iter()
        .map(|m| mod_dto(m, &g.groups))
        .collect();
    let findings = checker::check(&g.info)
        .into_iter()
        .map(|f| FindingDto {
            severity: match f.severity {
                Severity::Error => "error",
                Severity::Warn => "warn",
            },
            msg: f.msg,
        })
        .collect();
    Snapshot {
        gta_path,
        game: Some(GameDto {
            root: g.info.root.display().to_string(),
            modloader: g.info.modloader.is_some(),
            fla: g.info.fla_ini().is_some(),
            ola: g.info.ola_asi.is_some(),
            refs: g
                .refs
                .iter()
                .map(|r| RefDto {
                    model: r.model.clone(),
                    kind: r.kind.clone(),
                })
                .collect(),
            weapons: g
                .weapons
                .iter()
                .map(|w| WeaponDto {
                    name: w.name.clone(),
                    melee: w.melee,
                })
                .collect(),
            groups: g
                .groups
                .iter()
                .map(|x| GroupDto {
                    index: x.index,
                    label: group_label(x),
                    len: x.len,
                })
                .collect(),
        }),
        mods,
        findings,
    }
}

fn item_dto(id: u64, c: &Candidate) -> ItemDto {
    ItemDto {
        id,
        name: c.name.clone(),
        short: c.short.clone(),
        reference: c.reference.clone().unwrap_or_default(),
        weapon: c.weapon,
        folder: c.folder.display().to_string(),
        extras: Extras {
            parts: c.files.parts.is_some(),
            paintjobs: c.files.paintjobs.is_some(),
            ivfc: c.files.ivfc.is_some(),
            handling: c.files.handling.is_some(),
            weapon_lines: c.files.weapon.is_some(),
            cleo: c.files.cleo.is_some(),
        },
    }
}

// ---- commands -------------------------------------------------------------------------

#[tauri::command]
async fn snapshot(state: State<'_, AppState>) -> Result<Snapshot, String> {
    Ok(snapshot_of(&state))
}

/// Opens the saved install on startup (no-op without one).
#[tauri::command]
async fn startup(state: State<'_, AppState>) -> Result<Snapshot, String> {
    let settings = AppSettings::load();
    let saved = settings.gta_path.clone();
    *state.settings.lock().unwrap() = settings;
    if let Some(root) = saved {
        if let Ok(g) = load_game(&root) {
            *state.game.lock().unwrap() = Some(g);
        }
    }
    Ok(snapshot_of(&state))
}

#[tauri::command]
async fn open_game(path: String, state: State<'_, AppState>) -> Result<Snapshot, String> {
    let root = PathBuf::from(path.trim());
    let game = load_game(&root)?;
    {
        let mut s = state.settings.lock().unwrap();
        s.gta_path = Some(root);
        s.save().map_err(|e| e.to_string())?;
    }
    *state.game.lock().unwrap() = Some(game);
    Ok(snapshot_of(&state))
}

/// Dropped or picked paths -> queue items (the packs stay here, the front end gets ids).
#[tauri::command]
async fn scan_paths(
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ItemDto>, String> {
    let game = state.game.lock().unwrap();
    let g = game
        .as_ref()
        .ok_or("abra a pasta do GTA SA antes de adicionar veículos")?;
    let settings = state.settings.lock().unwrap();
    let mut taken = injector(g, &settings).taken_shorts();
    // shorts already in the queue are not free either
    let mut queue = state.queue.lock().unwrap();
    taken.extend(queue.values().map(|c| c.short.clone()));
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let found = batch::from_paths(&paths, &g.refs, &g.weapons, &mut taken);
    Ok(found
        .into_iter()
        .map(|c| {
            let id = state.next_id.fetch_add(1, Ordering::Relaxed) + 1;
            let dto = item_dto(id, &c);
            queue.insert(id, c);
            dto
        })
        .collect())
}

/// A free short for `name`, ignoring the queue item `id` itself (so editing a name does not
/// collide with the name it already had).
#[tauri::command]
async fn suggest_short(
    id: Option<u64>,
    name: String,
    weapon: bool,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let game = state.game.lock().unwrap();
    let Some(g) = game.as_ref() else {
        return Ok(String::new());
    };
    let settings = state.settings.lock().unwrap();
    let mut taken = injector(g, &settings).taken_shorts();
    let queue = state.queue.lock().unwrap();
    taken.extend(
        queue
            .iter()
            .filter(|(k, _)| Some(**k) != id)
            .map(|(_, c)| c.short.clone()),
    );
    Ok(if weapon {
        batch::suggest_weapon_name(&name, &taken)
    } else {
        batch::suggest_short(&name, &taken)
    })
}

/// How the name will read in the game (the HUD font has no accents and a few reserved symbols).
#[tauri::command]
async fn hud_name(name: String) -> Result<String, String> {
    Ok(ide::hud_name(&name))
}

#[tauri::command]
async fn forget_items(ids: Vec<u64>, state: State<'_, AppState>) -> Result<(), String> {
    let mut queue = state.queue.lock().unwrap();
    for id in ids {
        queue.remove(&id);
    }
    Ok(())
}

/// What blocks each queued item (empty reference, a name already taken, repeated shorts).
#[tauri::command]
async fn validate(checks: Vec<Check>, state: State<'_, AppState>) -> Result<Vec<Problem>, String> {
    let game = state.game.lock().unwrap();
    let Some(g) = game.as_ref() else {
        return Ok(vec![]);
    };
    let settings = state.settings.lock().unwrap();
    let taken: BTreeSet<String> = injector(g, &settings).taken_shorts();
    let mut count: HashMap<&str, usize> = HashMap::new();
    for c in &checks {
        *count.entry(c.short.as_str()).or_default() += 1;
    }
    Ok(checks
        .iter()
        .map(|c| {
            let problem = if c.reference.is_empty() {
                Some("escolha a referência".to_string())
            } else if count[c.short.as_str()] > 1 {
                Some("nome curto repetido na fila".to_string())
            } else {
                let r = if c.weapon {
                    validators::check_weapon_name(&c.short, |s| taken.contains(s))
                } else {
                    validators::check_short(&c.short, |s| taken.contains(s))
                };
                r.err().map(|e| e.to_string())
            };
            Problem { id: c.id, problem }
        })
        .collect())
}

#[tauri::command]
async fn inject(
    requests: Vec<InjectRequest>,
    state: State<'_, AppState>,
) -> Result<Vec<InjectResult>, String> {
    let game = state.game.lock().unwrap();
    let g = game.as_ref().ok_or("abra a pasta do GTA SA primeiro")?;
    let settings = state.settings.lock().unwrap();
    let inj = injector(g, &settings);
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let created = meta::rfc3339(secs);
    let mut out = vec![];
    for r in requests {
        let Some(c) = state.queue.lock().unwrap().get(&r.id).cloned() else {
            out.push(InjectResult {
                id: r.id,
                ok: false,
                message: "item não está mais na fila".into(),
            });
            continue;
        };
        let (Some(dff), Some(txd)) = (c.files.dff.as_deref(), c.files.txd.as_deref()) else {
            out.push(InjectResult {
                id: r.id,
                ok: false,
                message: "faltam o .dff ou o .txd".into(),
            });
            continue;
        };
        let result = if r.weapon {
            inj.inject_weapon(&InjectWeapon {
                short: &r.short,
                dff,
                txd,
                reference: &r.reference,
                created: &created,
                own: c.files.weapon.as_ref().filter(|_| r.use_handling),
                special: Special::parse(r.special.as_deref().unwrap_or_default()),
            })
        } else {
            let traffic = match (r.traffic.kind.as_str(), r.traffic.index) {
                ("none", _) => Traffic::None,
                ("group", Some(i)) => Traffic::Group(i),
                _ => Traffic::Auto,
            };
            inj.inject_vehicle(&InjectVehicle {
                name: &r.name,
                short: &r.short,
                dff,
                txd,
                reference: &r.reference,
                traffic,
                audio: r.audio,
                created: &created,
                paintjobs: c.files.paintjobs.as_deref(),
                parts: c.files.parts.as_deref(),
                ivfc: c.files.ivfc.as_deref(),
                handling: c.files.handling.as_ref().filter(|_| r.use_handling),
                cleo: c.files.cleo.as_deref().filter(|_| r.use_cleo),
            })
        };
        match result {
            Ok(key) => {
                state.queue.lock().unwrap().remove(&r.id);
                out.push(InjectResult {
                    id: r.id,
                    ok: true,
                    message: key.display().to_string(),
                });
            }
            Err(e) => out.push(InjectResult {
                id: r.id,
                ok: false,
                message: e.to_string(),
            }),
        }
    }
    Ok(out)
}

/// Runs an operation of the injector on the opened game and returns its message.
fn with_injector<T>(
    state: &AppState,
    f: impl FnOnce(&Injector) -> awr_core::Result<T>,
) -> Result<T, String> {
    let game = state.game.lock().unwrap();
    let g = game.as_ref().ok_or("abra a pasta do GTA SA primeiro")?;
    let settings = state.settings.lock().unwrap();
    f(&injector(g, &settings)).map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_enabled(
    key: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<String, String> {
    with_injector(&state, |i| i.set_enabled(&PathBuf::from(&key), enabled))
        .map(|k| k.display().to_string())
}

#[tauri::command]
async fn remove(key: String, state: State<'_, AppState>) -> Result<String, String> {
    with_injector(&state, |i| i.remove(&PathBuf::from(&key))).map(|()| "removido".to_string())
}

#[tauri::command]
async fn repair(state: State<'_, AppState>) -> Result<String, String> {
    with_injector(&state, |i| i.repair())
        .map(|n| format!("reparado ({n} injeção(ões) interrompida(s) limpa(s))"))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            snapshot,
            startup,
            open_game,
            scan_paths,
            forget_items,
            suggest_short,
            hud_name,
            validate,
            inject,
            set_enabled,
            remove,
            repair
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o SA-AWR");
}
