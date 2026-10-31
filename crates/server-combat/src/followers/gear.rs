//! Source dungeon gear candidates directly from the active Journal and LFG data.
#[cfg(test)]
#[path = "gear/equip_tests.rs"]
mod equip_tests;
pub mod keystone;
pub mod levels;
pub mod select;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;

use shared::components::Level;
use shared::item_data::ItemDefinitionSource;

// Stockade is a Normal simulator fixture, not a follower LFG catalog entry.
const STOCKADE_SIMULATOR_MAP: u32 = 34;

#[derive(Deserialize)]
struct JournalInstance {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "MapID")]
    map: i32,
}

#[derive(Deserialize)]
struct JournalEncounter {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "JournalInstanceID")]
    instance: u32,
}

#[derive(Deserialize)]
struct JournalItem {
    #[serde(rename = "JournalEncounterID")]
    encounter: u32,
    #[serde(rename = "ItemID")]
    item: u32,
    #[serde(rename = "DifficultyMask")]
    difficulty_mask: i64,
    #[serde(rename = "FactionMask")]
    faction_mask: i64,
    #[serde(rename = "DisplaySeasonID")]
    season: u32,
    #[serde(rename = "WorldStateExpressionID")]
    world_state: u32,
}

fn read_rows<T: serde::de::DeserializeOwned>(
    directory: &Path,
    name: &str,
) -> Result<Vec<T>, String> {
    let path = directory.join(name);
    let mut reader = csv::Reader::from_path(&path)
        .map_err(|error| format!("open {}: {error}", path.display()))?;
    reader
        .deserialize()
        .collect::<Result<_, _>>()
        .map_err(|error| format!("read {}: {error}", path.display()))
}

/// A Journal row retaining its source dungeon and native availability masks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct JournalGearCandidate {
    pub item_id: u32,
    pub source_map: u32,
    pub difficulty_mask: i64,
    pub faction_mask: i64,
}

/// Return actual unconditional Journal items from the same ContentTuning expansion.
/// Conditional/seasonal Journal rows need separate state and are not inferred.
pub fn read_dungeon_item_pools(
    db: &Connection,
    db2: &Path,
) -> Result<HashMap<u32, Vec<JournalGearCandidate>>, String> {
    let mut statement = db.prepare("SELECT d.map_id,t.ExpansionID FROM follower_lfg_dungeons d JOIN content_tuning t ON t.ID=d.content_tuning_id WHERE d.difficulty_id=205 ORDER BY d.map_id")
        .map_err(|error| format!("read follower dungeon tiers: {error}"))?;
    let tiers: HashMap<u32, i32> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|error| format!("query follower dungeon tiers: {error}"))?
        .collect::<Result<_, _>>()
        .map_err(|error| format!("decode follower dungeon tiers: {error}"))?;
    let instances: HashMap<u32, u32> = read_rows::<JournalInstance>(db2, "JournalInstance.csv")?
        .into_iter()
        .filter_map(|row| u32::try_from(row.map).ok().map(|map| (row.id, map)))
        .collect();
    let encounters: HashMap<u32, u32> = read_rows::<JournalEncounter>(db2, "JournalEncounter.csv")?
        .into_iter()
        .filter_map(|row| instances.get(&row.instance).map(|map| (row.id, *map)))
        .collect();
    let mut by_tier: HashMap<i32, BTreeSet<JournalGearCandidate>> = HashMap::new();
    let mut stockade = BTreeSet::new();
    for row in read_rows::<JournalItem>(db2, "JournalEncounterItem.csv")? {
        if row.season != 0 || row.world_state != 0 {
            continue;
        }
        let Some(map) = encounters.get(&row.encounter) else {
            continue;
        };
        let item = JournalGearCandidate {
            item_id: row.item,
            source_map: *map,
            difficulty_mask: row.difficulty_mask,
            faction_mask: row.faction_mask,
        };
        if *map == STOCKADE_SIMULATOR_MAP {
            stockade.insert(item);
        }
        if let Some(tier) = tiers.get(map) {
            by_tier.entry(*tier).or_default().insert(item);
        }
    }
    let mut pools: HashMap<u32, Vec<JournalGearCandidate>> = tiers
        .into_iter()
        .map(|(map, tier)| {
            let items = by_tier
                .get(&tier)
                .filter(|items| !items.is_empty())
                .ok_or_else(|| {
                    format!("no unconditional Journal loot for map{map} expansion{tier}")
                })?;
            Ok((map, items.iter().copied().collect()))
        })
        .collect::<Result<_, String>>()?;
    if stockade.is_empty() {
        return Err("FOLLOWER_STOCKADE_JOURNAL_LOOT_UNAVAILABLE".into());
    }
    pools.insert(STOCKADE_SIMULATOR_MAP, stockade.into_iter().collect());
    Ok(pools)
}

/// Active source data needed to select and scale a follower's equipment.
#[derive(Resource)]
pub struct FollowerGearSource {
    pools: HashMap<u32, Vec<JournalGearCandidate>>,
    levels: levels::DungeonItemLevels,
    database: std::path::PathBuf,
}

pub fn load_source(database: &Path, db2: &Path) -> Result<FollowerGearSource, String> {
    let db = Connection::open_with_flags(database, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("open follower gear source: {error}"))?;
    Ok(FollowerGearSource {
        pools: read_dungeon_item_pools(&db, db2)?,
        levels: levels::DungeonItemLevels::read(db2)?,
        database: database.to_owned(),
    })
}

pub(super) fn load_startup(mut commands: Commands) {
    let database = Path::new(crate::spell_info::WORLD_DB_PATH);
    let db2 = database
        .parent()
        .expect("world database directory")
        .join("db2/active");
    match load_source(database, &db2) {
        Ok(source) => {
            commands.insert_resource(source);
        }
        Err(error) => error!("Follower gear source unavailable: {error}"),
    }
}

/// Equip source items at the current dungeon loot level through normal item instances.
pub fn equip(world: &mut World, actor: Entity, profile: &super::Profile) -> Result<(), String> {
    let source = world
        .get_resource::<FollowerGearSource>()
        .ok_or("FOLLOWER_GEAR_SOURCE_UNAVAILABLE")?;
    let copy = *world
        .get::<crate::maps::InMap>(actor)
        .ok_or("follower gear map unavailable")?;
    let difficulty = crate::zones::map_difficulty_id(world, copy);
    let level = world
        .get::<Level>(actor)
        .ok_or("follower gear level unavailable")?
        .0;
    let pool = source
        .pools
        .get(&copy.map_id)
        .ok_or_else(|| format!("FOLLOWER_GEAR_DUNGEON_UNAVAILABLE map{}", copy.map_id))?;
    let templates = world
        .get_resource::<crate::item_templates::ItemTemplates>()
        .ok_or("follower gear item templates unavailable")?;
    let native = world
        .get_resource::<crate::stats::NativeItemInstances>()
        .ok_or("follower gear item bonuses unavailable")?;
    let scaling = world
        .get_resource::<crate::item_stats::ItemScalingTables>()
        .ok_or("follower gear item stats unavailable")?;
    let classes = world
        .get_resource::<crate::spell_info::ClassSpells>()
        .ok_or("follower gear classes unavailable")?;
    let spec = classes
        .classes
        .iter()
        .find(|class| class.class_id == profile.class_id)
        .and_then(|class| {
            class
                .specs
                .iter()
                .find(|spec| spec.spec_id == profile.spec_id)
        })
        .ok_or("follower gear specialization unavailable")?;
    let known = world
        .get::<crate::class_progression::KnownSpells>(actor)
        .ok_or("follower gear spellbook unavailable")?;
    let spells = world
        .get_resource::<crate::spell_info::SpellStore>()
        .ok_or("follower gear spells unavailable")?;
    let selection = select::select_items(
        pool,
        copy.map_id,
        profile.class_id,
        crate::stats::PrimaryStat::from_priority(spec.primary_stat_priority),
        profile.spec_id,
        known,
        spells,
        templates,
    );
    if !selection.missing.is_empty() {
        if copy.map_id == STOCKADE_SIMULATOR_MAP {
            warn!(
                "FOLLOWER_STOCKADE_JOURNAL_SLOTS_UNAVAILABLE spec{}: {:?}",
                profile.spec_id, selection.missing
            );
        } else {
            return Err(format!(
                "FOLLOWER_GEAR_SLOTS_UNAVAILABLE spec{}: {:?}",
                profile.spec_id, selection.missing
            ));
        }
    }
    let representative = pool
        .iter()
        .filter(|item| {
            item.source_map == copy.map_id && item.difficulty_mask == -1 && item.faction_mask == -1
        })
        .find_map(|item| {
            templates
                .get(ItemDefinitionSource::Retail, item.item_id)
                .filter(|template| template.inventory_type != 0)
                .map(|template| (item.item_id, template))
        })
        .ok_or("FOLLOWER_GEAR_DUNGEON_LOOT_UNAVAILABLE")?;
    let db = Connection::open_with_flags(&source.database, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("open follower gear database: {error}"))?;
    let item_level = source.levels.resolve(
        &db,
        copy.map_id,
        difficulty,
        level,
        representative.0,
        representative.1,
        &native.bonuses,
        scaling,
    )?;
    let modifiers = selection
        .items
        .iter()
        .map(|(slot, _, _)| {
            (
                *slot,
                crate::item_instance::ItemInstanceModifiers {
                    item_level_override: Some(item_level),
                    ..Default::default()
                },
            )
        })
        .collect();
    super::parity_profile::equip_instances(
        world,
        actor,
        selection.items,
        crate::stats::EquippedItemModifiers {
            items: modifiers,
            ..Default::default()
        },
    );
    Ok(())
}

/// Publish the native appearance of the follower's equipped Journal items.
pub(super) fn publish_appearance(world: &mut World, actor: Entity) -> Result<(), String> {
    let source = world
        .get_resource::<FollowerGearSource>()
        .ok_or("FOLLOWER_GEAR_SOURCE_UNAVAILABLE")?;
    let db = Connection::open_with_flags(&source.database, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("open follower appearance database: {error}"))?;
    let items = world
        .get::<crate::stats::EquippedItems>(actor)
        .ok_or("follower equipment unavailable")?;
    let appearance = super::visual::read_equipment_appearance(&db, items)?;
    world.entity_mut(actor).insert(appearance);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::OpenFlags;

    #[test]
    fn stockade_fixture_uses_only_its_own_journal_loot() {
        let path = crate::test_data::world_db_path();
        let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let pools =
            read_dungeon_item_pools(&db, &path.parent().unwrap().join("db2/active")).unwrap();
        let stockade = &pools[&34];
        assert!(stockade.iter().any(|item| item.item_id == 2168));
        assert!(stockade.iter().all(|item| item.source_map == 34));
        assert_eq!(stockade.len(), 15);
    }

    #[test]
    fn follower_gear_journal_pool_uses_content_tuning_expansion() {
        let path = crate::test_data::world_db_path();
        let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let db2 = path.parent().unwrap().join("db2/active");
        let pools = read_dungeon_item_pools(&db, &db2).unwrap();
        let has_item = |map, item| {
            pools[&map]
                .iter()
                .any(|candidate| candidate.item_id == item)
        };
        assert!(has_item(2515, 193633), "Azure Leymor ring");
        assert!(
            pools[&2515]
                .iter()
                .any(|candidate| candidate.item_id == 193633
                    && candidate.source_map == 2515
                    && candidate.difficulty_mask == -1
                    && candidate.faction_mask == -1),
            "retain source availability"
        );
        assert!(has_item(2515, 193691), "Ruby item from the same expansion");
        assert!(
            !has_item(2515, 219294),
            "Rookery belongs to the next expansion"
        );
        assert!(has_item(2648, 219294), "Rookery item stays in its own tier");
        assert_eq!(pools[&2515], pools[&2521], "same-tier pool is shared");
        assert_ne!(
            pools[&2515], pools[&2662],
            "Dawnbreaker uses ContentTuning expansion, not LFG ExpansionLevel"
        );
    }
}
