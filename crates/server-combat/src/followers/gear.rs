//! Source dungeon gear candidates directly from the active Journal and LFG data.
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use rusqlite::Connection;
use serde::Deserialize;

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

/// Return actual unconditional Journal items from the same ContentTuning expansion.
/// Conditional/seasonal Journal rows need separate state and are not inferred.
pub fn read_dungeon_item_pools(
    db: &Connection,
    db2: &Path,
) -> Result<HashMap<u32, Vec<u32>>, String> {
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
    let mut by_tier: HashMap<i32, BTreeSet<u32>> = HashMap::new();
    for row in read_rows::<JournalItem>(db2, "JournalEncounterItem.csv")? {
        if row.season != 0 || row.world_state != 0 {
            continue;
        }
        let Some(tier) = encounters
            .get(&row.encounter)
            .and_then(|map| tiers.get(map))
        else {
            continue;
        };
        by_tier.entry(*tier).or_default().insert(row.item);
    }
    tiers
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
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::OpenFlags;

    #[test]
    fn follower_gear_journal_pool_uses_content_tuning_expansion() {
        let path = crate::test_data::world_db_path();
        let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let db2 = path.parent().unwrap().join("db2/active");
        let pools = read_dungeon_item_pools(&db, &db2).unwrap();
        assert!(pools[&2515].contains(&193633), "Azure Leymor ring");
        assert!(
            pools[&2515].contains(&193691),
            "Ruby item from the same expansion"
        );
        assert!(
            !pools[&2515].contains(&219294),
            "Rookery belongs to the next expansion"
        );
        assert!(
            pools[&2648].contains(&219294),
            "Rookery item stays in its own tier"
        );
        assert_eq!(pools[&2515], pools[&2521], "same-tier pool is shared");
        assert_ne!(
            pools[&2515], pools[&2662],
            "Dawnbreaker uses ContentTuning expansion, not LFG ExpansionLevel"
        );
    }
}
