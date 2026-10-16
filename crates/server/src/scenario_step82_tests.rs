//! Native scenario58 progression; controlled NPC identities are not Researchers content.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness_with_plugins, spawn_npc};
use shared::components::Health;

const RESEARCH_CRITERION: u32 = 60015;
const EVENT_SPELL: u32 = 123744;
const DAMAGE_SPELL: u32 = 686;

fn create_arena_fixture() -> (Harness, tempfile::NamedTempFile, u64, u64) {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 17, "Stageguard", 1, 9).unwrap();
    let (foreign, _) = persistence::create_character(&db, 18, "Otherstage", 1, 9).unwrap();
    let mut h = harness_with_plugins(vec![], &[DAMAGE_SPELL, EVENT_SPELL], 9, false, |app| {
        app.add_plugins(AchievementPlugin);
    })
    .unwrap();
    let map = bind_authored_arena_run(&mut h, &conn, character);
    h.app.insert_resource(GameDatabase { db });
    h.app.world_mut().entity_mut(h.caster).insert((
        map,
        persistence::ClientId(character),
        PlayerAchievementState::default(),
    ));
    h.app
        .world_mut()
        .get_mut::<InWorld>(h.connection)
        .unwrap()
        .character_id = character;
    h.advance(1);
    assert_eq!(snapshot(h.app.world(), map).scenario.unwrap().step_id, 138);
    (h, file, character, foreign)
}

fn assert_arena_admission_rejected(store: &crate::maps::MapStore, character: u64) {
    let mut instances = crate::maps::InstanceManager::default();
    let admission = instances.create_map(
        store,
        1031,
        crate::maps::instances::InstanceEntrant {
            character_id: character,
            leader: character,
            difficulty_id: 11,
            ..Default::default()
        },
        crate::maps::instances::Now {
            elapsed: 0.0,
            unix: 1791763200,
        },
    );
    assert!(matches!(
        admission,
        Err(shared::protocol::TransferAbortReason::MapNotAllowed)
    ));
}

fn bind_authored_arena_run(h: &mut Harness, conn: &Connection, character: u64) -> InMap {
    let catalog = load_achievement_catalog(conn).unwrap();
    h.app
        .insert_resource(load_scenarios(conn, &catalog).unwrap());
    h.app.insert_resource(catalog);
    let store = crate::maps::store::load_map_store(conn).unwrap();
    assert_arena_admission_rejected(&store, character);
    // Native scenario admission is not implemented. Explicitly bind only the authored
    // run for component-level event proof; no map-entry or research-content claim.
    let map = InMap {
        map_id: 1031,
        instance_id: 7,
    };
    h.app
        .world_mut()
        .resource_mut::<ScenarioRuns>()
        .0
        .insert(map, (58, ScenarioRun::default()));
    h.app.insert_resource(store);
    h.app
        .insert_resource(crate::maps::InstanceManager::default());
    map
}

fn kill_controlled_npc(h: &mut Harness, entry: u32) {
    let map = *h.app.world().get::<InMap>(h.caster).unwrap();
    let target = spawn_npc(&mut h.app, 3.0);
    h.app.world_mut().entity_mut(target).insert((
        map,
        Npc {
            template_id: entry,
            name: "Controlled stage-bound victim".into(),
        },
        Health {
            current: 1.0,
            max: 1.0,
        },
    ));
    h.cast(DAMAGE_SPELL, Some(target));
    h.advance(160);
    assert_eq!(h.health(target), 0.0, "real lethal Shadow Bolt");
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(target)
            .is_some()
    );
}

fn finish_authored_stage(h: &mut Harness, entry: u32, next_step: u32) {
    kill_controlled_npc(h, entry);
    assert!(
        crate::spell_cast::try_cast_triggered(h.app.world_mut(), h.caster, EVENT_SPELL, None, 0),
        "authored SendEvent32548 cast accepted"
    );
    h.advance(3);
    let map = *h.app.world().get::<InMap>(h.caster).unwrap();
    assert_eq!(
        snapshot(h.app.world(), map).scenario.unwrap().step_id,
        next_step
    );
}

fn research_progress(h: &Harness) -> Option<u64> {
    h.app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap()
        .criteria_progress
        .get(&RESEARCH_CRITERION)
        .copied()
}

#[test]
fn achievements_criteria_scenario_step82_native_stage_and_lethal_kill_persist() {
    let (mut h, file, character, foreign) = create_arena_fixture();
    kill_controlled_npc(&mut h, 204799);
    assert_eq!(research_progress(&h), None, "stage1 cannot supply stage3");
    finish_authored_stage(&mut h, 63311, 139);
    finish_authored_stage(&mut h, 63312, 140);
    assert_eq!(
        research_progress(&h),
        None,
        "stage transitions alone are not research kills"
    );
    assert!(
        !crate::spell_cast::try_cast_triggered(
            h.app.world_mut(),
            h.caster,
            DAMAGE_SPELL,
            Some(h.npc),
            255
        ),
        "refused cast cannot supply a stage-qualified kill"
    );
    h.advance(2);
    assert_eq!(research_progress(&h), None);
    let catalog = h
        .app
        .world_mut()
        .remove_resource::<ScenarioCatalog>()
        .unwrap();
    assert_eq!(
        player_conditions(h.app.world(), h.caster)
            .player
            .unwrap()
            .scenario_step,
        None,
        "missing scenario catalog is unknown"
    );
    h.app.insert_resource(catalog);
    kill_controlled_npc(&mut h, 204799);
    assert_eq!(
        research_progress(&h),
        Some(1),
        "authored step140 OrderIndex2 qualifies required stage3"
    );
    h.advance(3);
    assert_eq!(
        research_progress(&h),
        Some(1),
        "dead and idle updates cannot replay a kill"
    );
    let map = *h.app.world().get::<InMap>(h.caster).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(InMap::continent(0));
    h.advance(1);
    kill_controlled_npc(&mut h, 204799);
    assert_eq!(
        research_progress(&h),
        Some(1),
        "outside the scenario has no current step authority"
    );
    h.app.world_mut().entity_mut(h.caster).insert(map);
    h.advance(1);
    assert_eq!(snapshot(h.app.world(), map).scenario.unwrap().step_id, 140);
    let unbound_copy = InMap {
        map_id: map.map_id,
        instance_id: map.instance_id + 1,
    };
    h.app.world_mut().entity_mut(h.caster).insert(unbound_copy);
    h.advance(1);
    kill_controlled_npc(&mut h, 204799);
    assert_eq!(
        research_progress(&h),
        Some(1),
        "same map in another copy has no borrowed stage"
    );
    h.app.world_mut().entity_mut(h.caster).insert(map);
    h.advance(1);
    kill_controlled_npc(&mut h, 204071);
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&60017),
        "stage3 cannot supply stage5"
    );
    finish_authored_stage(&mut h, 63313, 141);
    finish_authored_stage(&mut h, 63318, 142);
    kill_controlled_npc(&mut h, 204071);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&60017),
        Some(&1),
        "authored step142 OrderIndex4 supplies stage5"
    );
    finish_authored_stage(&mut h, 64894, 0);
    assert!(snapshot(h.app.world(), map).scenario.unwrap().completed);
    kill_controlled_npc(&mut h, 204799);
    assert_eq!(
        research_progress(&h),
        Some(1),
        "finished scenario has no current stage"
    );
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&db, character)
            .criteria_progress
            .get(&RESEARCH_CRITERION),
        Some(&1)
    );
    assert_eq!(
        persistence::load_achievements(&db, character)
            .criteria_progress
            .get(&60017),
        Some(&1)
    );
    assert!(
        !persistence::load_achievements(&db, foreign)
            .criteria_progress
            .contains_key(&RESEARCH_CRITERION)
    );
}
