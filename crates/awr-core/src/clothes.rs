//! New CJ clothes (spec: plano roupas). Each item is a folder in `awr-roupas` with its `.txd`
//! (and maybe a `.dff`) in a `player.img` folder, which ModLoader streams as new clothing,
//! plus the `.fxt` with its menu name. The shops list it through one shared `shopping.dat`
//! rebuilt from the enabled items ([`Injector::sync_stores`]).

use crate::error::{AwrError, Result};
use crate::injector::{io_at, Injector, STAGING};
use crate::inventory::{self, ModKey, CLOTHES_DIR};
use crate::meta::{self, Clothes, Files, Meta, ModKind};
use crate::{fsutil, ide, stores, validators};
use std::{fs, path::Path, path::PathBuf};

/// Holds the one `shopping.dat` ModLoader merges over the game's.
pub const STORES_DIR: &str = "awr-lojas";
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
    pub shops: &'a [String],
    pub created: &'a str,
}

/// One shop's free places per body part (`BODY_PARTS` order), for the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopSlots {
    pub section: String,
    pub free: Vec<usize>,
    /// Another mod changes this shop: AWR leaves it alone (ModLoader merges by line position).
    pub blocked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreInfo {
    pub shops: Vec<ShopSlots>,
    /// `(model, body part)` of the original clothing models, to wear a new texture.
    pub models: Vec<(String, u8)>,
    /// Price lines left before the game's limit.
    pub prices_free: usize,
    /// Another mod changes the clothes prices: no clothes can be added.
    pub blocked: bool,
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

/// Bytes as chars 1:1, so CP-1252 text survives the round trip (like `sync_mod_file`).
fn text_of(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
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
    fn game_shopping(&self) -> Result<String> {
        Ok(text_of(&fsutil::read(
            &self.info.data().join("shopping.dat"),
        )?))
    }

    /// Other mods' `shopping.dat` that ModLoader would load (disabled folders skipped).
    fn foreign_shopping(&self) -> Result<Vec<PathBuf>> {
        fn walk(dir: &Path, depth: u8, out: &mut Vec<PathBuf>) {
            for d in inventory::read_dirs(dir) {
                let name = d.file_name().map(|n| n.to_string_lossy().to_lowercase());
                let name = name.unwrap_or_default();
                if name.starts_with('.') || name == STORES_DIR {
                    continue;
                }
                let f = d.join("shopping.dat");
                if name == "data" && f.is_file() {
                    out.push(f);
                }
                if depth > 0 {
                    walk(&d, depth - 1, out);
                }
            }
        }
        let mut out = vec![];
        walk(&self.modloader()?, 5, &mut out);
        Ok(out)
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
    pub fn texture_fit(&self, txd: &[u8], model: Option<&str>, body_part: u8) -> TextureFit {
        let have = crate::txd::size(txd);
        let want = self.game_shopping().ok().and_then(|game| {
            let originals = stores::originals(&game);
            let same_part = originals.iter().filter(|(_, _, p)| *p == body_part);
            let same_model = same_part
                .clone()
                .filter(|(_, m, _)| Some(m.as_str()) == model);
            same_model.chain(same_part).take(6).find_map(|(tex, _, _)| {
                crate::txd::size(&self.active_clothing(&format!("{tex}.txd"))?)
            })
        });
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

    /// Free shop places, original models and limits, counting the enabled AWR clothes except
    /// `skip` (the item being edited).
    pub fn store_info(&self, skip: Option<&str>) -> Result<StoreInfo> {
        let game = self.game_shopping()?;
        let mut touched: Vec<String> = vec![];
        let mut foreign_prices = 0;
        for f in self.foreign_shopping()? {
            let t = text_of(&fsutil::read(&f)?);
            touched.extend(stores::touched(&game, &t));
            foreign_prices += stores::prices(&t).saturating_sub(stores::prices(&game));
        }
        let ours: Vec<Meta> = self
            .enabled_clothes()?
            .into_iter()
            .filter(|m| Some(m.short.as_str()) != skip)
            .collect();
        let items: Vec<&Clothes> = ours.iter().filter_map(|m| m.clothes.as_ref()).collect();
        let now = stores::build(&game, &items);
        let is_touched = |s: &str| touched.iter().any(|t| t.eq_ignore_ascii_case(s));
        Ok(StoreInfo {
            shops: stores::shops(&game)
                .into_iter()
                .map(|section| ShopSlots {
                    free: stores::BODY_PARTS
                        .iter()
                        .map(|&p| {
                            stores::MAX_PER_PART.saturating_sub(stores::count(&now, &section, p))
                        })
                        .collect(),
                    blocked: is_touched(&section),
                    section,
                })
                .collect(),
            models: {
                let mut m: Vec<(String, u8)> = stores::originals(&game)
                    .into_iter()
                    .map(|(_, model, part)| (model, part))
                    .collect();
                m.sort();
                m.dedup();
                m
            },
            prices_free: stores::MAX_PRICES.saturating_sub(stores::prices(&now) + foreign_prices),
            blocked: is_touched(stores::CLOTHES),
        })
    }

    /// Checks shops, body part and limits for one item (`skip`: itself, when editing).
    fn check_store(&self, c: &Clothes, skip: Option<&str>) -> Result<()> {
        let info = self.store_info(skip)?;
        if info.blocked {
            return Err(AwrError::InvalidName(
                "outro mod altera os preços de roupas do shopping.dat; desative-o para adicionar roupas".into(),
            ));
        }
        if skip.is_none() && info.prices_free == 0 {
            return Err(AwrError::InvalidName(format!(
                "o shopping.dat já tem {} itens, o limite do jogo",
                stores::MAX_PRICES
            )));
        }
        let part = stores::BODY_PARTS
            .iter()
            .position(|&p| p == c.body_part)
            .ok_or_else(|| {
                AwrError::InvalidName(format!("parte do corpo inválida: {}", c.body_part))
            })?;
        if c.shops.is_empty() {
            return Err(AwrError::InvalidName("escolha ao menos uma loja".into()));
        }
        for name in &c.shops {
            let s = info
                .shops
                .iter()
                .find(|s| s.section.eq_ignore_ascii_case(name))
                .ok_or_else(|| {
                    AwrError::InvalidName(format!("loja de roupas desconhecida: {name}"))
                })?;
            if s.blocked {
                return Err(AwrError::InvalidName(format!(
                    "outro mod altera a loja {name}; escolha outra"
                )));
            }
            if s.free[part] == 0 {
                return Err(AwrError::InvalidName(format!(
                    "a loja {name} já mostra {} itens dessa parte do corpo",
                    stores::MAX_PER_PART
                )));
            }
        }
        Ok(())
    }

    pub fn inject_clothes(&self, req: &InjectClothes) -> Result<ModKey> {
        self.not_running()?;
        let ml = self.modloader()?;
        let game = self.game_shopping()?;
        let originals = stores::originals(&game);
        let taken = self.taken_shorts();
        validators::check_short(req.short, |s| {
            taken.contains(s) || originals.iter().any(|(t, m, _)| t == s || m == s)
        })?;
        let model = match (req.dff, req.model) {
            (Some(_), _) => req.short.to_string(),
            (None, Some(m)) => {
                let m = m.to_lowercase();
                if !originals
                    .iter()
                    .any(|(_, om, p)| *om == m && *p == req.body_part)
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
        self.check_store(&clothes, None)?;

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
        let txd = match self.texture_fit(&txd, model, req.body_part).fit {
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
        if inventory::split_disabled(key).0 {
            let c = c.clone();
            self.check_store(&c, Some(&m.short))?;
        }
        write_fxt(key, m)
    }

    /// Rebuilds the shared `shopping.dat` from the enabled clothes; gone when it would equal
    /// the game's.
    pub(crate) fn sync_stores(&self) -> Result<()> {
        let Ok(ml) = self.modloader() else {
            return Ok(());
        };
        let dir = ml.join(STORES_DIR).join("data");
        let path = dir.join("shopping.dat");
        let metas = self.enabled_clothes()?;
        if metas.is_empty() && !path.exists() {
            return Ok(());
        }
        let game = self.game_shopping()?;
        let items: Vec<&Clothes> = metas.iter().filter_map(|m| m.clothes.as_ref()).collect();
        let new = stores::build(&game, &items);
        if new == game {
            let _ = fs::remove_dir_all(ml.join(STORES_DIR));
            return Ok(());
        }
        let bytes: Vec<u8> = new.chars().map(|c| c as u8).collect();
        if fs::read(&path).ok().as_deref() != Some(&bytes[..]) {
            io_at(&dir, fs::create_dir_all(&dir))?;
            fsutil::write_atomic(&path, &bytes)?;
        }
        Ok(())
    }
}

fn write_fxt(dir: &Path, m: &Meta) -> Result<()> {
    let (Some(fxt), Some(c)) = (&m.files.fxt, &m.clothes) else {
        return Ok(());
    };
    let line = format!("{} {}\r\n", c.gxt, ide::hud_name(&m.name));
    fsutil::write_atomic(&dir.join(fxt), line.as_bytes())
}
