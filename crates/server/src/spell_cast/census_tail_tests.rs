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
const BARKSKIN: u32 = 22_812;

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
    let mut h = harness(Vec::new(), &[FEROCIOUS_BITE, CAT_FORM, WILT, BARKSKIN], 11).unwrap();
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
fn census_tail_62_native_wilt_bypasses_spell_bonus_taken_but_still_burns_mana() {
    let mut h = burn_fixture(false);
    h.cast_and_deliver(BARKSKIN, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, BARKSKIN));
    let caster = h.caster;
    h.app.world_mut().get_mut::<Mana>(caster).unwrap().current = 3.0;
    let health = h.health(caster);
    hit_native(&mut h, WILT, 0, caster);
    assert_eq!(h.app.world().get::<Mana>(caster).unwrap().current, 0.0);
    assert_eq!(
        h.health(caster),
        health - 6.0,
        "TC EffectPowerBurn does not call SpellDamageBonusTaken"
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

const ZEN_FLIGHT: u32 = 125_883;
const CROW_SPEED: u32 = 48_403;
const MOUNT_SPEED: u32 = 13_587;
const FLIGHT_FORM_SPEED: u32 = 33_948;

fn flight_fixture() -> Harness {
    let extras = [ZEN_FLIGHT, CROW_SPEED, MOUNT_SPEED, FLIGHT_FORM_SPEED];
    let mut h = crate::auras::mount_tests::rider(225, true, &extras)
        .expect("native70448 flight and mount records");
    // Keep native passive records loadable without auto-granting Crow Speed.
    let baseline = h
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(h.caster)
        .unwrap()
        .iter()
        .filter(|id| !extras.contains(id))
        .collect();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(baseline));
    let caster = h.caster;
    for spell_id in extras {
        remove_flight_aura(&mut h, caster, spell_id);
    }
    h.advance(3);
    h
}

fn remove_flight_aura(h: &mut Harness, unit: Entity, spell_id: u32) {
    crate::auras::remove_auras(
        h.app.world_mut(),
        unit,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == spell_id,
    );
}

fn assert_flight(h: &Harness, unit: Entity, expected: f32) {
    let actual = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(unit)
        .unwrap()
        .flight_speed;
    assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
}

#[test]
fn census_tail_206_native_zen_and_crow_add_unmounted_then_remove() {
    let mut h = flight_fixture();
    let caster = h.caster;
    hit_native(&mut h, ZEN_FLIGHT, 1, caster);
    assert!(crate::auras::has_aura(h.app.world(), caster, ZEN_FLIGHT));
    assert_flight(&h, caster, 1.6);
    hit_native(&mut h, CROW_SPEED, 1, caster);
    assert_flight(&h, caster, 1.7);
    remove_flight_aura(&mut h, caster, ZEN_FLIGHT);
    assert_flight(&h, caster, 1.1);
    remove_flight_aura(&mut h, caster, CROW_SPEED);
    assert_flight(&h, caster, 1.0);
}

#[test]
fn census_tail_206_native_zen_does_not_change_mounted_flight() {
    let mut h = flight_fixture();
    crate::auras::mount_tests::mount(&mut h);
    let caster = h.caster;
    assert_flight(&h, caster, 3.2);
    hit_native(&mut h, ZEN_FLIGHT, 1, caster);
    assert_flight(&h, caster, 3.2);
}

#[test]
fn census_tail_206_native_crow_no_longer_uses_wrong_mounted_alias() {
    let mut h = flight_fixture();
    crate::auras::mount_tests::mount(&mut h);
    let caster = h.caster;
    hit_native(&mut h, CROW_SPEED, 1, caster);
    assert!(crate::auras::has_aura(h.app.world(), caster, CROW_SPEED));
    assert_flight(&h, caster, 3.2);
}

#[test]
fn census_tail_206_native_mount_speed_209_multiplies_only_mounted_flight() {
    let mut h = flight_fixture();
    let caster = h.caster;
    hit_native(&mut h, MOUNT_SPEED, 1, caster);
    assert_flight(&h, caster, 1.0);
    crate::auras::mount_tests::mount(&mut h);
    assert!(crate::auras::has_aura(h.app.world(), caster, MOUNT_SPEED));
    assert_flight(&h, caster, 3.296);
    remove_flight_aura(&mut h, caster, MOUNT_SPEED);
    assert_flight(&h, caster, 3.2);
}

#[test]
fn census_tail_206_native_controlled_npc_uses_strongest_vehicle_bonus() {
    let mut h = flight_fixture();
    let npc = spawn_npc(&mut h.app, 2.0);
    h.app.world_mut().entity_mut(npc).insert((
        shared::components::UnitFlags(crate::faction::UNIT_FLAG_PLAYER_CONTROLLED),
        crate::auras::ActiveAuras::default(),
        crate::auras::AuraStatMods::default(),
    ));
    hit_native(&mut h, ZEN_FLIGHT, 1, npc);
    hit_native(&mut h, FLIGHT_FORM_SPEED, 1, npc);
    hit_native(&mut h, CROW_SPEED, 1, npc);
    assert_flight(&h, npc, 3.2);
    remove_flight_aura(&mut h, npc, FLIGHT_FORM_SPEED);
    assert_flight(&h, npc, 1.6);
}

#[test]
fn census_tail_206_native_uncontrolled_npc_adds_flight_modifiers() {
    let mut h = flight_fixture();
    let npc = spawn_npc(&mut h.app, 2.0);
    h.app.world_mut().entity_mut(npc).insert((
        crate::auras::ActiveAuras::default(),
        crate::auras::AuraStatMods::default(),
    ));
    hit_native(&mut h, ZEN_FLIGHT, 1, npc);
    hit_native(&mut h, FLIGHT_FORM_SPEED, 1, npc);
    hit_native(&mut h, CROW_SPEED, 1, npc);
    assert_flight(&h, npc, 3.9);
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
