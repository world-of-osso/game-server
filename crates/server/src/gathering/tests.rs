use super::*;
use crate::spell_cast::test_harness::{Harness, harness_with_plugins};
use shared::casting::CastState;
use shared::profession::ProfessionSkillLine;
use shared::protocol::LootContent;

fn setup(skill: u32) -> (Harness, tempfile::TempDir) {
    let path = crate::test_data::world_db_path();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let mut h = harness_with_plugins(Vec::new(), &[265819, 265837, 8613], 2, false, |app| {
        app.add_plugins(GatheringPlugin);
        app.insert_resource(load_data(&conn).unwrap());
        app.insert_resource(crate::profession::load_profession_data(&conn).unwrap());
        app.insert_resource(crate::loot::load_loot_tables(&conn).unwrap());
        app.insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
        app.insert_resource(crate::merchant::load_vendor_catalog(&conn).unwrap());
        app.insert_resource(crate::game_objects::load_game_object_spawns(&conn).unwrap());
    })
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("gatherer.redb")).unwrap();
    crate::persistence::create_tables(&db);
    let (id, _) = crate::persistence::create_character(&db, 1, "Gatherer", 1, 2).unwrap();
    h.app
        .insert_resource(crate::persistence::GameDatabase::new(db));
    h.app.world_mut().entity_mut(h.caster).insert((
        crate::persistence::ClientId(id),
        CombatStatus(false),
        PlayerProfessions {
            lines: vec![ProfessionSkillLine {
                skill_line: skill,
                step: 1,
                rank: 1,
                max_rank: 300,
            }],
            spells: Vec::new(),
        },
    ));
    (h, directory)
}

fn nearby_node(h: &mut Harness, entry: u32) -> Entity {
    let mut spawn = h
        .app
        .world()
        .resource::<crate::game_objects::GameObjectSpawns>()
        .0
        .iter()
        .find(|spawn| spawn.info.entry == entry && spawn.map_id == 0)
        .unwrap()
        .clone();
    // Keep real identity, model/box, timer and loot; only place the test actor at it.
    let position = crate::game_objects::game_object_bundle(&spawn).0;
    *h.app.world_mut().get_mut::<Position>(h.caster).unwrap() = position;
    spawn.orientation = 0.0;
    spawn_node(h.app.world_mut(), &spawn, InMap::continent(0)).unwrap()
}

#[test]
fn gathering_herb_and_ore_cast_then_roll_authored_materials_and_raise_skill() {
    for (entry, skill, material) in [(1617, 2556, 765), (1731, 2572, 2770)] {
        let (mut h, _directory) = setup(skill);
        let target = nearby_node(&mut h, entry);
        use_resource(h.app.world_mut(), h.caster, target);
        assert!(
            h.app.world().get::<CastState>(h.caster).is_some(),
            "gathering must use a real cast"
        );
        assert_eq!(
            h.app
                .world()
                .get::<PlayerProfessions>(h.caster)
                .unwrap()
                .rank(skill),
            1
        );
        h.advance(200);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert_eq!(
            h.app
                .world()
                .get::<PlayerProfessions>(h.caster)
                .unwrap()
                .rank(skill),
            2
        );
        let loot = h
            .app
            .world()
            .get::<crate::loot::CorpseLoot>(h.caster)
            .unwrap();
        assert!(loot.looters.values().flatten().any(
            |slot| matches!(slot,Some(LootContent::Item { item_id,.. }) if *item_id==material)
        ));
        use_resource(h.app.world_mut(), h.caster, target);
        h.advance(1);
        assert_eq!(
            h.app
                .world()
                .get::<PlayerProfessions>(h.caster)
                .unwrap()
                .rank(skill),
            2,
            "same character cannot gather the same node twice"
        );
    }
}

#[test]
fn gathering_missing_skill_and_movement_interrupt_do_not_roll_or_raise_skill() {
    let (mut h, _directory) = setup(2556);
    let target = nearby_node(&mut h, 1731);
    use_resource(h.app.world_mut(), h.caster, target);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    let herb = nearby_node(&mut h, 1617);
    use_resource(h.app.world_mut(), h.caster, herb);
    assert!(h.app.world().get::<CastState>(h.caster).is_some());
    h.app
        .world_mut()
        .get_mut::<crate::networking::Moving>(h.caster)
        .unwrap()
        .0 = true;
    h.advance(200);
    assert!(
        h.app
            .world()
            .get::<crate::loot::CorpseLoot>(h.caster)
            .is_none()
    );
    assert_eq!(
        h.app
            .world()
            .get::<PlayerProfessions>(h.caster)
            .unwrap()
            .rank(2556),
        1
    );
}

#[test]
#[ignore = "RED: one source roll exceeds the 1/1000 false-failure budget"]
fn gathering_skinning_requires_looted_dead_corpse_and_uses_authored_loot() {
    const GATHERS: i32 = 1;
    let mut found_material = false;
    for _ in 0..GATHERS {
        // A fresh fixture supplies a fresh corpse and no pending loot from another roll.
        let (mut h, _directory) = setup(2564);
        h.app.world_mut().entity_mut(h.npc).insert((
            Npc {
                template_id: 525,
                name: "Mangy Wolf".into(),
            },
            Health {
                current: 0.0,
                max: 100.0,
            },
            crate::combat::Corpse { remaining: 60.0 },
            crate::loot::CorpseLoot::default(),
            GatherableResource {
                parent_skill: data::SKINNING,
            },
        ));
        let position = *h.app.world().get::<Position>(h.npc).unwrap();
        *h.app.world_mut().get_mut::<Position>(h.caster).unwrap() = position;
        use_resource(h.app.world_mut(), h.caster, h.npc);
        assert!(h.app.world().get::<CastState>(h.caster).is_some());
        h.advance(200);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert_eq!(
            h.app
                .world()
                .get::<PlayerProfessions>(h.caster)
                .unwrap()
                .rank(2564),
            2
        );
        assert!(h.app.world().get::<Skinned>(h.npc).is_some());
        assert!(h.app.world().get::<GatherableResource>(h.npc).is_none());
        let loot = h
            .app
            .world()
            .get::<crate::loot::CorpseLoot>(h.caster)
            .unwrap();
        for slot in loot.looters.values().flatten() {
            match slot {
                None => {}
                Some(LootContent::Item {
                    item_id: 2318 | 2934,
                    ..
                }) => found_material = true,
                other => panic!("unexpected source skinning output: {other:?}"),
            }
        }
        // Derive the false-failure bound from the loaded explicit source group.
        let skin = skin_template(h.app.world(), h.npc).unwrap();
        let tables = h.app.world().resource::<crate::loot::LootTables>();
        let material_probability = tables.skinning[&skin.loot].groups[0]
            .iter()
            .filter(|row| matches!(row.item, 2318 | 2934))
            .map(|row| f64::from(row.chance) / 100.0)
            .sum::<f64>();
        assert!((0.0..=1.0).contains(&material_probability));
        let false_failure = (1.0 - material_probability).powi(GATHERS);
        assert!(
            false_failure < 1.0 / 1000.0,
            "{GATHERS} real gathers have source-derived false-failure probability {false_failure}"
        );
    }
    assert!(
        found_material,
        "independent fresh corpses must yield at least one authored material"
    );
}

#[test]
fn gathering_authored_thresholds_are_not_recipe_trivial_ranks() {
    let progress = crate::profession::GatherProgress::node(2556, 0, 50, 100);
    assert_eq!(
        [1, 50, 75, 100].map(|rank| progress.chance_permille(rank)),
        [1000, 750, 250, 0]
    );
}

#[test]
fn gathering_pending_materials_survive_a_second_resource_request() {
    let (mut h, _directory) = setup(2556);
    let herb = nearby_node(&mut h, 1617);
    use_resource(h.app.world_mut(), h.caster, herb);
    h.advance(200);
    let original = h
        .app
        .world()
        .get::<crate::loot::CorpseLoot>(h.caster)
        .unwrap()
        .clone();
    h.app
        .world_mut()
        .get_mut::<PlayerProfessions>(h.caster)
        .unwrap()
        .lines
        .push(ProfessionSkillLine {
            skill_line: 2572,
            step: 1,
            rank: 1,
            max_rank: 300,
        });
    let ore = nearby_node(&mut h, 1731);
    use_resource(h.app.world_mut(), h.caster, ore);
    assert!(
        h.app.world().get::<CastState>(h.caster).is_none(),
        "pending gathered loot must not be replaced"
    );
    h.advance(100);
    assert_eq!(
        *h.app
            .world()
            .get::<crate::loot::CorpseLoot>(h.caster)
            .unwrap(),
        original
    );
    assert_eq!(
        h.app
            .world()
            .get::<PlayerProfessions>(h.caster)
            .unwrap()
            .rank(2572),
        1
    );
}

#[test]
fn gathering_underivable_authored_lock_is_a_gap_not_a_pool_update_panic() {
    let (mut h, _directory) = setup(2572);
    // TDB authored gathering-node entry176116 has no local DB2 gathering ability.
    let spawn = h
        .app
        .world()
        .resource::<crate::game_objects::GameObjectSpawns>()
        .0
        .iter()
        .find(|spawn| spawn.info.entry == 176116)
        .unwrap()
        .clone();
    let before = h
        .app
        .world_mut()
        .query::<&GameObjectInfo>()
        .iter(h.app.world())
        .count();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = spawn_node(h.app.world_mut(), &spawn, InMap::continent(spawn.map_id));
    }));
    assert!(
        result.is_ok(),
        "unavailable authored mapping must not crash a shared pool update"
    );
    let after = h
        .app
        .world_mut()
        .query::<&GameObjectInfo>()
        .iter(h.app.world())
        .count();
    assert_eq!(
        before, after,
        "underivable node must not acquire a fabricated gathering category"
    );
}

#[test]
fn gathering_skill_up_survives_redb_close_and_reopen() {
    let (mut h, directory) = setup(2556);
    h.app
        .add_systems(PostUpdate, crate::profession::save_and_send_professions);
    let target = nearby_node(&mut h, 1617);
    use_resource(h.app.world_mut(), h.caster, target);
    h.advance(200);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerProfessions>(h.caster)
            .unwrap()
            .rank(2556),
        2
    );
    drop(h);
    let db = redb::Database::open(directory.path().join("gatherer.redb")).unwrap();
    let restored = crate::persistence::load_professions(&db, 1);
    assert_eq!(restored.rank(2556), 2);
}

#[test]
fn gathering_depletes_then_respawns_at_the_authored_timer_and_position() {
    use std::time::Duration;
    let (mut h, _directory) = setup(2556);
    let path = crate::test_data::world_db_path();
    h.app
        .insert_resource(crate::world_data::load_world_data(&path));
    let target = nearby_node(&mut h, 1617);
    let position = *h.app.world().get::<Position>(target).unwrap();
    let guid = h.app.world().get::<NodeSource>(target).unwrap().0.guid;
    let delay = h.app.world().resource::<GatheringData>().nodes[&1617].despawn_delay;
    let respawn = h.app.world().resource::<GatheringData>().respawn_seconds[&guid];
    use_resource(h.app.world_mut(), h.caster, target);
    h.advance(200);
    assert!(
        h.app.world().get_entity(target).is_ok(),
        "Retail depletion delay retains the shared node"
    );
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(Duration::from_secs(1000));
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs(1),
        ));
    for _ in 0..(delay.ceil() as usize + 1) {
        if h.app.world().get_entity(target).is_err() {
            break;
        }
        h.advance(1);
    }
    assert!(h.app.world().get_entity(target).is_err());
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs_f32(respawn - 2.0),
        ));
    h.advance(1);
    assert_eq!(
        h.app
            .world_mut()
            .query::<&GameObjectInfo>()
            .iter(h.app.world())
            .filter(|info| info.entry == 1617)
            .count(),
        0,
        "no early respawn"
    );
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs(3),
        ));
    h.advance(1);
    let nodes = h
        .app
        .world_mut()
        .query::<(&GameObjectInfo, &Position, &InMap)>()
        .iter(h.app.world())
        .filter(|(info, _, _)| info.entry == 1617)
        .map(|(_, position, map)| (*position, *map))
        .collect::<Vec<_>>();
    assert_eq!(nodes, vec![(position, InMap::continent(0))]);
}

#[test]
fn gathering_authored_pool_selection_and_repick_respect_member_limits() {
    use rand::{SeedableRng, rngs::StdRng};
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let pools = crate::spawn_pools::SpawnPools::load(&conn).unwrap();
    let mut rng = StdRng::seed_from_u64(5720);
    let (mut selected, _) = pools.init_map(0, &mut rng);
    let members=conn.prepare("SELECT p.spawnId,p.poolSpawnId,t.max_limit FROM content_pool_members p JOIN content_gameobject g ON g.guid=p.spawnId JOIN content_pool_template t ON t.entry=p.poolSpawnId WHERE p.type=1 AND g.map=0 ORDER BY p.poolSpawnId,p.spawnId").unwrap()
        .query_map([],|row|Ok((row.get::<_,u32>(0)?,row.get::<_,u32>(1)?,row.get::<_,usize>(2)?))).unwrap()
        .collect::<rusqlite::Result<Vec<_>>>().unwrap();
    let (guid, pool, limit) = members
        .iter()
        .copied()
        .find(|(guid, _, _)| selected.gameobject_spawned(*guid))
        .expect("authored map0 resource pool must select a live member");
    let count = |state: &crate::spawn_pools::SpawnedPoolData| {
        members
            .iter()
            .filter(|(guid, owner, _)| *owner == pool && state.gameobject_spawned(*guid))
            .count()
    };
    assert!(count(&selected) > 0 && count(&selected) <= limit);
    let changes = pools.respawn_gameobject(&mut selected, guid, &mut rng);
    assert!(changes.gameobjects_despawned.contains(&guid));
    assert!(!changes.gameobjects_spawned.is_empty());
    assert!(
        changes
            .gameobjects_spawned
            .iter()
            .all(|guid| selected.gameobject_spawned(*guid))
    );
    assert!(count(&selected) <= limit);
}

#[test]
fn gathering_skin_marker_waits_for_completed_normal_loot_roll() {
    for has_drop in [true, false] {
        let (mut h, _directory) = setup(2564);
        let character_id = h
            .app
            .world()
            .get::<crate::persistence::ClientId>(h.caster)
            .unwrap()
            .0;
        h.app.world_mut().entity_mut(h.npc).insert((
            Npc {
                template_id: 525,
                name: "Mangy Wolf".into(),
            },
            Health {
                current: 0.0,
                max: 100.0,
            },
            crate::combat::Corpse { remaining: 60.0 },
        ));
        // Production ordering: death is visible before the deferred normal loot roll.
        h.advance(1);
        assert!(
            h.app.world().get::<GatherableResource>(h.npc).is_none(),
            "unrolled corpse must not advertise Skinning"
        );
        use_resource(h.app.world_mut(), h.caster, h.npc);
        assert!(
            h.app.world().get::<CastState>(h.caster).is_none(),
            "a direct Skinning request must also wait for the normal loot roll"
        );
        let looters = if has_drop {
            std::collections::HashMap::from([(
                character_id,
                vec![Some(LootContent::Money { copper: 1 })],
            )])
        } else {
            std::collections::HashMap::new()
        };
        h.app
            .world_mut()
            .entity_mut(h.npc)
            .insert(crate::loot::CorpseLoot { looters });
        h.advance(1);
        if has_drop {
            assert!(
                h.app.world().get::<GatherableResource>(h.npc).is_none(),
                "normal corpse loot precedes Skinning"
            );
            h.app
                .world_mut()
                .get_mut::<crate::loot::CorpseLoot>(h.npc)
                .unwrap()
                .looters
                .get_mut(&character_id)
                .unwrap()[0] = None;
            h.advance(1);
        }
        assert_eq!(
            h.app.world().get::<GatherableResource>(h.npc),
            Some(&GatherableResource {
                parent_skill: data::SKINNING
            })
        );
    }
}
