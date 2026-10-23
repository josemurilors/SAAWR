//! SA-AWR: single-screen app over `awr-core` (spec §6). No business logic lives here.

use awr_core::{
    checker::{self, Finding, Severity},
    config::AppSettings,
    injector::{InjectVehicle, Injector},
    inventory::{self, InstalledMod},
    meta::{self, ModKind},
    pack,
    paths::{self, InstallInfo},
    references::{self, VehicleRef},
    validators,
};
use eframe::egui::{self, Color32, RichText};
use std::{
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
            .with_title("SA-AWR"),
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
}

#[derive(Default)]
struct Form {
    dff: Option<PathBuf>,
    txd: Option<PathBuf>,
    paintjobs: Option<PathBuf>,
    parts: Option<PathBuf>,
    ivfc: Option<PathBuf>,
    reference: String,
    ref_filter: String,
    name: String,
    short: String,
    short_edited: bool,
    no_traffic: bool,
    no_audio: bool,
}

struct App {
    settings: AppSettings,
    gta_text: String,
    game: Option<Game>,
    mods: Vec<InstalledMod>,
    findings: Vec<Finding>,
    form: Form,
    confirm_delete: Option<PathBuf>,
    log: Vec<String>,
}

impl App {
    fn new() -> Self {
        let settings = AppSettings::load();
        let mut app = App {
            gta_text: settings
                .gta_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            settings,
            game: None,
            mods: vec![],
            findings: vec![],
            form: Form::default(),
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
            Ok(Game { info, refs })
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
                .unwrap_or_default();
            self.findings = checker::check(&g.info);
        }
    }

    fn injector(&self) -> Option<Injector<'_>> {
        self.game.as_ref().map(|g| Injector {
            info: &g.info,
            refs: &g.refs,
            id_floor: self.settings.id_floor_vehicle,
        })
    }

    /// Dropped folder = a pack; dropped files = the `.dff` / `.txd` themselves.
    fn accept_drop(&mut self, path: PathBuf) {
        let f = &mut self.form;
        if path.is_dir() {
            let found = pack::discover(&path, &f.reference);
            f.dff = found.dff;
            f.txd = found.txd;
            f.paintjobs = found.paintjobs;
            f.parts = found.parts;
            f.ivfc = found.ivfc;
            if f.name.is_empty() {
                f.name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
            }
        } else {
            match path.extension().and_then(|e| e.to_str()) {
                Some(e) if e.eq_ignore_ascii_case("dff") => f.dff = Some(path),
                Some(e) if e.eq_ignore_ascii_case("txd") => f.txd = Some(path),
                _ => {}
            }
        }
        if !f.short_edited {
            f.short = validators::short_from(&f.name);
        }
    }

    fn inject(&mut self) {
        let f = &self.form;
        let (Some(dff), Some(txd)) = (f.dff.clone(), f.txd.clone()) else {
            return self.say("erro: falta o .dff ou o .txd");
        };
        let secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let result = self.injector().map(|inj| {
            inj.inject_vehicle(&InjectVehicle {
                name: &f.name,
                short: &f.short,
                dff: &dff,
                txd: &txd,
                reference: &f.reference,
                traffic: !f.no_traffic,
                audio: !f.no_audio,
                created: &meta::rfc3339(secs),
                paintjobs: f.paintjobs.as_deref(),
                parts: f.parts.as_deref(),
                ivfc: f.ivfc.as_deref(),
            })
        });
        match result {
            Some(Ok(key)) => {
                self.say(format!("injetado: {}", key.display()));
                self.form = Form::default();
            }
            Some(Err(e)) => self.say(format!("erro: {e}")),
            None => self.say("erro: abra a pasta do GTA SA primeiro"),
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
        ui.heading(RichText::new("Novo veículo").color(GOLD));
        let drop_text = match (&self.form.dff, &self.form.txd) {
            (Some(d), Some(t)) => format!("{}\n{}", file_name(d), file_name(t)),
            _ => "Arraste a pasta do mod (ou .dff + .txd) aqui".into(),
        };
        ui.group(|ui| {
            ui.set_min_height(64.0);
            ui.set_width(ui.available_width());
            ui.label(RichText::new(drop_text).color(MUTED));
            if ui.button("Procurar pasta...").clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    self.accept_drop(p);
                }
            }
        });
        let f = &self.form;
        let extras = [
            (f.parts.is_some(), "tuning"),
            (f.paintjobs.is_some(), "paintjobs"),
            (f.ivfc.is_some(), "cores IVF"),
        ]
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, n)| *n)
        .collect::<Vec<_>>();
        if !extras.is_empty() {
            ui.label(RichText::new(format!("Inclui: {}", extras.join(", "))).color(MUTED));
        }
        ui.add_space(8.0);

        if let Some(g) = &self.game {
            ui.label("Referência");
            ui.add(
                egui::TextEdit::singleline(&mut self.form.ref_filter)
                    .hint_text("filtrar (ex.: uranus)"),
            );
            let filter = self.form.ref_filter.to_ascii_lowercase();
            egui::ComboBox::from_id_salt("ref")
                .width(ui.available_width())
                .selected_text(if self.form.reference.is_empty() {
                    "escolha..."
                } else {
                    &self.form.reference
                })
                .show_ui(ui, |ui| {
                    for r in g.refs.iter().filter(|r| r.model.contains(&filter)) {
                        ui.selectable_value(
                            &mut self.form.reference,
                            r.model.clone(),
                            format!("{} ({})", r.model, r.kind),
                        );
                    }
                });
        }
        ui.add_space(8.0);
        ui.label("Nome");
        if ui.text_edit_singleline(&mut self.form.name).changed() && !self.form.short_edited {
            self.form.short = validators::short_from(&self.form.name);
        }
        ui.label("Nome curto (até 7)");
        if ui.text_edit_singleline(&mut self.form.short).changed() {
            self.form.short_edited = true;
        }
        ui.checkbox(&mut self.form.no_traffic, "Sem tráfego");
        ui.checkbox(&mut self.form.no_audio, "Sem áudio clonado");
        ui.add_space(12.0);
        let f = &self.form;
        let ready = self.game.is_some()
            && f.dff.is_some()
            && f.txd.is_some()
            && !f.reference.is_empty()
            && !f.short.is_empty()
            && !checker::has_errors(&self.findings);
        let btn = egui::Button::new(
            RichText::new("INJETAR NO JOGO")
                .strong()
                .color(Color32::BLACK),
        )
        .fill(if ready { GOLD } else { MUTED })
        .min_size(egui::vec2(ui.available_width(), 36.0));
        if ui.add_enabled(ready, btn).clicked() {
            self.inject();
        }
    }

    fn right(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Inventário").color(GOLD));
        if self.mods.is_empty() {
            ui.label(RichText::new("Nenhum mod instalado.").color(MUTED));
        }
        let mods = self.mods.clone();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for m in mods {
                let Some(meta) = m.meta.as_ref().filter(|x| x.kind == ModKind::Vehicle) else {
                    let id = m.id.map_or("-".into(), |i| i.to_string());
                    ui.label(
                        RichText::new(format!("{id:>6}  (terceiro)  {}", file_name(&m.key)))
                            .color(MUTED),
                    );
                    continue;
                };
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{:>6}", meta.id)).monospace());
                    let name = RichText::new(&meta.name);
                    ui.label(if m.enabled { name } else { name.color(MUTED) });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.confirm_delete.as_ref() == Some(&m.key) {
                            if ui.button("Cancelar").clicked() {
                                self.confirm_delete = None;
                            }
                            if ui.button(RichText::new("Confirmar").color(BAD)).clicked() {
                                self.confirm_delete = None;
                                let key = m.key.clone();
                                self.act(|i| i.remove(&key).map(|()| "removido".into()));
                            }
                        } else {
                            if ui.button("Excluir").clicked() {
                                self.confirm_delete = Some(m.key.clone());
                            }
                            let label = if m.enabled { "ON" } else { "OFF" };
                            if ui.button(RichText::new(label).color(GOLD)).clicked() {
                                let key = m.key.clone();
                                self.act(|i| {
                                    i.set_enabled(&key, !m.enabled)
                                        .map(|k| format!("{}", k.display()))
                                });
                            }
                        }
                    });
                });
            }
        });
    }

    fn footer(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("GTA SA:");
            let w = ui.available_width() - 150.0;
            ui.add(egui::TextEdit::singleline(&mut self.gta_text).desired_width(w));
            if ui.button("Abrir").clicked() {
                self.open_game();
            }
            if ui.button("...").clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    self.gta_text = p.display().to_string();
                    self.open_game();
                }
            }
        });
        if self.game.is_some() {
            if self.findings.is_empty() {
                ui.label(RichText::new("Instalação OK").color(GOLD));
            }
            for f in &self.findings {
                let (tag, c) = match f.severity {
                    Severity::Error => ("ERRO", BAD),
                    Severity::Warn => ("aviso", GOLD),
                };
                ui.label(RichText::new(format!("{tag}: {}", f.msg)).color(c));
            }
            if ui.small_button("Reparar instalação").clicked() {
                self.act(|i| i.repair().map(|n| format!("reparado ({n} limpas)")));
            }
        }
        if let Some(last) = self.log.last() {
            ui.label(RichText::new(last).color(MUTED));
        }
        egui::CollapsingHeader::new("Log").show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(120.0)
                .show(ui, |ui| {
                    for l in self.log.iter().rev() {
                        ui.label(RichText::new(l).small().color(MUTED));
                    }
                });
        });
    }
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let dropped: Vec<PathBuf> = ui.ctx().input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        for p in dropped {
            self.accept_drop(p);
        }
        egui::Panel::bottom("footer").show(ui, |ui| self.footer(ui));
        egui::Panel::left("form")
            .resizable(false)
            .exact_size(340.0)
            .show(ui, |ui| self.left(ui));
        egui::CentralPanel::default().show(ui, |ui| self.right(ui));
    }
}
