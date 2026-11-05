//! New CJ clothes, hair and accessories. Each item is a folder in `awr-roupas` with its `.txd`
//! (and maybe a `.dff`) in a `player.img` folder, which ModLoader streams as new clothing, plus
//! the `.fxt` with its menu name. Selling and wearing them is the job of the CLEO catalog
//! ([`crate::catalog`]); nothing goes into the game's `shopping.dat`.

use crate::catalog;
use crate::error::{AwrError, Result};
use crate::injector::{io_at, Injector, STAGING};
use crate::inventory::{self, ModKey, CLOTHES_DIR};
use crate::meta::{self, Clothes, Files, Meta, ModKind};
use crate::stores::{self, Original};
use crate::{fsutil, ide, validators};
use std::{fs, path::Path, path::PathBuf};

/// ModLoader streams files in a folder with this name as new clothing.
const IMG: &str = "player.img";

pub struct InjectClothes<'a> {
    /// Menu name in the shop.
    pub name: &'a str,
    /// Texture (and own model) name.
    pub short: &'a str,
    pub txd: &'a Path,
    /// Own model; without it `model` names an original clothing model.
    pub dff: Option<&'a Path>,
    pub model: Option<&'a str>,
    pub body_part: u8,
    pub price: u32,
    /// Shop sections that sell it; none: only the wardrobe (cheat ROUPA), free.
    pub shops: &'a [String],
    pub created: &'a str,
}

/// A shop of the catalog, for the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopInfo {
    pub section: &'static str,
    pub label: &'static str,
    /// `clothes` or `haircuts` (barbers).
    pub kind: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreInfo {
    pub shops: Vec<ShopInfo>,
    /// `(model, body part)` of the original clothing models, to wear a new texture.
    pub models: Vec<(String, u8)>,
    /// CLEO (and its INI plugin) is installed: the catalog needs it.
    pub cleo: bool,
}

/// How a new clothing texture compares with the clothes of the CJ in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// Same size, or no original texture to compare with: copied as it is.
    Keep,
    /// Different size, same proportions: scaled to `want` on injection.
    Resize { want: (u16, u16) },
    /// Different size, but compressed or another proportion: copied as it is (it may show as
    /// skin in the game).
    Mismatch { want: (u16, u16) },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureFit {
    pub have: Option<(u16, u16)>,
    pub fit: Fit,
}

/// Lowest free `AWRR000` key (GXT keys are 7 characters at most).
fn next_gxt(installed: &[Meta]) -> Result<String> {
    (1..1000)
        .map(|n| format!("AWRR{n:03}"))
        .find(|k| {
            !installed
                .iter()
                .any(|m| m.clothes.as_ref().is_some_and(|c| &c.gxt == k))
        })
        .ok_or_else(|| AwrError::InvalidName("sem chaves GXT livres para roupas".into()))
}

impl Injector<'_> {
    /// The game's own clothes and haircuts (what the suggestions and the originals check use).
    pub fn clothes_db(&self) -> Vec<Original> {
        stores::load(&self.info.data())
    }

    fn require_cleo(&self) -> Result<()> {
        if self.info.cleo_ok() {
            return Ok(());
        }
        Err(AwrError::Unhealthy(
            "CLEO 4 com o plugin IniFiles (CLEO/IniFiles.cleo) não foi encontrado; as roupas novas precisam dele"
                .into(),
        ))
    }

    /// An original clothing file as the game loads it: a loose override in a ModLoader
    /// `player.img` folder, then a whole `player.img` in ModLoader (Proper Player Retex), then
    /// the game's `models/player.img`.
    fn active_clothing(&self, file: &str) -> Option<Vec<u8>> {
        fn walk(
            dir: &Path,
            depth: u8,
            file: &str,
            loose: &mut Vec<PathBuf>,
            imgs: &mut Vec<PathBuf>,
        ) {
            let Ok(rd) = fs::read_dir(dir) else { return };
            let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
            entries.sort();
            for p in entries {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                if name.starts_with('.') {
                    continue;
                }
                if name == IMG && p.is_file() {
                    imgs.push(p);
                } else if p.is_dir() {
                    if name == IMG {
                        loose.extend(
                            fs::read_dir(&p)
                                .into_iter()
                                .flatten()
                                .flatten()
                                .map(|e| e.path())
                                .filter(|f| {
                                    f.file_name().is_some_and(|n| {
                                        n.to_string_lossy().eq_ignore_ascii_case(file)
                                    })
                                }),
                        );
                    } else if depth > 0 {
                        walk(&p, depth - 1, file, loose, imgs);
                    }
                }
            }
        }
        let (mut loose, mut imgs) = (vec![], vec![]);
        if let Some(ml) = &self.info.modloader {
            walk(ml, 5, file, &mut loose, &mut imgs);
        }
        if let Some(b) = loose.iter().find_map(|p| fs::read(p).ok()) {
            return Some(b);
        }
        imgs.push(self.info.root.join("models").join(IMG));
        imgs.iter().find_map(|p| crate::img::read_entry(p, file))
    }

    /// Compares a new texture with an original one worn on the same model (or, for an own
    /// model, of the same body part) in the clothes the game will actually load.
    /// `file` is the new texture's file name: packs replace an original texture of the same
    /// name, which is the best match (original sizes differ even within one body part).
    pub fn texture_fit(
        &self,
        txd: &[u8],
        file: Option<&str>,
        model: Option<&str>,
        body_part: u8,
    ) -> TextureFit {
        let have = crate::txd::size(txd);
        let stem = file.map(|f| f.trim_end_matches(".txd").to_lowercase());
        let db = self.clothes_db();
        let same_part = db.iter().filter(|o| o.part == body_part);
        let same_name = same_part
            .clone()
            .filter(|o| Some(&o.texture) == stem.as_ref());
        let same_model = same_part
            .clone()
            .filter(|o| Some(o.model.as_str()) == model);
        let want = same_name
            .chain(same_model)
            .chain(same_part)
            .take(6)
            .find_map(|o| crate::txd::size(&self.active_clothing(&format!("{}.txd", o.texture))?));
        let fit = match (have, want) {
            (Some((w, h)), Some(want)) if (w, h) != want => {
                let same_shape =
                    u32::from(w) * u32::from(want.1) == u32::from(h) * u32::from(want.0);
                if same_shape && crate::txd::resizable(txd) {
                    Fit::Resize { want }
                } else {
                    Fit::Mismatch { want }
                }
            }
            _ => Fit::Keep,
        };
        TextureFit { have, fit }
    }

    fn enabled_clothes(&self) -> Result<Vec<Meta>> {
        Ok(inventory::scan(&self.modloader()?)
            .into_iter()
            .filter(|m| m.enabled)
            .filter_map(|m| m.meta)
            .filter(|m| m.clothes.is_some())
            .collect())
    }

    /// The shops, the original models and whether CLEO is there.
    pub fn store_info(&self) -> StoreInfo {
        StoreInfo {
            shops: stores::SHOPS
                .iter()
                .map(|s| ShopInfo {
                    section: s.section,
                    label: s.label,
                    kind: s.kind,
                })
                .collect(),
            models: stores::models(&self.clothes_db()),
            cleo: self.info.cleo_ok(),
        }
    }

    /// The suggestion for a new item from the names of its files (see [`stores::suggest`]).
    pub fn suggest_clothes(&self, txd: &Path, dff: Option<&Path>) -> Option<stores::Suggestion> {
        let stem = |p: &Path| p.file_stem().map(|n| n.to_string_lossy().into_owned());
        stores::suggest(
            &self.clothes_db(),
            &stem(txd)?,
            dff.and_then(stem).as_deref(),
        )
    }

    /// Checks the body part and that every shop exists and sells that kind of item.
    fn check_store(&self, c: &Clothes) -> Result<()> {
        if !stores::BODY_PARTS.contains(&c.body_part) {
            return Err(AwrError::InvalidName(format!(
                "parte do corpo inválida: {}",
                c.body_part
            )));
        }
        for name in &c.shops {
            if !stores::shop_sells(name, c.body_part) {
                return Err(AwrError::InvalidName(format!(
                    "a loja {name} não existe ou não vende {}",
                    if c.body_part == stores::HAIR {
                        "cortes de cabelo"
                    } else {
                        "roupas"
                    }
                )));
            }
        }
        Ok(())
    }

    pub fn inject_clothes(&self, req: &InjectClothes) -> Result<ModKey> {
        self.not_running()?;
        self.require_cleo()?;
        let ml = self.modloader()?;
        let originals = self.clothes_db();
        let taken = self.taken_shorts();
        validators::check_short(req.short, |s| {
            taken.contains(s) || originals.iter().any(|o| o.texture == s || o.model == s)
        })?;
        let model = match (req.dff, req.model) {
            (Some(_), _) => req.short.to_string(),
            (None, Some(m)) => {
                let m = m.to_lowercase();
                if !originals
                    .iter()
                    .any(|o| o.model == m && o.part == req.body_part)
                {
                    return Err(AwrError::InvalidName(format!(
                        "'{m}' não é um modelo original dessa parte do corpo"
                    )));
                }
                m
            }
            (None, None) => {
                return Err(AwrError::InvalidName(
                    "sem .dff, escolha um modelo original para a textura".into(),
                ))
            }
        };
        let installed: Vec<Meta> = inventory::scan(&ml)
            .into_iter()
            .filter_map(|m| m.meta)
            .collect();
        let clothes = Clothes {
            texture: req.short.into(),
            model,
            body_part: req.body_part,
            price: req.price,
            shops: req.shops.to_vec(),
            gxt: next_gxt(&installed)?,
        };
        self.check_store(&clothes)?;

        let dest = ml.join(CLOTHES_DIR).join(req.short);
        if dest.exists() {
            return Err(AwrError::AlreadyExists { path: dest });
        }
        let name = match req.name.trim() {
            "" => req.short,
            n => n,
        };
        let m = Meta {
            v: 1,
            kind: ModKind::Clothes,
            id: 0,
            weapon_type_id: None,
            name: name.into(),
            short: req.short.into(),
            vehicle_type: None,
            reference: clothes.model.clone(),
            traffic_group: None,
            audio: None,
            created: req.created.into(),
            files: Files {
                dff: req
                    .dff
                    .map(|_| format!("{IMG}/{}.dff", req.short))
                    .unwrap_or_default(),
                txd: format!("{IMG}/{}.txd", req.short),
                fxt: Some(format!("{}.fxt", req.short)),
            },
            parts: vec![],
            pack_handling: false,
            weapon_config: vec![],
            shop: None,
            pickups: vec![],
            clothes: Some(clothes),
        };

        let stage = ml.join(STAGING).join(CLOTHES_DIR).join(req.short);
        io_at(&stage, fs::create_dir_all(stage.join(IMG)))?;
        // scaled to the clothes of the CJ in use (game or Retex); the source file is untouched
        let txd = fsutil::read(req.txd)?;
        let model = (req.dff.is_none()).then_some(m.reference.as_str());
        let file = req
            .txd
            .file_name()
            .map(|n| n.to_string_lossy().into_owned());
        let txd = match self
            .texture_fit(&txd, file.as_deref(), model, req.body_part)
            .fit
        {
            Fit::Resize { want } => crate::txd::resize(&txd, want.0, want.1).unwrap_or(txd),
            _ => txd,
        };
        fsutil::write_atomic(&stage.join(&m.files.txd), &txd)?;
        if let Some(dff) = req.dff {
            io_at(dff, fs::copy(dff, stage.join(&m.files.dff)))?;
        }
        write_fxt(&stage, &m)?;
        meta::write(&stage, &m)?;

        io_at(&dest, fs::create_dir_all(dest.parent().unwrap_or(&ml)))?;
        io_at(&dest, fs::rename(&stage, &dest))?;
        let _ = fs::remove_dir(stage.parent().unwrap_or(&ml));
        let _ = fs::remove_dir(ml.join(STAGING));
        self.sync_fla()?;
        self.sync_addons()?;
        Ok(dest)
    }

    /// New name, price and shops of an installed item.
    pub(crate) fn edit_clothes(
        &self,
        key: &ModKey,
        m: &mut Meta,
        price: u32,
        shops: &[String],
    ) -> Result<()> {
        let Some(c) = m.clothes.as_mut() else {
            return Ok(());
        };
        c.price = price;
        c.shops = shops.to_vec();
        let c = c.clone();
        self.check_store(&c)?;
        write_fxt(key, m)
    }

    /// Writes (or takes out) the CLEO catalog: script and texts in ModLoader, the item list in
    /// the game's `CLEO\AWR-Roupas`. Gone when no clothing item is on; the player's purchases
    /// (`estado.ini`) stay.
    pub(crate) fn sync_catalog(&self) -> Result<()> {
        let Ok(ml) = self.modloader() else {
            return Ok(());
        };
        let dir = ml.join(catalog::CATALOG_DIR);
        let data = self.info.root.join(catalog::DATA_DIR);
        let metas = self.enabled_clothes()?;
        if metas.is_empty() {
            let _ = fs::remove_dir_all(&dir);
            let _ = fs::remove_file(data.join(catalog::CONFIG_FILE));
            let _ = fs::remove_dir(&data); // only when empty
            return Ok(());
        }
        let items: Vec<&Clothes> = metas.iter().filter_map(|m| m.clothes.as_ref()).collect();
        write_if_changed(
            &dir.join("cleo").join(catalog::SCRIPT_FILE),
            catalog::SCRIPT,
        )?;
        write_if_changed(&dir.join(catalog::UI_FXT_FILE), catalog::UI_FXT)?;
        write_if_changed(
            &data.join(catalog::CONFIG_FILE),
            catalog::config_ini(&items).as_bytes(),
        )
    }
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<()> {
    if fs::read(path).ok().as_deref() == Some(bytes) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        io_at(dir, fs::create_dir_all(dir))?;
    }
    fsutil::write_atomic(path, bytes)
}

fn write_fxt(dir: &Path, m: &Meta) -> Result<()> {
    let (Some(fxt), Some(c)) = (&m.files.fxt, &m.clothes) else {
        return Ok(());
    };
    let line = format!("{} {}\r\n", c.gxt, ide::hud_name(&m.name));
    fsutil::write_atomic(&dir.join(fxt), line.as_bytes())
}
