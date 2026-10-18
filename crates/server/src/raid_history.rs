//! Capture first-kill credit at the authoritative scripted DONE boundary.
use crate::maps::InMap;
use bevy::prelude::*;

pub(crate) fn record_first_kill(world: &mut World, map: InMap, encounter: u32, difficulty: u32) {
    use crate::maps::store::InstanceType;
    use crate::persistence::{ClientId, GameDatabase};
    use ::server::raid_history_store::{Participant, persist_first_kills};
    use shared::components::GuildMembership;
    let store = world.resource::<crate::maps::MapStore>();
    let is_raid = store
        .map(map.map_id)
        .is_some_and(|m| m.instance_type == InstanceType::Raid);
    let is_authored = store
        .dungeon_encounter(encounter, map.map_id, difficulty)
        .is_some();
    if !is_raid || !is_authored {
        return;
    }
    let mut party: Vec<_> = crate::scripted_ai::instance_script::players_in_map(world, map)
        .into_iter()
        .map(|player| {
            let id = world
                .get::<ClientId>(player)
                .expect("persistent raid player identity")
                .0;
            let guild = world.get::<GuildMembership>(player);
            Participant {
                character_id: id.to_string(),
                guild_id: guild.map(|g| g.guild_id.to_string()),
                guild_name: guild.map(|g| g.guild_name.clone()),
            }
        })
        .collect();
    party.sort_by_key(|p| {
        p.character_id
            .parse::<u64>()
            .expect("server-generated character id")
    });
    let unix = world.resource::<crate::maps::ServerClock>().unix();
    let txn = world
        .resource::<GameDatabase>()
        .db
        .begin_write()
        .expect("begin raid first-kill transaction");
    persist_first_kills(&txn, map.map_id, difficulty, encounter, unix, &party)
        .expect("persist raid first kills");
    txn.commit().expect("commit raid first-kill transaction");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::{InstanceManager, ServerClock};
    use crate::persistence::ClientId;
    use crate::scripted_ai::instance_script::{
        BossInfo, EncounterState, InstanceScript, InstanceScripts, set_boss_state,
    };
    use redb::{ReadableDatabase, ReadableTableMetadata, TableDefinition};
    use shared::components::{GuildMembership, Player};

    #[test]
    fn raid_first_kill_done_boundary_preserves_guild_credit_and_first_timestamp() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut h = crate::bank_test_harness::harness_at(file.path(), "FbRaidOwner").unwrap();
        let conn = rusqlite::Connection::open_with_flags(
            std::env::var("SITE_DATA_WORLD_COPY").expect("read-only private world copy"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let store = crate::maps::store::load_map_store(&conn).unwrap();
        let mut manager = InstanceManager::default();
        let map = manager
            .create_map(
                &store,
                409,
                crate::maps::instances::InstanceEntrant {
                    character_id: h.character_id,
                    group: None,
                    leader: h.character_id,
                    difficulty_id: 9,
                    raid_difficulty: crate::maps::RaidDifficulty {
                        primary: 14,
                        legacy: 9,
                    },
                    raid_group: true,
                },
                crate::maps::instances::Now {
                    unix: 1000,
                    elapsed: 0.0,
                },
            )
            .unwrap()
            .map;
        h.app.world_mut().insert_resource(store);
        h.app.world_mut().insert_resource(manager);
        h.app.world_mut().insert_resource(ServerClock {
            offset_secs: 0,
            fixed_unix: Some(1000),
        });
        h.app.world_mut().entity_mut(h.player).insert((
            map,
            GuildMembership {
                guild_id: 7,
                guild_name: "Proof Guild".into(),
            },
        ));
        let mut players = vec![h.player];
        let identity = h.app.world().get::<Player>(h.player).unwrap().clone();
        for i in 0..4 {
            let entity = h
                .app
                .world_mut()
                .spawn((
                    Player {
                        name: format!("FbRaider{i}"),
                        ..identity.clone()
                    },
                    map,
                    ClientId(100 + i),
                ))
                .id();
            if i < 3 {
                h.app
                    .world_mut()
                    .entity_mut(entity)
                    .insert(GuildMembership {
                        guild_id: 7,
                        guild_name: "Proof Guild".into(),
                    });
            }
            players.push(entity);
        }
        // Same-guild player elsewhere must not affect 4/5 threshold.
        h.app.world_mut().spawn((
            identity,
            InMap {
                map_id: 409,
                instance_id: 999,
            },
            ClientId(999),
            GuildMembership {
                guild_id: 7,
                guild_name: "Proof Guild".into(),
            },
        ));
        h.app
            .world_mut()
            .insert_resource(InstanceScripts(std::collections::HashMap::from([(
                map,
                InstanceScript {
                    name: "private_raid_fixture",
                    bosses: vec![BossInfo {
                        state: EncounterState::NotStarted,
                        encounter_id: 663,
                    }],
                    boss_creatures: vec![],
                    handled_area_triggers: Default::default(),
                    data: Default::default(),
                    ramparts_sentry_deaths: 0,
                },
            )])));
        assert!(set_boss_state(
            h.app.world_mut(),
            map,
            0,
            EncounterState::InProgress
        ));
        assert!(set_boss_state(
            h.app.world_mut(),
            map,
            0,
            EncounterState::NotStarted
        ));
        let table_def = TableDefinition::<&str, &[u8]>::new("raid_first_kills_v1");
        assert!(matches!(
            h.db().begin_read().unwrap().open_table(table_def),
            Err(redb::TableError::TableDoesNotExist(_))
        ));
        assert!(set_boss_state(
            h.app.world_mut(),
            map,
            0,
            EncounterState::Done
        ));
        let first = {
            let txn = h.db().begin_read().unwrap();
            let table = txn.open_table(table_def).unwrap();
            assert_eq!(table.len().unwrap(), 6);
            let row = table.get("guild:7:409:9:663").unwrap().unwrap();
            let value: serde_json::Value = serde_json::from_slice(row.value()).unwrap();
            assert_eq!(value["first_kill_at"], 1000);
            assert_eq!(value["participants"].as_array().unwrap().len(), 5);
            assert_eq!(value["guild_group_credit"], true);
            row.value().to_vec()
        };
        h.app.world_mut().resource_mut::<ServerClock>().fixed_unix = Some(2000);
        h.app
            .world_mut()
            .entity_mut(players[1])
            .remove::<GuildMembership>();
        record_first_kill(h.app.world_mut(), map, 663, 9);
        assert!(!set_boss_state(
            h.app.world_mut(),
            map,
            0,
            EncounterState::Done
        ));
        drop(h);
        let db = redb::ReadOnlyDatabase::open(file.path()).unwrap();
        let txn = db.begin_read().unwrap();
        let table = txn.open_table(table_def).unwrap();
        assert_eq!(
            table.get("guild:7:409:9:663").unwrap().unwrap().value(),
            first
        );
    }
}
