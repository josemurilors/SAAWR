//! Weapons added without replacing (MixMods tutorial). A new weapon is a copy of an
//! original one: its `weapon.dat` lines, its `weap` IDE line, its `object.dat` line and its
//! `gtasa_weapon_config.dat` line, all renamed. The game's own files are never edited:
//! the first three live in the mod folder, the last one is the FLA's global file.

use crate::checker::{self, Severity};
use crate::error::{AwrError, NotFoundKind, Result};
use crate::injector::{edit_global, io_at, Injector, STAGING};
use crate::inventory::{self, ModKey, WEAPONS_DIR};
use crate::meta::{self, Files, Meta, ModKind};
use crate::text::{lines, replace_spans, tokens};
use crate::{fsutil, ids, validators};
use std::{collections::BTreeSet, fs, path::Path};

/// An original weapon that can be cloned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponRef {
    /// `weapon.dat` name, e.g. `DESERT_EAGLE`.
    pub name: String,
    /// `£` (0xA3) melee lines instead of `$` firearm lines.
    pub melee: bool,
    pub model_id: u32,
    /// IDE model name, e.g. `desert_eagle`.
    pub model: String,
    /// One `weapon.dat` line per skill level (poor, standard, pro), prefix included.
    pub dat_lines: Vec<Vec<u8>>,
    pub ide_line: Vec<u8>,
    pub object_line: Option<Vec<u8>>,
    pub config_line: Vec<u8>,
    /// `! SET_ASSOCIATED_WEAPON {name} X` lines of the config.
    pub associated: Vec<Vec<u8>>,
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn tok(line: &[u8], i: usize) -> Option<&[u8]> {
    tokens(line).get(i).map(|&(s, e)| &line[s..e])
}

/// `$` / `£` line of weapon.dat -> (melee, name, model id).
fn dat_head(line: &[u8]) -> Option<(bool, String, u32)> {
    let melee = match tok(line, 0)? {
        b"$" => false,
        b"\xA3" => true,
        _ => return None,
    };
    let id = text(tok(line, 5)?).parse().ok()?;
    Some((melee, text(tok(line, 1)?), id))
}

pub fn load_weapons(game_data: &Path, fla_data: &Path) -> Result<Vec<WeaponRef>> {
    Ok(parse_weapons(
        &fsutil::read(&game_data.join("default.ide"))?,
        &fsutil::read(&game_data.join("weapon.dat"))?,
        &fsutil::read(&game_data.join("object.dat"))?,
        &fsutil::read(&fla_data.join(CONFIG))?,
    ))
}

/// Weapons that have all the pieces to be cloned (the fist, jetpack etc. do not).
pub fn parse_weapons(ide: &[u8], dat: &[u8], objects: &[u8], config: &[u8]) -> Vec<WeaponRef> {
    let mut weap_ide = vec![];
    let mut inside = false;
    for l in lines(ide) {
        let t = l.trim_ascii();
        if inside && t == b"end" {
            inside = false;
        } else if !inside && t == b"weap" {
            inside = true;
        } else if inside && !t.is_empty() && !t.starts_with(b"#") {
            weap_ide.push(l);
        }
    }
    let mut out: Vec<WeaponRef> = vec![];
    for l in lines(dat).filter(|l| dat_head(l).is_some()) {
        let (melee, name, model_id) = dat_head(l).expect("filtered above");
        if let Some(w) = out.iter_mut().find(|w| w.name == name) {
            w.dat_lines.push(l.to_vec());
            continue;
        }
        let Some(ide_line) = weap_ide
            .iter()
            .find(|i| tok(i, 0).is_some_and(|t| text(t) == model_id.to_string()))
        else {
            continue;
        };
        let model = text(tok(ide_line, 1).unwrap_or_default());
        let config_line = lines(config)
            .find(|c| !c.starts_with(b"#") && tok(c, 1).is_some_and(|n| n == name.as_bytes()));
        let Some(config_line) = config_line else {
            continue;
        };
        let object_line = lines(objects)
            .find(|o| {
                !o.starts_with(b";")
                    && tok(o, 0).is_some_and(|t| t.eq_ignore_ascii_case(model.as_bytes()))
            })
            .map(<[u8]>::to_vec);
        let associated = lines(config)
            .filter(|c| is_associated(c, &name))
            .map(<[u8]>::to_vec)
            .collect();
        out.push(WeaponRef {
            name,
            melee,
            model_id,
            model,
            dat_lines: vec![l.to_vec()],
            ide_line: ide_line.to_vec(),
            object_line,
            config_line: config_line.to_vec(),
            associated,
        });
    }
    out
}

pub const CONFIG: &str = "gtasa_weapon_config.dat";

fn is_associated(line: &[u8], name: &str) -> bool {
    tok(line, 0) == Some(b"!")
        && tok(line, 1) == Some(b"SET_ASSOCIATED_WEAPON")
        && tok(line, 2).is_some_and(|t| t == name.as_bytes())
}

/// The reference's `weapon.dat` lines under the new name and model ID.
pub fn clone_dat(r: &WeaponRef, name: &str, model_id: u32) -> Vec<Vec<u8>> {
    let id = model_id.to_string();
    r.dat_lines
        .iter()
        .map(|l| {
            let t = tokens(l);
            replace_spans(l, &[(t[1], name.as_bytes()), (t[5], id.as_bytes())])
        })
        .collect()
}

/// `id, model, txd, ifp, ...` of the `weap` block: ID and file names swapped.
pub fn clone_ide(r: &WeaponRef, model_id: u32, short: &str) -> Vec<u8> {
    let t = tokens(&r.ide_line);
    let id = model_id.to_string();
    replace_spans(
        &r.ide_line,
        &[
            (t[0], id.as_bytes()),
            (t[1], short.as_bytes()),
            (t[2], short.as_bytes()),
        ],
    )
}

pub fn clone_object(r: &WeaponRef, short: &str) -> Option<Vec<u8>> {
    let l = r.object_line.as_deref()?;
    Some(replace_spans(l, &[(tokens(l)[0], short.as_bytes())]))
}

/// Everything the weapon needs in its `Loader.txt`: the IDE include and the cloned lines.
pub fn loader_txt(r: &WeaponRef, name: &str, short: &str, model_id: u32) -> Vec<u8> {
    let mut out: Vec<Vec<u8>> = vec![format!("IDE DATA\\MAPS\\{short}.ide").into_bytes()];
    out.extend(clone_dat(r, name, model_id));
    out.extend(clone_object(r, short));
    out.into_iter()
        .flat_map(|l| [l, b"\r\n".to_vec()].concat())
        .collect()
}

/// Type IDs in use in the config (active lines only).
pub fn config_types(file: &[u8]) -> BTreeSet<u32> {
    lines(file)
        .filter(|l| !l.starts_with(b"#"))
        .filter_map(|l| text(tok(l, 0)?).parse().ok())
        .collect()
}

fn eol_of(file: &[u8]) -> &'static [u8] {
    if file.windows(2).any(|w| w == b"\r\n") {
        b"\r\n"
    } else {
        b"\n"
    }
}

fn is_line_of(l: &[u8], name: &str) -> bool {
    !l.starts_with(b"#")
        && (tok(l, 1).is_some_and(|t| {
            t == name.as_bytes() && tok(l, 0).is_some_and(|t| t[0].is_ascii_digit())
        }) || is_associated(l, name))
}

/// Appends the weapon at the end, as the tutorial says: the reference's line with the new
/// type ID and name, plus the reference as parent (so it shoots and sounds the same). The
/// associated lines (rocket launchers) go after it. No-op if the name is already there.
pub fn config_add(file: &[u8], r: &WeaponRef, name: &str, type_id: u32) -> Vec<u8> {
    if lines(file).any(|l| is_line_of(l, name)) {
        return file.to_vec();
    }
    let eol = eol_of(file);
    let line = r.config_line.trim_ascii_end();
    let t = tokens(line);
    let id = type_id.to_string();
    let mut edits: Vec<((usize, usize), &[u8])> =
        vec![(t[0], id.as_bytes()), (t[1], name.as_bytes())];
    let mut new = if let Some(&parent) = t.get(10) {
        edits.push((parent, r.name.as_bytes()));
        replace_spans(line, &edits)
    } else {
        [&replace_spans(line, &edits)[..], b" ", r.name.as_bytes()].concat()
    };
    for a in &r.associated {
        new.extend_from_slice(eol);
        new.extend(replace_spans(a, &[(tokens(a)[2], name.as_bytes())]));
    }
    let mut out = file.to_vec();
    // a file without a final newline stays that way, so `config_remove` can undo it exactly
    if out.is_empty() || out.ends_with(b"\n") {
        out.extend(new);
        out.extend_from_slice(eol);
    } else {
        out.extend_from_slice(eol);
        out.extend(new);
    }
    out
}

pub fn config_remove(file: &[u8], name: &str) -> Vec<u8> {
    let gone = |raw: &&[u8]| {
        let l = raw.strip_suffix(b"\n").unwrap_or(raw);
        is_line_of(l.strip_suffix(b"\r").unwrap_or(l), name)
    };
    let raw: Vec<&[u8]> = file.split_inclusive(|&b| b == b'\n').collect();
    let mut out: Vec<u8> = raw
        .iter()
        .filter(|l| !gone(l))
        .flat_map(|l| l.iter().copied())
        .collect();
    // the last line had no newline: the one before it only ended because we added a line
    if raw.last().is_some_and(|l| gone(l) && !l.ends_with(b"\n")) {
        out.truncate(out.len() - eol_of(file).len());
    }
    out
}

pub struct InjectWeapon<'a> {
    pub short: &'a str,
    pub dff: &'a Path,
    pub txd: &'a Path,
    /// `weapon.dat` name of the original weapon to clone, e.g. `DESERT_EAGLE`.
    pub reference: &'a str,
    pub created: &'a str,
}

/// `(offset of the 32-byte name field, name)` of every texture in a RenderWare `.txd`.
/// Layout: TXD (0x16) > texture native (0x15) > struct (0x01) = platform, filter, name[32].
pub fn txd_textures(b: &[u8]) -> Vec<(usize, String)> {
    let word = |o: usize| {
        b.get(o..o + 4)
            .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
    };
    let mut out = vec![];
    let mut o = 12; // inside the TXD chunk
    while let (Some(t), Some(s)) = (word(o), word(o + 4)) {
        let body = o + 12;
        if t == 0x15 {
            let at = body + 12 + 8;
            if let Some(n) = b.get(at..at + 32) {
                let end = n.iter().position(|&c| c == 0).unwrap_or(32);
                out.push((at, String::from_utf8_lossy(&n[..end]).into_owned()));
            }
        }
        o = body + s as usize;
    }
    out
}

/// HUD textures a weapon `.txd` carries: `{name}icon`, and `{name}crosshair` for scopes.
const HUD_SUFFIXES: [&str; 2] = ["icon", "crosshair"];

/// Renames the pack's HUD textures (`gun_dildo1icon`) to the new weapon's (`yardstickicon`)
/// when there is exactly one of each kind and it is not already right.
/// Returns the new file, or None if nothing needed changing.
pub fn fix_hud_textures(txd: &[u8], short: &str) -> Option<Vec<u8>> {
    let tex = txd_textures(txd);
    let mut out = txd.to_vec();
    let mut changed = false;
    for suffix in HUD_SUFFIXES {
        let want = format!("{short}{suffix}");
        let found: Vec<_> = tex
            .iter()
            .filter(|(_, n)| n.to_ascii_lowercase().ends_with(suffix))
            .collect();
        if let [(at, name)] = found[..] {
            if !name.eq_ignore_ascii_case(&want) && want.len() <= 31 {
                out[*at..*at + 32].fill(0);
                out[*at..*at + want.len()].copy_from_slice(want.as_bytes());
                changed = true;
            }
        }
    }
    changed.then_some(out)
}

/// The HUD icon is the texture `{short}icon` inside the weapon's `.txd` (tutorial step 6).
pub fn has_icon(txd: &Path, short: &str) -> bool {
    let want = format!("{short}icon");
    fs::read(txd).is_ok_and(|b| {
        txd_textures(&b)
            .iter()
            .any(|(_, n)| n.eq_ignore_ascii_case(&want))
    })
}

impl Injector<'_> {
    pub fn inject_weapon(&self, req: &InjectWeapon) -> Result<ModKey> {
        let health = checker::check(self.info);
        if let Some(e) = health.iter().find(|f| f.severity == Severity::Error) {
            return Err(AwrError::Unhealthy(e.msg.clone()));
        }
        self.not_running()?;
        let ml = self.modloader()?;
        let r = self
            .weapons
            .iter()
            .find(|w| w.name.eq_ignore_ascii_case(req.reference))
            .ok_or_else(|| {
                AwrError::InvalidName(format!(
                    "arma de referência desconhecida: {}",
                    req.reference
                ))
            })?;
        let taken = self.taken_shorts();
        validators::check_weapon_name(req.short, |s| taken.contains(s))?;

        let installed: Vec<Meta> = inventory::scan(&ml)
            .into_iter()
            .filter_map(|m| m.meta)
            .collect();
        let used = ids::collect_used_ids(
            self.refs
                .iter()
                .map(|v| v.id)
                .chain(self.weapons.iter().map(|w| w.model_id)),
            &ml,
        );
        let id = ids::allocate_weapon(&used, *crate::ids::WEAPON_RANGE.start())?;
        let config = fsutil::read(&self.weapon_config()?)?;
        let mut types = config_types(&config);
        types.extend(installed.iter().filter_map(|m| m.weapon_type_id));
        let type_id = ids::allocate_weapon_type(&types)?;

        let folder = format!("{id}-{}", req.short);
        let dest = ml.join(WEAPONS_DIR).join(&folder);
        if dest.exists() {
            return Err(AwrError::AlreadyExists { path: dest });
        }
        let name = req.short.to_ascii_uppercase();
        let m = Meta {
            v: 1,
            kind: ModKind::Weapon,
            id,
            weapon_type_id: Some(type_id),
            name: req.short.into(),
            short: req.short.into(),
            vehicle_type: None,
            reference: r.name.clone(),
            traffic_group: None,
            audio: None,
            created: req.created.into(),
            files: Files {
                dff: format!("{}.dff", req.short),
                txd: format!("{}.txd", req.short),
                fxt: None,
            },
            parts: vec![],
            pack_handling: false,
        };

        let stage = ml.join(STAGING).join(&folder);
        io_at(&stage, fs::create_dir_all(&stage))?;
        io_at(req.dff, fs::copy(req.dff, stage.join(&m.files.dff)))?;
        // the pack's HUD textures are renamed to this weapon's; the source file is untouched
        let txd = fsutil::read(req.txd)?;
        let txd = fix_hud_textures(&txd, req.short).unwrap_or(txd);
        fsutil::write_atomic(&stage.join(&m.files.txd), &txd)?;
        let ide = [
            b"weap\r\n".to_vec(),
            clone_ide(r, id, req.short),
            b"\r\nend\r\n".to_vec(),
        ]
        .concat();
        fsutil::write_atomic(&stage.join(format!("{}.ide", req.short)), &ide)?;
        fsutil::write_atomic(
            &stage.join("Loader.txt"),
            &loader_txt(r, &name, req.short, id),
        )?;
        meta::write(&stage, &m)?;

        self.weapon_globals(&m, true)?;
        io_at(&dest, fs::create_dir_all(dest.parent().unwrap_or(&ml)))?;
        io_at(&dest, fs::rename(&stage, &dest))?;
        let _ = fs::remove_dir(ml.join(STAGING));
        self.sync_fla()?;
        Ok(dest)
    }

    fn weapon_config(&self) -> Result<std::path::PathBuf> {
        Ok(self
            .info
            .fla_data()
            .ok_or(AwrError::NotFound(NotFoundKind::FlaIni))?
            .join(CONFIG))
    }

    /// The weapon's line in the FLA's `gtasa_weapon_config.dat`.
    pub(crate) fn weapon_globals(&self, m: &Meta, add: bool) -> Result<()> {
        let name = m.short.to_ascii_uppercase();
        let reference = self.weapons.iter().find(|w| w.name == m.reference);
        let type_id = m.weapon_type_id.unwrap_or(70);
        let path = self.weapon_config()?;
        match (add, reference) {
            (true, Some(r)) => edit_global(&path, |f| config_add(f, r, &name, type_id)),
            (true, None) => Err(AwrError::InvalidName(format!(
                "arma de referência desconhecida: {}",
                m.reference
            ))),
            (false, _) => edit_global(&path, |f| config_remove(f, &name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDE: &[u8] = b"weap\r\n# gift\r\n321, gun_dildo1, gun_dildo1, null, 1, 50, 0\r\n348, desert_eagle, desert_eagle, python, 1, 30, 0\r\n35, rlauncher, rlauncher, rocket, 1, 30, 0\r\nend\r\n";
    const DAT: &[u8] = b"; head\r\n\xA3 DILDO1\t\tMELEE\t10.0 1.6\t321\t-1\r\n$ DESERT_EAGLE\t\tINSTANT_HIT\t25.0 30.0\t348\t-1\r\n$ DESERT_EAGLE\t\tINSTANT_HIT\t30.0 35.0\t348\t-1\r\n$ FIST\tMELEE\t1.0 1.0\t99\t-1\r\n";
    const OBJ: &[u8] = b"; c\r\ndesert_eagle\t\t1250.0,\t2500.0\t0.99\r\n";
    const CFG: &[u8] = b"# head\r\n10    DILDO1       -1  1  1\r\n24    DESERT_EAGLE        71   1   0   10   149  194   24  1.0   \r\n! SET_ASSOCIATED_WEAPON DESERT_EAGLE ROCKET\r\n";

    fn refs() -> Vec<WeaponRef> {
        parse_weapons(IDE, DAT, OBJ, CFG)
    }

    #[test]
    fn parses_only_weapons_with_every_piece() {
        let r = refs();
        let names: Vec<_> = r
            .iter()
            .map(|w| (w.name.as_str(), w.melee, w.dat_lines.len()))
            .collect();
        // FIST has no IDE/config line; DILDO1 has no object.dat line but is still cloneable
        assert_eq!(names, [("DILDO1", true, 1), ("DESERT_EAGLE", false, 2)]);
        let d = &r[1];
        assert_eq!((d.model.as_str(), d.model_id), ("desert_eagle", 348));
        assert!(d.object_line.is_some() && r[0].object_line.is_none());
        assert_eq!(d.associated.len(), 1);
    }

    #[test]
    fn clones_keep_the_pound_sign_byte_and_swap_name_and_id() {
        let r = refs();
        let melee = clone_dat(&r[0], "MYBAT", 12400);
        assert_eq!(melee[0], b"\xA3 MYBAT\t\tMELEE\t10.0 1.6\t12400\t-1");
        assert_eq!(
            clone_ide(&r[1], 12400, "revolver"),
            b"12400, revolver, revolver, python, 1, 30, 0"
        );
        assert_eq!(
            clone_object(&r[1], "revolver").unwrap(),
            b"revolver\t\t1250.0,\t2500.0\t0.99"
        );
        let loader = loader_txt(&r[1], "REVOLVER", "revolver", 12400);
        assert!(loader.starts_with(b"IDE DATA\\MAPS\\revolver.ide\r\n$ REVOLVER"));
    }

    /// A TXD with the given texture names: header, then one native texture each.
    fn fake_txd(names: &[&str]) -> Vec<u8> {
        let mut b = vec![];
        let chunk = |t: u32, body: &[u8]| {
            [
                &t.to_le_bytes()[..],
                &(body.len() as u32).to_le_bytes(),
                &0u32.to_le_bytes(),
                body,
            ]
            .concat()
        };
        let mut natives = vec![];
        for n in names {
            let mut st = vec![0u8; 8];
            let mut name = [0u8; 32];
            name[..n.len()].copy_from_slice(n.as_bytes());
            st.extend(name);
            st.extend([0u8; 40]);
            natives.extend(chunk(0x15, &chunk(0x01, &st)));
        }
        b.extend(chunk(0x16, &[chunk(0x01, &[0u8; 4]), natives].concat()));
        b
    }

    #[test]
    fn hud_textures_follow_the_new_name_without_touching_others() {
        let txd = fake_txd(&["gun_dildo1icon", "WP00_skin"]);
        let names = |b: &[u8]| txd_textures(b).into_iter().map(|t| t.1).collect::<Vec<_>>();
        assert_eq!(names(&txd), ["gun_dildo1icon", "WP00_skin"]);
        let fixed = fix_hud_textures(&txd, "yardstick").unwrap();
        assert_eq!(names(&fixed), ["yardstickicon", "WP00_skin"]);
        assert_eq!(fixed.len(), txd.len());
        assert!(
            fix_hud_textures(&fixed, "yardstick").is_none(),
            "already right"
        );
        // no icon at all (the AUG pack): nothing to rename
        assert!(fix_hud_textures(&fake_txd(&["aug", "weaponskin"]), "aug").is_none());
        // two icons: ambiguous, left alone
        assert!(fix_hud_textures(&fake_txd(&["aicon", "bicon"]), "x").is_none());
    }

    #[test]
    fn config_adds_at_the_end_with_parent_and_removes_cleanly() {
        let r = refs();
        let out = config_add(CFG, &r[1], "REVOLVER", 70);
        let tail = String::from_utf8_lossy(&out[CFG.len()..]).into_owned();
        assert_eq!(
            tail,
            "70    REVOLVER        71   1   0   10   149  194   24  1.0 DESERT_EAGLE\r\n! SET_ASSOCIATED_WEAPON REVOLVER ROCKET\r\n"
        );
        assert_eq!(config_add(&out, &r[1], "REVOLVER", 70), out);
        assert_eq!(config_remove(&out, "REVOLVER"), CFG);
        assert!(config_types(&out).contains(&70) && !config_types(CFG).contains(&70));
        // no newline at the end of the file: one is added before the new line
        let bare = config_add(b"24 X", &r[0], "A", 70);
        assert!(bare.starts_with(b"24 X\n70") || bare.starts_with(b"24 X\r\n70"));
    }
}
