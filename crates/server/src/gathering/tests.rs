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
        .insert_resource(crate::persistence::GameDatabase { db });
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
fn gathering_skinning_requires_looted_dead_corpse_and_uses_authored_loot() {
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
    assert!(loot.looters.values().flatten().any(|slot| matches!(
        slot,
        Some(LootContent::Item {
            item_id: 2318 | 2934,
            ..
        })
    )));
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
