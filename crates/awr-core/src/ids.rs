//! Deterministic ID allocation (spec §4.1): lowest free ID in the range wins.

use crate::error::{AwrError, Result};
use crate::inventory::{folder_id, read_dirs, split_disabled};
use std::{collections::BTreeSet, ops::RangeInclusive, path::Path};

pub const CEILING: u32 = 19999;
/// MixMods weapons tutorial reserves these model IDs for weapons.
pub const WEAPON_RANGE: RangeInclusive<u32> = 12400..=12499;

/// IDs in use: the game's own (`original`) plus every `{id}-*` folder in the
/// modloader, two levels deep, enabled or `. `-disabled (disabled mods keep their ID).
pub fn collect_used_ids(
    original: impl IntoIterator<Item = u32>,
    modloader: &Path,
) -> BTreeSet<u32> {
    let mut used: BTreeSet<u32> = original.into_iter().collect();
    for top in read_dirs(modloader) {
        for dir in std::iter::once(top.clone()).chain(read_dirs(&top)) {
            if let Some(id) = folder_id(&split_disabled(&dir).1) {
                used.insert(id);
            }
        }
    }
    used
}

/// Weapon type IDs start at 70 (MixMods tutorial); 0-69 belong to the game and the FLA.
pub fn allocate_weapon_type(used: &BTreeSet<u32>) -> Result<u32> {
    allocate(used, 70..=255, None)
}

pub fn allocate_vehicle(used: &BTreeSet<u32>, floor: u32) -> Result<u32> {
    allocate(used, floor..=CEILING, Some(&WEAPON_RANGE))
}

pub fn allocate_weapon(used: &BTreeSet<u32>, floor: u32) -> Result<u32> {
    allocate(
        used,
        floor.max(*WEAPON_RANGE.start())..=*WEAPON_RANGE.end(),
        None,
    )
}

fn allocate(
    used: &BTreeSet<u32>,
    range: RangeInclusive<u32>,
    skip: Option<&RangeInclusive<u32>>,
) -> Result<u32> {
    range
        .clone()
        .find(|id| !used.contains(id) && !skip.is_some_and(|s| s.contains(id)))
        .ok_or(AwrError::IdExhausted {
            start: *range.start(),
            end: *range.end(),
            used: used.range(range).count(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn disabled_folders_keep_their_id_and_gaps_are_reused() {
        let d = tempfile::tempdir().unwrap();
        for name in [
            "awr-cars/12500-gtr34",
            "awr-cars/. 12501-supra",
            "awr-cars/12503-evo",
            "other/12502-foreign",
        ] {
            fs::create_dir_all(d.path().join(name)).unwrap();
        }
        let used = collect_used_ids(400..=611, d.path());
        assert_eq!(allocate_vehicle(&used, 12500).unwrap(), 12504);

        fs::remove_dir(d.path().join("awr-cars/12503-evo")).unwrap();
        let used = collect_used_ids([], d.path());
        assert_eq!(allocate_vehicle(&used, 12500).unwrap(), 12503);
    }

    #[test]
    fn vehicles_never_take_weapon_ids() {
        let used = BTreeSet::new();
        assert_eq!(allocate_vehicle(&used, 12000).unwrap(), 12000);
        let used: BTreeSet<u32> = (12000..12400).collect();
        assert_eq!(allocate_vehicle(&used, 12000).unwrap(), 12500);
    }

    #[test]
    fn weapons_stay_in_their_range() {
        let used: BTreeSet<u32> = (12400..12499).collect();
        assert_eq!(allocate_weapon(&used, 12400).unwrap(), 12499);
        let full: BTreeSet<u32> = WEAPON_RANGE.collect();
        assert!(matches!(
            allocate_weapon(&full, 12400),
            Err(AwrError::IdExhausted { used: 100, .. })
        ));
    }
}
