//! Batch import: a folder of sub-folders, one vehicle pack each. The folder name
//! becomes the vehicle name, and the reference defaults to the model the pack replaces
//! (the `.dff` name), so most packs need no questions. Clothes are recognised first: a
//! texture named like one of the game's own clothing textures is an item (see
//! [`ClothesGuess`]), and a pack with several of them gives one item each.

use crate::pack::{self, PackFiles};
use crate::references::VehicleRef;
use crate::stores::{self, Original};
use crate::validators;
use crate::weapons::WeaponRef;
use std::{collections::BTreeSet, fs, path::Path, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub folder: PathBuf,
    pub name: String,
    pub short: String,
    /// Original model the pack was made to replace, if the `.dff` name says so.
    pub reference: Option<String>,
    /// `reference` is a weapon name (`weapon.dat`), not a vehicle model.
    pub weapon: bool,
    pub files: PackFiles,
    /// The item is clothing, hair or an accessory, and what the game's own list suggests for it.
    pub clothes: Option<ClothesGuess>,
}

/// What the original clothing list says about a texture: see [`stores::suggest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClothesGuess {
    pub part: u8,
    /// Original model to wear the texture on; empty when the pack brings its own `.dff`.
    pub model: String,
    pub price: u32,
    /// Shop sections that sold the original item.
    pub shops: Vec<String>,
    /// The original texture the guess comes from.
    pub from: String,
}

/// `.dff` named after an original vehicle (`sanchez.dff`) means "clone that one".
pub fn guess_reference(files: &PackFiles, refs: &[VehicleRef]) -> Option<String> {
    let stem = files
        .dff
        .as_ref()?
        .file_stem()?
        .to_string_lossy()
        .to_lowercase();
    refs.iter()
        .find(|r| r.model == stem)
        .map(|r| r.model.clone())
}

/// `.dff` named after an original weapon model (`gun_dildo1.dff`) means "clone that one".
pub fn guess_weapon(files: &PackFiles, weapons: &[WeaponRef]) -> Option<String> {
    let stem = files
        .dff
        .as_ref()?
        .file_stem()?
        .to_string_lossy()
        .to_lowercase();
    weapons
        .iter()
        .find(|w| w.model.to_lowercase() == stem)
        .map(|w| w.name.clone())
}

/// First free short: the name's own letters, then with a number (`launch` -> `launc2`).
pub fn suggest_short(name: &str, taken: &BTreeSet<String>) -> String {
    let base = match validators::short_from(name) {
        s if s.is_empty() || !s.as_bytes()[0].is_ascii_lowercase() => format!("car{}", s),
        s => s,
    };
    free_name(&base, validators::SHORT_MAX, taken)
}

/// Same for weapons: up to 15 characters, `_` allowed (`Desert Eagle Gold` -> `desert_eagle_go`).
pub fn suggest_weapon_name(name: &str, taken: &BTreeSet<String>) -> String {
    let words: Vec<String> = crate::text::fold_ascii(name)
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect();
    let base = words.join("_");
    let base = match base.as_bytes().first() {
        Some(b) if b.is_ascii_lowercase() => base,
        _ => format!("wpn{base}"),
    };
    free_name(&base, 15, taken)
}

fn free_name(base: &str, max: usize, taken: &BTreeSet<String>) -> String {
    let base: String = base.chars().take(max).collect();
    let base = base.trim_end_matches('_').to_string();
    if !taken.contains(&base) {
        return base;
    }
    (2..100)
        .map(|n| {
            let tail = n.to_string();
            let keep = max - tail.len();
            format!("{}{tail}", base.chars().take(keep).collect::<String>())
        })
        .find(|s| !taken.contains(s))
        .unwrap_or(base)
}

fn candidate(
    folder: &Path,
    refs: &[VehicleRef],
    taken: &mut BTreeSet<String>,
) -> Option<Candidate> {
    let files = pack::discover(folder, "");
    files.dff.as_ref()?;
    files.txd.as_ref()?;
    let name = folder.file_name()?.to_string_lossy().into_owned();
    let short = suggest_short(&name, taken);
    taken.insert(short.clone());
    Some(Candidate {
        folder: folder.to_path_buf(),
        name,
        short,
        reference: guess_reference(&files, refs),
        weapon: false,
        files,
        clothes: None,
    })
}

/// One candidate per `.txd` named like an original clothing texture. The pack's own `.dff` is
/// the one named like the original model, or the only one; without it the texture is worn on
/// the original model. `seen` keeps a texture name from being offered twice (packs often ship
/// a second, high-resolution copy in a sub-folder).
#[allow(clippy::too_many_arguments)]
fn clothes_candidates(
    folder: &Path,
    pack_name: bool,
    txds: &[PathBuf],
    dffs: &[PathBuf],
    db: &[Original],
    taken: &mut BTreeSet<String>,
    seen: &mut BTreeSet<String>,
) -> Vec<Candidate> {
    let stem = |p: &Path| {
        p.file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let matched: Vec<(&PathBuf, stores::Suggestion)> = txds
        .iter()
        .filter(|t| !seen.contains(&stem(t).to_lowercase()))
        .filter_map(|t| Some((t, stores::suggest(db, &stem(t), None)?)))
        .collect();
    let alone = matched.len() == 1;
    let mut out = vec![];
    for (txd, sug) in matched {
        let tex = stem(txd);
        seen.insert(tex.to_lowercase());
        let own = dffs
            .iter()
            .find(|d| stem(d).eq_ignore_ascii_case(&sug.model))
            .or_else(|| (dffs.len() == 1).then(|| &dffs[0]))
            .cloned();
        let name = match folder.file_name() {
            // a pack with one item is named after its folder; loose files, by their own name
            Some(n) if alone && pack_name => n.to_string_lossy().into_owned(),
            _ => tex,
        };
        let short = suggest_short(&name, taken);
        taken.insert(short.clone());
        out.push(Candidate {
            folder: folder.to_path_buf(),
            name,
            short,
            reference: None,
            weapon: false,
            clothes: Some(ClothesGuess {
                part: sug.part,
                model: if own.is_some() {
                    String::new()
                } else {
                    sug.model
                },
                price: sug.price,
                shops: sug.shops,
                from: sug.from,
            }),
            files: PackFiles {
                dff: own,
                txd: Some(txd.clone()),
                ..Default::default()
            },
        });
    }
    out
}

/// Clothes in `dir` and, `depth` levels down, in its sub-folders.
fn clothes_tree(
    dir: &Path,
    db: &[Original],
    taken: &mut BTreeSet<String>,
    seen: &mut BTreeSet<String>,
    depth: u8,
) -> Vec<Candidate> {
    let mut out = clothes_candidates(
        dir,
        true,
        &pack::with_ext(dir, "txd"),
        &pack::with_ext(dir, "dff"),
        db,
        taken,
        seen,
    );
    if depth > 0 {
        let mut subs: Vec<PathBuf> = fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        subs.sort();
        for s in subs {
            out.extend(clothes_tree(&s, db, taken, seen, depth - 1));
        }
    }
    out
}

/// A folder of clothes gives one candidate per texture (looking three levels down). A vehicle
/// pack folder itself gives one candidate; otherwise every sub-folder that holds a `.dff` and
/// a `.txd`. `taken` are shorts already used (game models, installed mods).
pub fn scan(
    dir: &Path,
    refs: &[VehicleRef],
    db: &[Original],
    taken: &BTreeSet<String>,
) -> Vec<Candidate> {
    let mut taken = taken.clone();
    let clothes = clothes_tree(dir, db, &mut taken, &mut BTreeSet::new(), 3);
    if !clothes.is_empty() {
        return clothes;
    }
    if let Some(c) = candidate(dir, refs, &mut taken) {
        return vec![c];
    }
    let mut subs: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    subs.sort();
    subs.iter()
        .filter_map(|s| candidate(s, refs, &mut taken))
        .collect()
}

/// Everything the user dropped or picked: pack folders, folders of packs, loose `.dff` +
/// `.txd` pairs, lone `.txd`s (clothes). A `.dff` named after an original weapon model
/// becomes a weapon; a texture named after an original clothing texture becomes clothing.
/// `taken` grows with the shorts handed out, so a whole drop gets distinct names.
pub fn from_paths(
    paths: &[PathBuf],
    refs: &[VehicleRef],
    weapons: &[WeaponRef],
    db: &[Original],
    taken: &mut BTreeSet<String>,
) -> Vec<Candidate> {
    let mut found: Vec<Candidate> = vec![];
    for p in paths.iter().filter(|p| p.is_dir()) {
        let c = scan(p, refs, db, taken);
        taken.extend(c.iter().map(|c| c.short.clone()));
        found.extend(c);
    }
    let is = |p: &Path, ext: &str| p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext));
    // loose textures named like the game's own clothes
    let loose = |ext| -> Vec<PathBuf> { paths.iter().filter(|p| is(p, ext)).cloned().collect() };
    let (txds, dffs) = (loose("txd"), loose("dff"));
    let folder = paths
        .first()
        .and_then(|p| p.parent())
        .unwrap_or(Path::new(""));
    let clothes = clothes_candidates(folder, false, &txds, &dffs, db, taken, &mut BTreeSet::new());
    let used: Vec<PathBuf> = clothes.iter().filter_map(|c| c.files.txd.clone()).collect();
    found.extend(clothes);
    for dff in paths.iter().filter(|p| is(p, "dff")) {
        let Some(txd) = paths
            .iter()
            .find(|p| is(p, "txd") && p.file_stem() == dff.file_stem() && !used.contains(p))
        else {
            continue;
        };
        let name = dff
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let files = PackFiles {
            dff: Some(dff.clone()),
            txd: Some(txd.clone()),
            ..Default::default()
        };
        let short = suggest_short(&name, taken);
        taken.insert(short.clone());
        found.push(Candidate {
            folder: dff.parent().map(Path::to_path_buf).unwrap_or_default(),
            reference: guess_reference(&files, refs),
            weapon: false,
            name,
            short,
            files,
            clothes: None,
        });
    }
    // a lone `.txd` can only be clothing: a new texture over an original model
    let paired = |t: &PathBuf| {
        paths
            .iter()
            .any(|d| is(d, "dff") && d.file_stem() == t.file_stem())
    };
    for txd in paths
        .iter()
        .filter(|p| is(p, "txd") && !paired(p) && !used.contains(p))
    {
        let name = txd
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let short = suggest_short(&name, taken);
        taken.insert(short.clone());
        found.push(Candidate {
            folder: txd.parent().map(Path::to_path_buf).unwrap_or_default(),
            reference: None,
            weapon: false,
            name,
            short,
            files: PackFiles {
                txd: Some(txd.clone()),
                ..Default::default()
            },
            clothes: None,
        });
    }
    for c in found
        .iter_mut()
        .filter(|c| c.reference.is_none() && c.clothes.is_none())
    {
        if let Some(w) = guess_weapon(&c.files, weapons) {
            c.reference = Some(w);
            c.weapon = true;
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn dff_named_after_a_weapon_model_picks_that_weapon() {
        let w = |name: &str, model: &str| WeaponRef {
            name: name.into(),
            melee: true,
            model_id: 321,
            model: model.into(),
            dat_lines: vec![],
            ide_line: vec![],
            object_line: None,
            config_line: vec![],
            associated: vec![],
        };
        let weapons = [w("DILDO1", "gun_dildo1"), w("KATANA", "katana")];
        let files = |dff: &str| PackFiles {
            dff: Some(PathBuf::from(dff)),
            ..Default::default()
        };
        assert_eq!(
            guess_weapon(&files("x/GUN_DILDO1.dff"), &weapons).as_deref(),
            Some("DILDO1")
        );
        assert_eq!(guess_weapon(&files("x/luger.dff"), &weapons), None);
    }

    #[test]
    fn shorts_avoid_taken_names() {
        assert_eq!(suggest_short("GT R34 Skyline", &set(&[])), "gtr34sk");
        assert_eq!(
            suggest_short("Launch", &set(&["launch"])),
            "launch2".chars().take(7).collect::<String>()
        );
        assert_eq!(
            suggest_short("Launch", &set(&["launch", "launch2"])),
            "launch3"
        );
        assert_eq!(
            suggest_short("Marinha do Brasil", &set(&["marinha"])),
            "marinh2"
        );
        assert_eq!(suggest_short("800 GS", &set(&[])), "car800g");
        assert_eq!(suggest_short("???", &set(&[])), "car");
    }

    #[test]
    fn weapon_names_allow_underscores_and_fifteen_characters() {
        assert_eq!(
            suggest_weapon_name("Desert Eagle Gold", &set(&[])),
            "desert_eagle_go"
        );
        assert_eq!(
            suggest_weapon_name("Pistola Parabéllum", &set(&["pistola_parabel"])),
            "pistola_parabe2"
        );
        assert_eq!(suggest_weapon_name("9mm", &set(&[])), "wpn9mm");
    }

    #[test]
    fn a_lone_txd_becomes_a_texture_only_candidate() {
        let p = |s: &str| PathBuf::from(s);
        let found = from_paths(
            &[p("x/carro.dff"), p("x/carro.txd"), p("x/Camiseta.txd")],
            &[],
            &[],
            &[],
            &mut set(&[]),
        );
        let view: Vec<_> = found
            .iter()
            .map(|c| (c.short.as_str(), c.files.dff.is_some()))
            .collect();
        assert_eq!(view, [("carro", true), ("camiset", false)]);
    }

    #[test]
    fn folder_of_packs_gets_one_candidate_per_subfolder() {
        let d = tempfile::tempdir().unwrap();
        for f in [
            "Moto GS/sanchez.dff",
            "Moto GS/sanchez.txd",
            "Barco/launch.dff",
            "Barco/launch.txd",
            "Vazia/readme.txt",
        ] {
            let p = d.path().join(f);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, b"").unwrap();
        }
        let c = scan(d.path(), &[], &[], &set(&["barco"]));
        let view: Vec<_> = c
            .iter()
            .map(|c| (c.name.as_str(), c.short.as_str(), c.reference.clone()))
            .collect();
        assert_eq!(
            view,
            [("Barco", "barco2", None), ("Moto GS", "motogs", None)]
        );
    }

    fn orig(tex: &str, model: &str, part: u8, price: u32, shop: &str) -> Original {
        Original {
            texture: tex.into(),
            model: model.into(),
            part,
            price,
            shops: vec![shop.into()],
        }
    }

    fn game_clothes() -> Vec<Original> {
        vec![
            orig("hoodyablack", "hoodya", 0, 65, "clothgp"),
            orig("hoodyablue", "hoodya", 0, 65, "clothgp"),
            orig("watchyellow", "watch", 14, 15, "CSchp"),
            orig("sneakerheatgry", "sneaker", 3, 65, "LACS1"),
        ]
    }

    fn touch(d: &Path, files: &[&str]) {
        for f in files {
            let p = d.join(f);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, b"").unwrap();
        }
    }

    #[test]
    fn a_clothes_pack_gives_one_item_per_texture_and_suggests_from_the_originals() {
        let d = tempfile::tempdir().unwrap();
        touch(
            d.path(),
            &[
                // nested like downloaded packs; the pack's own model has the original name
                "Original Hoodie DLC/Hoodies/hoodya.dff",
                "Original Hoodie DLC/Hoodies/hoodyablack.txd",
                "Original Hoodie DLC/Hoodies/hoodyablue.txd",
                "Original Hoodie DLC/alt/preview.png",
                // two models and one texture: the one named like the original model is used
                "Rolex/watch.dff",
                "Rolex/watch_hoody.dff",
                "Rolex/watchyellow.txd",
                // a texture alone is worn on the original model; a second copy is not offered
                "vans/sneakerheatgry.txd",
                "vans/lpp retexture/sneakerheatgry.txd",
                // not clothes
                "Carro/landstal.dff",
                "Carro/landstal.txd",
            ],
        );
        let db = game_clothes();
        let c = scan(d.path(), &[], &db, &set(&[]));
        let view: Vec<_> = c
            .iter()
            .map(|c| {
                let g = c.clothes.as_ref().unwrap();
                (
                    c.name.as_str(),
                    c.files.dff.is_some(),
                    g.part,
                    g.model.as_str(),
                    g.price,
                    g.shops.clone(),
                )
            })
            .collect();
        assert_eq!(
            view,
            [
                ("hoodyablack", true, 0, "", 65, vec!["clothgp".to_string()]),
                ("hoodyablue", true, 0, "", 65, vec!["clothgp".to_string()]),
                ("Rolex", true, 14, "", 15, vec!["CSchp".to_string()]),
                ("vans", false, 3, "sneaker", 65, vec!["LACS1".to_string()]),
            ]
        );
        assert!(c[2]
            .files
            .dff
            .as_ref()
            .unwrap()
            .ends_with("Rolex/watch.dff"));
        assert_eq!(
            c[1].short, "hoodya2",
            "a short is never repeated in one scan"
        );
        // a folder without clothing textures stays a vehicle pack
        let v = scan(&d.path().join("Carro"), &[], &db, &set(&[]));
        assert_eq!(v.len(), 1);
        assert!(v[0].clothes.is_none());
    }

    #[test]
    fn loose_clothing_textures_become_items_with_the_loose_model() {
        let p = |s: &str| PathBuf::from(s);
        let found = from_paths(
            &[p("x/hoodya.dff"), p("x/hoodyablue.txd"), p("x/semnome.txd")],
            &[],
            &[],
            &game_clothes(),
            &mut set(&[]),
        );
        let view: Vec<_> = found
            .iter()
            .map(|c| (c.name.as_str(), c.files.dff.is_some(), c.clothes.is_some()))
            .collect();
        // the clothing texture takes the loose model; the unknown texture stays "texture only"
        assert_eq!(
            view,
            [("hoodyablue", true, true), ("semnome", false, false)]
        );
    }
}
