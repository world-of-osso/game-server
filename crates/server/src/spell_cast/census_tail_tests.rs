//! Native70448 effect hits through the ordinary production dispatch boundary.
use super::test_harness::*;
use crate::persistence::{self, ClientId, GameDatabase};
use bevy::prelude::*;
use shared::components::{Health, Mana, PowerType};

const DREAMWALK: u32 = 193_753;
const DREAMWALK_TRACKING_QUEST: u32 = 39_174;
const FEROCIOUS_BITE: u32 = 22_568;
const CAT_FORM: u32 = 768;
const WILT: u32 = 23_772;

/// Resolve one native effect after target selection, without unrelated teleport siblings.
fn hit_native(h: &mut Harness, spell_id: u32, effect_index: u8, target: Entity) {
    let spell = world_db().unwrap().store.get(spell_id).unwrap();
    let index = spell
        .effects
        .iter()
        .position(|effect| effect.index == effect_index)
        .unwrap();
    let plan = vec![super::PlannedEffect {
        index,
        units: vec![target],
        dest: None,
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }];
    super::resolve_effects(h.app.world_mut(), h.caster, spell, &plan, 0);
}

fn quest_fixture() -> (Harness, tempfile::TempDir, u64) {
    let mut h = harness(Vec::new(), &[DREAMWALK], 11).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("quest.redb")).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "CensusTail", 4, 11).unwrap();
    h.app.insert_resource(GameDatabase::new(db));
    h.app.insert_resource(
        crate::quest_data::load_quest_db(&crate::test_data::world_db_path()).unwrap(),
    );
    h.app.init_resource::<crate::quest::QuestDirty>();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ClientId(character));
    (h, directory, character)
}

fn burn_fixture(cat: bool) -> Harness {
    let mut h = harness(Vec::new(), &[FEROCIOUS_BITE, CAT_FORM, WILT], 11).unwrap();
    let level = h
        .app
        .world()
        .get::<shared::components::Level>(h.caster)
        .unwrap()
        .0;
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::UnitLevel(level));
    let source = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::world_data::load_creature_spell_stats(&source));
    if cat {
        h.cast_and_deliver(CAT_FORM, None);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    }
    h
}

#[test]
#[ignore = "RED census-tail effect62"]
fn census_tail_62_native_ferocious_bite_burns_only_remaining_energy_without_damage() {
    let mut h = burn_fixture(true);
    for (before, after) in [(75.0, 50.0), (10.0, 0.0)] {
        h.set_power(PowerType::Energy, before);
        let caster = h.caster;
        let health = h.health(caster);
        hit_native(&mut h, FEROCIOUS_BITE, 1, caster);
        assert_eq!(h.power(PowerType::Energy), after);
        assert_eq!(
            h.health(caster),
            health,
            "native zero amplitude adds no damage"
        );
    }
}

#[test]
fn census_tail_62_native_bite_does_not_burn_a_nonprimary_power_pool() {
    let mut h = burn_fixture(false);
    h.set_power(PowerType::Energy, 75.0);
    let caster = h.caster;
    hit_native(&mut h, FEROCIOUS_BITE, 1, caster);
    assert_eq!(h.power(PowerType::Energy), 75.0);
}

#[test]
#[ignore = "RED census-tail effect62"]
fn census_tail_62_native_wilt_damage_uses_actual_mana_burn_and_value_multiplier() {
    let mut h = burn_fixture(false);
    let caster = h.caster;
    h.app.world_mut().get_mut::<Mana>(caster).unwrap().current = 3.0;
    let health = h.health(caster);
    hit_native(&mut h, WILT, 0, caster);
    assert_eq!(h.app.world().get::<Mana>(caster).unwrap().current, 0.0);
    assert_eq!(
        h.health(caster),
        health - 6.0,
        "native amplitude2, not chain amplitude1; only three Mana existed"
    );
}

#[test]
fn census_tail_62_native_bite_does_not_burn_a_dead_target() {
    let mut h = burn_fixture(true);
    let caster = h.caster;
    h.set_power(PowerType::Energy, 75.0);
    h.app.world_mut().get_mut::<Health>(caster).unwrap().current = 0.0;
    hit_native(&mut h, FEROCIOUS_BITE, 1, caster);
    assert_eq!(h.power(PowerType::Energy), 75.0);
}

fn rewarded(h: &Harness, character: u64) -> bool {
    persistence::quest_store::load_quest_log(
        &h.app.world().resource::<GameDatabase>().db,
        character,
    )
    .is_rewarded(DREAMWALK_TRACKING_QUEST)
}

#[test]
fn census_tail_16_native_dreamwalk_rewards_its_tracking_quest_and_persists() {
    let (mut h, directory, character) = quest_fixture();
    assert!(!rewarded(&h, character));
    let caster = h.caster;
    hit_native(&mut h, DREAMWALK, 1, caster);
    assert!(rewarded(&h, character));
    assert!(
        h.app
            .world()
            .resource::<crate::quest::QuestDirty>()
            .0
            .contains(&character)
    );
    let database = h.app.world_mut().remove_resource::<GameDatabase>().unwrap();
    drop(database);
    h.app.insert_resource(GameDatabase::new(
        redb::Database::open(directory.path().join("quest.redb")).unwrap(),
    ));
    assert!(rewarded(&h, character));
    h.app
        .world_mut()
        .resource_mut::<crate::quest::QuestDirty>()
        .0
        .clear();
    hit_native(&mut h, DREAMWALK, 1, caster);
    assert!(
        h.app
            .world()
            .resource::<crate::quest::QuestDirty>()
            .0
            .is_empty(),
        "rewarded tracking quests are not replayed"
    );
}

#[test]
fn census_tail_16_native_dreamwalk_does_not_credit_caster_for_an_npc_hit() {
    let (mut h, _directory, character) = quest_fixture();
    let npc = h.npc;
    hit_native(&mut h, DREAMWALK, 1, npc);
    assert!(!rewarded(&h, character));
    assert!(
        h.app
            .world()
            .resource::<crate::quest::QuestDirty>()
            .0
            .is_empty()
    );
}
