//! Capture first-kill credit at the authoritative scripted DONE boundary.
use crate::maps::InMap;
use bevy::prelude::*;

pub(crate) fn record_first_kill(
    _world: &mut World,
    _map: InMap,
    _encounter: u32,
    _difficulty: u32,
) {
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
                    boss_creatures: &[],
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
