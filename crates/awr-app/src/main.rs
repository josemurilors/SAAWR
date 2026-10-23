//! SA-AWR: single-screen app over `awr-core` (spec §6). No business logic lives here.

use awr_core::{
    batch::{self, Candidate},
    cargrp::{self, Group},
    checker::{self, Finding, Severity},
    config::AppSettings,
    fsutil,
    injector::{InjectVehicle, Injector, Traffic},
    inventory::{self, InstalledMod},
    meta::{self, ModKind},
    pack::PackFiles,
    paths::{self, InstallInfo},
    references::{self, VehicleRef},
    validators,
    weapons::{self, InjectWeapon, WeaponRef},
};
use eframe::egui::{self, Color32, RichText};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::SystemTime,
};

const GOLD: Color32 = Color32::from_rgb(0xD4, 0xAF, 0x37);
const BAD: Color32 = Color32::from_rgb(0xE0, 0x5A, 0x4F);
const MUTED: Color32 = Color32::from_rgb(0x9A, 0x9A, 0x9A);

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([880.0, 560.0])
            .with_title("SA-AWR")
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "SA-AWR",
        options,
        Box::new(|cc| {
            theme(&cc.egui_ctx);
            Ok(Box::new(App::new()))
        }),
    )
}

fn theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = Color32::from_rgb(0x14, 0x14, 0x14);
    v.window_fill = Color32::from_rgb(0x1B, 0x1B, 0x1B);
    v.selection.bg_fill = GOLD.gamma_multiply(0.45);
    v.selection.stroke.color = GOLD;
    v.hyperlink_color = GOLD;
    v.widgets.hovered.bg_stroke.color = GOLD;
    v.widgets.active.bg_stroke.color = GOLD;
    // sharp corners everywhere
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = egui::CornerRadius::ZERO;
    }
    v.window_corner_radius = egui::CornerRadius::ZERO;
    v.menu_corner_radius = egui::CornerRadius::ZERO;
    ctx.set_visuals(v);
}

/// An opened GTA SA install plus the original vehicles it can clone.
struct Game {
    info: InstallInfo,
    refs: Vec<VehicleRef>,
    weapons: Vec<WeaponRef>,
    groups: Vec<Group>,
}

/// One vehicle waiting to be injected (a dropped pack, or a sub-folder of a batch).
struct Item {
    name: String,
    short: String,
    reference: String,
    files: PackFiles,
    traffic: Traffic,
    audio: bool,
    /// Use the handling the pack ships (when it has one) instead of copying the reference.
    use_handling: bool,
    /// Copy the pack's CLEO scripts too (off by default: they may not suit the new ID).
    use_cleo: bool,
    /// A weapon instead of a vehicle: `reference` is then a `weapon.dat` name.
    weapon: bool,
    include: bool,
}

impl Item {
    fn from(c: Candidate, weapon_guess: Option<String>) -> Self {
        let weapon = c.reference.is_none() && weapon_guess.is_some();
        Item {
            name: c.name,
            short: c.short,
            reference: c.reference.or(weapon_guess).unwrap_or_default(),
            weapon,
            files: c.files,
            traffic: Traffic::Auto,
            audio: true,
            use_handling: true,
            use_cleo: false,
            include: true,
        }
    }

    /// What blocks this item from being injected, if anything.
    fn problem(&self, taken: &BTreeSet<String>, dup: bool) -> Option<String> {
        if self.reference.is_empty() {
            return Some("escolha a referência".into());
        }
        if dup {
            return Some("nome curto repetido na fila".into());
        }
        let checked = if self.weapon {
            validators::check_weapon_name(&self.short, |s| taken.contains(s))
        } else {
            validators::check_short(&self.short, |s| taken.contains(s))
        };
        checked.err().map(|e| e.to_string())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Import,
    Installed,
}

struct App {
    tab: Tab,
    show_log: bool,
    settings: AppSettings,
    gta_text: String,
    game: Option<Game>,
    mods: Vec<Row>,
    /// Shorts that cannot be reused; refreshed with the inventory, not every frame.
    taken: BTreeSet<String>,
    filter_text: String,
    filter_kind: Option<Kind>,
    findings: Vec<Finding>,
    queue: Vec<Item>,
    ref_filter: String,
    confirm_delete: Option<PathBuf>,
    log: Vec<String>,
}

impl App {
    fn new() -> Self {
        let settings = AppSettings::load();
        let mut app = App {
            tab: Tab::Import,
            show_log: false,
            gta_text: settings
                .gta_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            settings,
            game: None,
            mods: vec![],
            taken: BTreeSet::new(),
            filter_text: String::new(),
            filter_kind: None,
            findings: vec![],
            queue: vec![],
            ref_filter: String::new(),
            confirm_delete: None,
            log: vec![],
        };
        if !app.gta_text.is_empty() {
            app.open_game();
        }
        app
    }

    fn say(&mut self, msg: impl Into<String>) {
        self.log.push(msg.into());
        if self.log.len() > 100 {
            self.log.remove(0);
        }
    }

    fn open_game(&mut self) {
        let root = PathBuf::from(self.gta_text.trim());
        let loaded = paths::detect_install(&root).and_then(|info| {
            let refs = references::load_vehicles(&info.data(), info.fla_data().as_deref())?;
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
        });
        match loaded {
            Ok(g) => {
                self.settings.gta_path = Some(root);
                if let Err(e) = self.settings.save() {
                    self.say(format!("não salvou a configuração: {e}"));
                }
                self.say(format!("GTA SA aberto: {}", g.info.root.display()));
                self.game = Some(g);
                self.refresh();
            }
            Err(e) => {
                self.game = None;
                self.mods.clear();
                self.findings.clear();
                self.say(format!("erro: {e}"));
            }
        }
    }

    fn refresh(&mut self) {
        if let Some(g) = &self.game {
            self.mods = g
                .info
                .modloader
                .as_deref()
                .map(inventory::scan)
                .unwrap_or_default()
                .into_iter()
                .map(Row::new)
                .collect();
            self.findings = checker::check(&g.info);
        }
        self.taken = self
            .injector()
            .map(|i| i.taken_shorts())
            .unwrap_or_default();
    }

    fn injector(&self) -> Option<Injector<'_>> {
        self.game.as_ref().map(|g| Injector {
            info: &g.info,
            refs: &g.refs,
            weapons: &g.weapons,
            id_floor: self.settings.id_floor_vehicle,
        })
    }

    /// Anything dropped or picked: a pack folder, a folder of packs, or loose `.dff`/`.txd`.
    /// Names come from the folder (or file) name and the reference from the `.dff` name.
    fn add_paths(&mut self, paths: Vec<PathBuf>) {
        let Some(inj) = self.injector() else {
            return self.say("erro: abra a pasta do GTA SA antes de adicionar veículos");
        };
        let refs = inj.refs;
        let weapons = inj.weapons;
        let mut taken = inj.taken_shorts();
        taken.extend(self.queue.iter().map(|i| i.short.clone()));
        let mut found: Vec<Candidate> = vec![];
        for p in paths.iter().filter(|p| p.is_dir()) {
            let c = batch::scan(p, refs, &taken);
            taken.extend(c.iter().map(|c| c.short.clone()));
            found.extend(c);
        }
        for dff in paths.iter().filter(|p| ext_is(p, "dff")) {
            let txd = paths
                .iter()
                .find(|p| ext_is(p, "txd") && p.file_stem() == dff.file_stem());
            let Some(txd) = txd else { continue };
            let name = dff
                .file_stem()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let files = PackFiles {
                dff: Some(dff.clone()),
                txd: Some(txd.clone()),
                ..Default::default()
            };
            let short = batch::suggest_short(&name, &taken);
            taken.insert(short.clone());
            found.push(Candidate {
                folder: dff.parent().map(Path::to_path_buf).unwrap_or_default(),
                reference: batch::guess_reference(&files, refs),
                name,
                short,
                files,
            });
        }
        let items: Vec<Item> = found
            .into_iter()
            .map(|c| {
                let guess = batch::guess_weapon(&c.files, weapons);
                Item::from(c, guess)
            })
            .collect();
        if items.is_empty() {
            self.say("nada reconhecido: precisa de uma pasta com .dff + .txd");
        } else {
            self.say(format!("{} item(ns) na fila", items.len()));
            self.queue.extend(items);
        }
    }

    fn inject_all(&mut self) {
        let secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let mut logs = vec![];
        let mut left = vec![];
        for item in std::mem::take(&mut self.queue) {
            let done = item.include && {
                let r = item.files.dff.as_deref().zip(item.files.txd.as_deref());
                match (r, self.injector()) {
                    (Some((dff, txd)), Some(inj)) if item.weapon => {
                        let res = inj.inject_weapon(&InjectWeapon {
                            short: &item.short,
                            dff,
                            txd,
                            reference: &item.reference,
                            created: &meta::rfc3339(secs),
                            own: item.files.weapon.as_ref().filter(|_| item.use_handling),
                        });
                        match &res {
                            Ok(k) => logs.push(format!("injetada: {}", k.display())),
                            Err(e) => logs.push(format!("erro em {}: {e}", item.name)),
                        }
                        res.is_ok()
                    }
                    (Some((dff, txd)), Some(inj)) => {
                        let res = inj.inject_vehicle(&InjectVehicle {
                            name: &item.name,
                            short: &item.short,
                            dff,
                            txd,
                            reference: &item.reference,
                            traffic: item.traffic,
                            audio: item.audio,
                            created: &meta::rfc3339(secs),
                            paintjobs: item.files.paintjobs.as_deref(),
                            parts: item.files.parts.as_deref(),
                            ivfc: item.files.ivfc.as_deref(),
                            handling: item.files.handling.as_ref().filter(|_| item.use_handling),
                            cleo: item.files.cleo.as_deref().filter(|_| item.use_cleo),
                        });
                        match &res {
                            Ok(k) => logs.push(format!("injetado: {}", k.display())),
                            Err(e) => logs.push(format!("erro em {}: {e}", item.name)),
                        }
                        res.is_ok()
                    }
                    _ => false,
                }
            };
            if !done {
                left.push(item);
            }
        }
        self.queue = left;
        for l in logs {
            self.say(l);
        }
        self.refresh();
    }

    fn act(&mut self, what: impl FnOnce(&Injector) -> awr_core::Result<String>) {
        let msg = match self.injector().map(|i| what(&i)) {
            Some(Ok(m)) => m,
            Some(Err(e)) => format!("erro: {e}"),
            None => return,
        };
        self.say(msg);
        self.refresh();
    }

    fn left(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Importar veículos").color(GOLD));
        ui.label(
            RichText::new("Arraste aqui uma pasta de mod, uma pasta com várias, ou .dff + .txd")
                .color(MUTED),
        );
        ui.horizontal(|ui| {
            if ui.button("Procurar pasta...").clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    self.add_paths(vec![p]);
                }
            }
            if ui.button("Procurar arquivos...").clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("modelo/textura", &["dff", "txd"])
                    .pick_files()
                {
                    self.add_paths(p);
                }
            }
            if !self.queue.is_empty() && ui.button("Limpar fila").clicked() {
                self.queue.clear();
            }
        });
        ui.separator();

        let taken = self.taken.clone();
        let mut shorts: BTreeMap<String, usize> = BTreeMap::new();
        for i in self.queue.iter().filter(|i| i.include) {
            *shorts.entry(i.short.clone()).or_default() += 1;
        }
        let App {
            queue,
            game,
            ref_filter,
            ..
        } = self;
        let mut ready = 0;
        let mut remove = None;
        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - 52.0)
            .show(ui, |ui| {
                for (n, item) in queue.iter_mut().enumerate() {
                    let dup = shorts.get(&item.short).is_some_and(|c| *c > 1);
                    let problem = item.problem(&taken, dup);
                    if item.include && problem.is_none() {
                        ready += 1;
                    }
                    let kind = if item.weapon {
                        Kind::Weapon
                    } else {
                        game.as_ref()
                            .and_then(|g| g.refs.iter().find(|r| r.model == item.reference))
                            .map_or(Kind::Car, |r| Kind::of(&r.kind))
                    };
                    card(ui, !item.include, |ui| {
                        ui.horizontal(|ui| {
                            icon(ui, kind, if item.include { GOLD } else { MUTED });
                            ui.checkbox(&mut item.include, "");
                            ui.add(egui::TextEdit::singleline(&mut item.name).desired_width(200.0));
                            ui.add(
                                egui::TextEdit::singleline(&mut item.short)
                                    .desired_width(70.0)
                                    .hint_text("curto"),
                            );
                            if ui.small_button("x").clicked() {
                                remove = Some(n);
                            }
                        });
                        if let Some(g) = game.as_ref() {
                            ui.horizontal(|ui| {
                                let a = ui.selectable_value(&mut item.weapon, false, "Veículo");
                                let b = ui.selectable_value(&mut item.weapon, true, "Arma");
                                if a.changed() || b.changed() {
                                    item.reference.clear();
                                }
                                ui.label("Ref:");
                                if item.weapon {
                                    weapon_combo(
                                        ui,
                                        n,
                                        &g.weapons,
                                        &mut item.reference,
                                        ref_filter,
                                    );
                                } else {
                                    ref_combo(ui, n, &g.refs, &mut item.reference, ref_filter);
                                    ui.label("Tráfego:");
                                    traffic_combo(ui, n, &g.groups, &mut item.traffic);
                                }
                            });
                        }
                        if item.weapon && item.files.weapon.is_some() {
                            ui.checkbox(
                                &mut item.use_handling,
                                "Usar as linhas de arma do pacote (readme)",
                            );
                        }
                        if !item.weapon && item.files.cleo.is_some() {
                            ui.checkbox(
                                &mut item.use_cleo,
                                "Copiar os scripts CLEO do pacote (pode não servir para o novo ID)",
                            );
                        }
                        if !item.weapon && item.files.handling.is_some() {
                            ui.checkbox(
                                &mut item.use_handling,
                                "Usar o handling do pacote (readme)",
                            );
                        }
                        let extras: Vec<&str> = [
                            (item.files.parts.is_some(), "tuning"),
                            (item.files.paintjobs.is_some(), "paintjobs"),
                            (item.files.ivfc.is_some(), "cores IVF"),
                        ]
                        .iter()
                        .filter(|(on, _)| *on)
                        .map(|(_, n)| *n)
                        .collect();
                        let info = if item.weapon {
                            "arma: modelo e textura (ícone do HUD renomeado)".to_string()
                        } else if extras.is_empty() {
                            "só modelo e textura".to_string()
                        } else {
                            format!("inclui {}", extras.join(", "))
                        };
                        match (item.include, &problem) {
                            (true, Some(p)) => ui.label(RichText::new(p).color(BAD)),
                            _ => ui.label(RichText::new(info).small().color(MUTED)),
                        };
                    });
                    ui.add_space(6.0);
                }
            });
        if let Some(n) = remove {
            self.queue.remove(n);
        }
        let blocked = checker::has_errors(&self.findings) || self.game.is_none();
        let label = format!("INJETAR NO JOGO ({ready})");
        let on = ready > 0 && !blocked;
        let btn = egui::Button::new(RichText::new(label).strong().color(Color32::BLACK))
            .fill(if on { GOLD } else { MUTED })
            .min_size(egui::vec2(ui.available_width(), 36.0));
        if ui.add_enabled(on, btn).clicked() {
            self.inject_all();
        }
    }

    fn right(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(RichText::new("Instalados").color(GOLD));
            ui.add(
                egui::TextEdit::singleline(&mut self.filter_text)
                    .hint_text("filtrar por nome")
                    .desired_width(180.0),
            );
            ui.selectable_value(&mut self.filter_kind, None, "Todos");
            for k in Kind::ALL {
                ui.selectable_value(&mut self.filter_kind, Some(k), k.label());
            }
        });
        ui.separator();
        let text = self.filter_text.to_lowercase();
        let rows: Vec<Row> = self
            .mods
            .iter()
            .filter(|r| {
                let kind_ok = self.filter_kind.is_none_or(|k| Some(k) == r.kind());
                let text_ok = text.is_empty() || r.title().to_lowercase().contains(&text);
                kind_ok && text_ok
            })
            .cloned()
            .collect();
        if rows.is_empty() {
            ui.label(RichText::new("Nenhum mod para mostrar.").color(MUTED));
        }
        let groups: Vec<Group> = self
            .game
            .as_ref()
            .map(|g| g.groups.clone())
            .unwrap_or_default();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for row in rows {
                self.installed_card(ui, &row, &groups);
                ui.add_space(6.0);
            }
        });
    }

    fn installed_card(&mut self, ui: &mut egui::Ui, row: &Row, groups: &[Group]) {
        let m = &row.m;
        let Some(meta) = m.meta.as_ref() else {
            card(ui, true, |ui| {
                ui.horizontal(|ui| {
                    icon(ui, Kind::Car, MUTED);
                    ui.label(RichText::new(file_name(&m.key)).color(MUTED));
                    ui.label(
                        RichText::new("terceiro: não é alterado")
                            .small()
                            .color(MUTED),
                    );
                });
            });
            return;
        };
        card(ui, !m.enabled, |ui| {
            let c = if m.enabled { GOLD } else { MUTED };
            ui.horizontal(|ui| {
                icon(ui, row.kind().unwrap_or(Kind::Car), c);
                ui.vertical(|ui| {
                    let name = RichText::new(&meta.name).strong();
                    ui.label(if m.enabled { name } else { name.color(MUTED) });
                    ui.label(RichText::new(row.details(groups)).small().color(MUTED));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(meta.id.to_string()).monospace().color(MUTED));
                    let label = if m.enabled { "ON" } else { "OFF" };
                    let fill = if m.enabled {
                        GOLD
                    } else {
                        Color32::from_gray(0x33)
                    };
                    let txt = RichText::new(label).strong().color(if m.enabled {
                        Color32::BLACK
                    } else {
                        MUTED
                    });
                    if ui.add(egui::Button::new(txt).fill(fill)).clicked() {
                        let key = m.key.clone();
                        self.act(|i| {
                            i.set_enabled(&key, !m.enabled)
                                .map(|k| format!("{}", k.display()))
                        });
                    }
                });
            });
            ui.horizontal(|ui| {
                if self.confirm_delete.as_ref() == Some(&m.key) {
                    if ui
                        .button(RichText::new("Confirmar exclusão").color(BAD))
                        .clicked()
                    {
                        self.confirm_delete = None;
                        let key = m.key.clone();
                        self.act(|i| i.remove(&key).map(|()| "removido".into()));
                    }
                    if ui.button("Cancelar").clicked() {
                        self.confirm_delete = None;
                    }
                } else if ui.small_button("Excluir").clicked() {
                    self.confirm_delete = Some(m.key.clone());
                }
            });
        });
    }

    /// Title, the two tabs, and the GTA SA menu (path, health, repair).
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("SA-AWR").heading().strong().color(GOLD));
            ui.add_space(16.0);
            let installed = format!("Instalados ({})", self.mods.len());
            ui.selectable_value(&mut self.tab, Tab::Import, "Importar");
            ui.selectable_value(&mut self.tab, Tab::Installed, installed);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.game_menu(ui);
            });
        });
    }

    fn game_menu(&mut self, ui: &mut egui::Ui) {
        let errors = checker::has_errors(&self.findings);
        let (dot, color) = match (self.game.is_some(), errors, self.findings.is_empty()) {
            (false, _, _) | (_, true, _) => ("●", BAD),
            (_, _, false) => ("●", GOLD),
            _ => ("●", Color32::from_rgb(0x6B, 0xBF, 0x59)),
        };
        ui.menu_button(RichText::new(format!("{dot} GTA SA")).color(color), |ui| {
            ui.set_min_width(420.0);
            ui.label("Pasta do jogo");
            ui.horizontal(|ui| {
                let w = ui.available_width() - 110.0;
                ui.add(egui::TextEdit::singleline(&mut self.gta_text).desired_width(w));
                if ui.button("...").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                        self.gta_text = p.display().to_string();
                        self.open_game();
                    }
                }
                if ui.button("Abrir").clicked() {
                    self.open_game();
                }
            });
            if let Some(g) = &self.game {
                ui.separator();
                for (name, ok) in [
                    ("ModLoader", g.info.modloader.is_some()),
                    ("fastman92 (FLA)", g.info.fla_ini().is_some()),
                    ("Open Limit Adjuster", g.info.ola_asi.is_some()),
                ] {
                    let (txt, c) = if ok { ("ok", GOLD) } else { ("falta", BAD) };
                    ui.label(RichText::new(format!("{name}: {txt}")).color(c));
                }
                for f in &self.findings {
                    let (tag, c) = match f.severity {
                        Severity::Error => ("ERRO", BAD),
                        Severity::Warn => ("aviso", GOLD),
                    };
                    ui.label(RichText::new(format!("{tag}: {}", f.msg)).color(c));
                }
                ui.separator();
                if ui.button("Reparar instalação").clicked() {
                    self.act(|i| i.repair().map(|n| format!("reparado ({n} limpas)")));
                }
            }
        });
    }

    /// One thin line: what happened last, and a button for the full log.
    fn footer(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let last = self.log.last().map_or("pronto", String::as_str);
            let c = if last.starts_with("erro") { BAD } else { MUTED };
            ui.label(RichText::new(last).small().color(c));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Log").clicked() {
                    self.show_log = !self.show_log;
                }
            });
        });
    }
}

/// A row of the inventory: the mod plus what its folder holds.
#[derive(Clone)]
struct Row {
    m: InstalledMod,
    paintjobs: usize,
    ivfc: bool,
}

impl Row {
    fn new(m: InstalledMod) -> Self {
        let short = m.meta.as_ref().map(|x| x.short.clone()).unwrap_or_default();
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
        let ivfc = m.key.join("ImVehFt").is_dir();
        Row { m, paintjobs, ivfc }
    }

    fn title(&self) -> String {
        self.m
            .meta
            .as_ref()
            .map_or_else(|| file_name(&self.m.key), |x| x.name.clone())
    }

    fn kind(&self) -> Option<Kind> {
        let m = self.m.meta.as_ref()?;
        if m.kind == ModKind::Weapon {
            return Some(Kind::Weapon);
        }
        Some(Kind::of(m.vehicle_type.as_deref()?))
    }

    /// `ref: uranus · tuning (14) · paintjobs (4) · cores IVF · tráfego: ...`
    fn details(&self, groups: &[Group]) -> String {
        let Some(m) = &self.m.meta else {
            return String::new();
        };
        if m.kind == ModKind::Weapon {
            return format!(
                "arma · tipo {} (use este número em trainers e scripts) · modelo {} · ref: {}",
                m.weapon_type_id.unwrap_or_default(),
                m.id,
                m.reference
            );
        }
        let mut v = vec![format!("ref: {}", m.reference)];
        if !m.parts.is_empty() {
            v.push(format!("tuning ({})", m.parts.len()));
        }
        if self.paintjobs > 0 {
            v.push(format!("paintjobs ({})", self.paintjobs));
        }
        if self.ivfc {
            v.push("cores IVF".into());
        }
        if m.pack_handling {
            v.push("handling do pacote".into());
        }
        v.push(match m.traffic_group.as_deref() {
            None => "sem tráfego".into(),
            Some("auto") => "tráfego como a referência".into(),
            Some(g) => {
                let i: Option<usize> = g.strip_prefix("group:").and_then(|i| i.parse().ok());
                let label = i
                    .and_then(|i| groups.get(i))
                    .map_or(g.to_string(), group_label);
                format!("tráfego: {label}")
            }
        });
        v.join(" · ")
    }
}

/// The four vehicle families the UI tells apart.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Car,
    Bike,
    Boat,
    Plane,
    Weapon,
}

impl Kind {
    const ALL: [Kind; 5] = [Kind::Car, Kind::Bike, Kind::Boat, Kind::Plane, Kind::Weapon];

    /// From the IDE type column (car, bike, bmx, boat, plane, heli, quad, ...).
    fn of(t: &str) -> Kind {
        match t {
            "bike" | "bmx" => Kind::Bike,
            "boat" => Kind::Boat,
            "plane" | "heli" => Kind::Plane,
            _ => Kind::Car,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Kind::Car => "Carros",
            Kind::Bike => "Motos",
            Kind::Boat => "Barcos",
            Kind::Plane => "Aviões",
            Kind::Weapon => "Armas",
        }
    }
}

/// Small line icon per family, drawn with the painter (no image files).
fn icon(ui: &mut egui::Ui, kind: Kind, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    let stroke = egui::Stroke::new(1.8, color);
    let at = |x: f32, y: f32| rect.min + egui::vec2(x, y);
    let line = |pts: &[(f32, f32)]| {
        p.add(egui::Shape::line(
            pts.iter().map(|&(x, y)| at(x, y)).collect(),
            stroke,
        ));
    };
    match kind {
        Kind::Car => {
            line(&[
                (3.0, 22.0),
                (3.0, 16.0),
                (9.0, 15.0),
                (13.0, 9.0),
                (23.0, 9.0),
                (27.0, 15.0),
                (31.0, 16.0),
                (31.0, 22.0),
                (3.0, 22.0),
            ]);
            p.circle_stroke(at(10.0, 23.0), 4.0, stroke);
            p.circle_stroke(at(24.0, 23.0), 4.0, stroke);
        }
        Kind::Bike => {
            p.circle_stroke(at(8.0, 23.0), 6.0, stroke);
            p.circle_stroke(at(26.0, 23.0), 6.0, stroke);
            line(&[(8.0, 23.0), (15.0, 14.0), (22.0, 14.0), (26.0, 23.0)]);
            line(&[(22.0, 14.0), (24.0, 9.0), (28.0, 9.0)]);
        }
        Kind::Boat => {
            line(&[
                (3.0, 20.0),
                (31.0, 20.0),
                (26.0, 28.0),
                (8.0, 28.0),
                (3.0, 20.0),
            ]);
            line(&[(11.0, 20.0), (13.0, 13.0), (21.0, 13.0), (23.0, 20.0)]);
            line(&[(17.0, 13.0), (17.0, 6.0)]);
        }
        Kind::Plane => {
            line(&[(4.0, 17.0), (30.0, 17.0)]);
            line(&[(14.0, 17.0), (19.0, 4.0), (23.0, 4.0), (21.0, 17.0)]);
            line(&[(14.0, 17.0), (19.0, 30.0), (23.0, 30.0), (21.0, 17.0)]);
            line(&[(4.0, 17.0), (2.0, 12.0)]);
        }
        Kind::Weapon => {
            line(&[
                (3.0, 12.0),
                (24.0, 12.0),
                (24.0, 17.0),
                (16.0, 17.0),
                (14.0, 28.0),
                (9.0, 28.0),
                (10.0, 17.0),
                (3.0, 17.0),
                (3.0, 12.0),
            ]);
            line(&[(24.0, 14.0), (31.0, 14.0)]);
        }
    }
}

/// Dark card with a thin border; `dim` fades it for disabled or skipped entries.
fn card(ui: &mut egui::Ui, dim: bool, add: impl FnOnce(&mut egui::Ui)) {
    let border = if dim { 0x22 } else { 0x2A };
    egui::Frame::new()
        .fill(Color32::from_rgb(0x1B, 0x1B, 0x1B))
        .stroke(egui::Stroke::new(1.0, Color32::from_gray(border)))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}

/// Reference picker with a filter box at the top of the list.
fn ref_combo(
    ui: &mut egui::Ui,
    n: usize,
    refs: &[VehicleRef],
    selected: &mut String,
    filter: &mut String,
) {
    egui::ComboBox::from_id_salt(("ref", n))
        .width(120.0)
        .selected_text(if selected.is_empty() {
            "escolha..."
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            ui.add(egui::TextEdit::singleline(filter).hint_text("filtrar"));
            let f = filter.to_ascii_lowercase();
            for r in refs.iter().filter(|r| r.model.contains(&f)) {
                ui.selectable_value(
                    selected,
                    r.model.clone(),
                    format!("{} ({})", r.model, r.kind),
                );
            }
        });
}

/// Weapon picker (original weapons by their `weapon.dat` name).
fn weapon_combo(
    ui: &mut egui::Ui,
    n: usize,
    weapons: &[WeaponRef],
    selected: &mut String,
    filter: &mut String,
) {
    egui::ComboBox::from_id_salt(("weapon", n))
        .width(150.0)
        .selected_text(if selected.is_empty() {
            "escolha..."
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            ui.add(egui::TextEdit::singleline(filter).hint_text("filtrar"));
            let f = filter.to_ascii_lowercase();
            for w in weapons
                .iter()
                .filter(|w| w.name.to_ascii_lowercase().contains(&f))
            {
                ui.selectable_value(selected, w.name.clone(), &w.name);
            }
        });
}

fn traffic_combo(ui: &mut egui::Ui, n: usize, groups: &[Group], traffic: &mut Traffic) {
    let text = match *traffic {
        Traffic::Auto => "como a referência".to_string(),
        Traffic::None => "nenhum".to_string(),
        Traffic::Group(i) => groups.get(i).map_or(format!("grupo {i}"), group_label),
    };
    egui::ComboBox::from_id_salt(("traffic", n))
        .width(190.0)
        .selected_text(text)
        .show_ui(ui, |ui| {
            ui.selectable_value(traffic, Traffic::Auto, "como a referência");
            ui.selectable_value(traffic, Traffic::None, "nenhum");
            ui.separator();
            for g in groups {
                ui.selectable_value(traffic, Traffic::Group(g.index), group_label(g));
            }
        });
}

fn group_label(g: &Group) -> String {
    let l = g.label.strip_prefix("POPCYCLE_GROUP_").unwrap_or(&g.label);
    format!("{} - {}", g.index, l.replace('_', " ").to_lowercase())
}

fn ext_is(p: &Path, ext: &str) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let (dropped, hovering) = ui.ctx().input(|i| {
            let files = i
                .raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect::<Vec<_>>();
            (files, !i.raw.hovered_files.is_empty())
        });
        if !dropped.is_empty() {
            self.tab = Tab::Import;
            self.add_paths(dropped);
        }
        egui::Panel::top("bar").show(ui, |ui| self.top_bar(ui));
        egui::Panel::bottom("footer").show(ui, |ui| self.footer(ui));
        egui::CentralPanel::default().show(ui, |ui| match self.tab {
            Tab::Import => self.left(ui),
            Tab::Installed => self.right(ui),
        });
        let mut open = self.show_log;
        egui::Window::new("Log")
            .open(&mut open)
            .default_size([520.0, 260.0])
            .show(ui.ctx(), |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for l in self.log.iter().rev() {
                        ui.label(RichText::new(l).small().color(MUTED));
                    }
                });
            });
        self.show_log = open;
        if hovering {
            let rect = ui.max_rect();
            let p = ui.painter();
            p.rect_filled(
                rect,
                egui::CornerRadius::ZERO,
                Color32::from_black_alpha(190),
            );
            p.rect_stroke(
                rect.shrink(12.0),
                egui::CornerRadius::ZERO,
                egui::Stroke::new(2.0, GOLD),
                egui::StrokeKind::Inside,
            );
            p.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Solte para adicionar à fila",
                egui::FontId::proportional(26.0),
                GOLD,
            );
        }
    }
}
