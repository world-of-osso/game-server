//! Retail raid selections (TC a352b1fa MiscHandler::HandleSetRaidDifficultyOpcode).
//! Lock creation, boss credit, persistence and extension stay in the existing instance manager.
use super::difficulty::{drain, now, send_to_player};
use super::store::InstanceType;
use super::{InMap, InstanceManager, MapStore};
use crate::group::GroupRegistry;
use crate::persistence::{ClientId, GameDatabase};
use bevy::prelude::*;
use redb::{Database, ReadableDatabase, TableDefinition};
use shared::components::Player;
use shared::protocol::{RaidDifficultySet, SetRaidDifficulty};

const SAVED_RAID_DIFFICULTY: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_raid_difficulty");

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, bitcode::Encode, bitcode::Decode)]
pub struct RaidDifficulty {
    pub primary: u32,
    pub legacy: u32,
}
impl Default for RaidDifficulty {
    fn default() -> Self {
        Self {
            primary: 14,
            legacy: 3,
        }
    }
}

pub(crate) fn load(db: &Database, character: u64) -> RaidDifficulty {
    let txn = db.begin_read().expect("read raid difficulty");
    let Ok(table) = txn.open_table(SAVED_RAID_DIFFICULTY) else {
        return RaidDifficulty::default();
    };
    table
        .get(character)
        .expect("read character raid difficulty")
        .map_or_else(RaidDifficulty::default, |v| {
            bitcode::decode(v.value()).expect("decode raid difficulty")
        })
}

fn save(db: &Database, character: u64, difficulty: RaidDifficulty) {
    let txn = db.begin_write().expect("write raid difficulty");
    {
        txn.open_table(SAVED_RAID_DIFFICULTY)
            .expect("open raid difficulty")
            .insert(character, bitcode::encode(&difficulty).as_slice())
            .expect("save raid difficulty");
    }
    crate::persistence_commit::commit(txn).expect("commit raid difficulty");
}

#[derive(Resource, Default)]
pub(crate) struct RaidChanges(pub Vec<(u64, RaidDifficulty)>);

pub(crate) fn apply_changes(world: &mut World) {
    let changes = std::mem::take(&mut world.resource_mut::<RaidChanges>().0);
    for (character, selection) in changes {
        apply(world, character, selection);
    }
}

pub(crate) fn receive(world: &mut World) {
    for (player, character, request) in drain::<SetRaidDifficulty>(world) {
        set(world, player, character, request);
    }
}

pub(crate) fn set(world: &mut World, player: Entity, character: u64, request: SetRaidDifficulty) {
    let store = world.resource::<MapStore>();
    let Some(entry) = store.difficulty_entries.get(&request.difficulty_id) else {
        return;
    };
    if entry.instance_type != InstanceType::Raid
        || !entry.can_select()
        || entry.is_legacy() != request.legacy
    {
        return;
    }
    if world
        .get::<InMap>(player)
        .is_some_and(|m| store.map(m.map_id).is_some_and(|e| e.is_dungeon()))
    {
        return;
    }
    let mut selection = world
        .get::<RaidDifficulty>(player)
        .copied()
        .unwrap_or_default();
    if request.legacy {
        selection.legacy = request.difficulty_id;
    } else {
        selection.primary = request.difficulty_id;
    }
    let members = if world
        .get_resource::<GroupRegistry>()
        .is_some_and(|g| g.group_id_of(character).is_some())
    {
        let Some(members) = world
            .resource_mut::<GroupRegistry>()
            .set_raid_difficulty(character, selection)
        else {
            return;
        };
        members
    } else {
        vec![character]
    };
    if members.is_empty() {
        return;
    }
    let entrant = super::transfer::entrant_of(world, player);
    let now = now(world);
    world
        .resource_mut::<InstanceManager>()
        .reset_instances(entrant, true, now);
    for member in members {
        apply(world, member, selection);
    }
}

pub(crate) fn apply(world: &mut World, character: u64, selection: RaidDifficulty) {
    if let Some(db) = world.get_resource::<GameDatabase>() {
        save(&db.db, character, selection);
    }
    let player = world
        .query::<(Entity, &ClientId)>()
        .iter(world)
        .find_map(|(e, id)| (id.0 == character).then_some(e));
    if let Some(player) = player {
        world.entity_mut(player).insert(selection);
    }
}

pub(crate) fn send(world: &mut World) {
    let changes: Vec<_> = world
        .query_filtered::<(Entity, &RaidDifficulty), (With<Player>, Changed<RaidDifficulty>)>()
        .iter(world)
        .map(|(e, d)| (e, *d))
        .collect();
    for (player, d) in changes {
        send_to_player(
            world,
            player,
            RaidDifficultySet {
                difficulty_id: d.primary,
                legacy_difficulty_id: d.legacy,
            },
        );
    }
}

/// Real creature death boundary, shared by melee, spells and periodic damage.
/// Only unambiguous authored single-creature encounters receive generic credit.
pub(crate) fn record_creature_death(world: &mut World, creature: Entity) {
    let Some(map) = world.get::<InMap>(creature).copied() else {
        return;
    };
    let Some(npc) = world.get::<shared::components::Npc>(creature) else {
        return;
    };
    let Some(store) = world.get_resource::<MapStore>() else {
        return;
    };
    let Some(&bit) = store
        .raid_creature_encounters
        .get(&(map.map_id, npc.template_id))
    else {
        return;
    };
    let Some(instances) = world.get_resource::<InstanceManager>() else {
        return;
    };
    let difficulty = instances.copy_difficulty(map).map(|d| d.difficulty_id);
    let Some(difficulty) = difficulty else { return };
    let Some(encounter_id) = store
        .encounters_for_map(map.map_id, difficulty)
        .iter()
        .find(|e| e.bit == bit)
        .map(|e| e.id)
    else {
        return;
    };
    let completed = instances.completed_encounters(map);
    if completed & (1 << bit) != 0 {
        return;
    }
    let players: Vec<_> = crate::scripted_ai::instance_script::players_in_map(world, map)
        .into_iter()
        .filter_map(|entity| Some((entity, world.get::<ClientId>(entity)?.0)))
        .collect();
    let unix = now(world).unix;
    let saved = world
        .resource_mut::<InstanceManager>()
        .update_instance_lock(map, &players, completed | (1 << bit), Some(bit), unix);
    super::difficulty::send_instance_saved(world, &saved);
    crate::raid_history::record_first_kill(world, map, encounter_id, difficulty);
    for (entity, character) in players {
        super::difficulty::send_raid_info(world, entity, character);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raid_entry_separate_primary_legacy_selections_survive_database_reopen() {
        let file = tempfile::NamedTempFile::new().unwrap();
        {
            let db = Database::create(file.path()).unwrap();
            save(
                &db,
                71,
                RaidDifficulty {
                    primary: 15,
                    legacy: 6,
                },
            );
            save(
                &db,
                72,
                RaidDifficulty {
                    primary: 16,
                    legacy: 3,
                },
            );
        }
        let db = Database::open(file.path()).unwrap();
        assert_eq!(
            load(&db, 71),
            RaidDifficulty {
                primary: 15,
                legacy: 6
            }
        );
        assert_eq!(
            load(&db, 72),
            RaidDifficulty {
                primary: 16,
                legacy: 3
            }
        );
        assert_eq!(load(&db, 73), RaidDifficulty::default());
    }
}
