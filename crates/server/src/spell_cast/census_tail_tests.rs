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

const ASPHYXIATE: u32 = 221_562;
const HAMMER_OF_JUSTICE: u32 = 853;
const GRAVITY_DISABLE: u32 = 195_289;
const FIREBALL: u32 = 133;

fn stun_fixture() -> Harness {
    harness(
        Vec::new(),
        &[ASPHYXIATE, HAMMER_OF_JUSTICE, GRAVITY_DISABLE, FIREBALL],
        8,
    )
    .unwrap()
}

fn assert_stun_state(h: &Harness, stunned: bool, gravity: bool) {
    let auras = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap();
    assert_eq!(auras.is_stunned(), stunned);
    let mods = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(h.caster)
        .unwrap();
    assert_eq!(mods.movement_speed, if stunned { 0.0 } else { 1.0 });
    assert_eq!(mods.flight_speed, if stunned { 0.0 } else { 1.0 });
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::world_state::WorldAuraState>(h.caster)
            .unwrap()
            .disable_gravity,
        gravity
    );
    let fireball = world_db().unwrap().store.get(FIREBALL).unwrap();
    assert_eq!(
        crate::auras::cast_blocker(h.app.world(), h.caster, fireball),
        stunned.then_some(crate::auras::CastBlock::Stunned)
    );
}

#[test]
fn census_tail_298_native_asphyxiate_stops_cast_and_releases_stun_gravity() {
    let mut h = stun_fixture();
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .is_some()
    );
    let caster = h.caster;
    hit_native(&mut h, ASPHYXIATE, 0, caster);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_none(),
        "real Asphyxiate must stop the active Fireball"
    );
    assert_stun_state(&h, true, true);
    remove_flight_aura(&mut h, caster, ASPHYXIATE);
    assert_stun_state(&h, false, false);
}

#[test]
fn census_tail_298_native_removal_preserves_other_stun_and_gravity_sources() {
    let mut h = stun_fixture();
    let caster = h.caster;
    hit_native(&mut h, ASPHYXIATE, 0, caster);
    hit_native(&mut h, GRAVITY_DISABLE, 0, caster);
    hit_native(&mut h, HAMMER_OF_JUSTICE, 0, caster);
    assert_stun_state(&h, true, true);
    remove_flight_aura(&mut h, caster, ASPHYXIATE);
    assert_stun_state(&h, true, true);
    remove_flight_aura(&mut h, caster, HAMMER_OF_JUSTICE);
    assert_stun_state(&h, false, true);
    remove_flight_aura(&mut h, caster, GRAVITY_DISABLE);
    assert_stun_state(&h, false, false);
}

#[test]
fn census_tail_298_native_asphyxiate_remains_when_ordinary_stun_is_removed() {
    let mut h = stun_fixture();
    let caster = h.caster;
    hit_native(&mut h, HAMMER_OF_JUSTICE, 0, caster);
    hit_native(&mut h, ASPHYXIATE, 0, caster);
    remove_flight_aura(&mut h, caster, HAMMER_OF_JUSTICE);
    assert_stun_state(&h, true, true);
    remove_flight_aura(&mut h, caster, ASPHYXIATE);
    assert_stun_state(&h, false, false);
}

const SHIMMER: u32 = 212_653;

fn set_tail_moving(h: &mut Harness) {
    h.app
        .world_mut()
        .get_mut::<crate::networking::Moving>(h.caster)
        .unwrap()
        .0 = true;
}

#[test]
fn census_tail_377_native_shimmer_updates_active_fireball_and_removal_interrupts() {
    let mut h = harness(Vec::new(), &[SHIMMER, FIREBALL], 8).unwrap();
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    assert!(
        !h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .unwrap()
            .cast_while_moving
    );
    let caster = h.caster;
    hit_native(&mut h, SHIMMER, 1, caster);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .unwrap()
            .cast_while_moving
    );
    set_tail_moving(&mut h);
    h.advance(2);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_some()
    );
    remove_flight_aura(&mut h, caster, SHIMMER);
    assert!(
        !h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .unwrap()
            .cast_while_moving
    );
    h.advance(1);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_none()
    );
}

#[test]
fn census_tail_377_native_shimmer_allows_foreign_family_then_expires() {
    const LIGHTNING_BOLT: u32 = 188_196;
    let mut h = harness(Vec::new(), &[SHIMMER, LIGHTNING_BOLT], 7).unwrap();
    let caster = h.caster;
    hit_native(&mut h, SHIMMER, 1, caster);
    set_tail_moving(&mut h);
    h.cast_and_deliver(LIGHTNING_BOLT, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .unwrap()
            .cast_while_moving
    );
    h.advance(20);
    assert!(!crate::auras::has_aura(h.app.world(), caster, SHIMMER));
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_none()
    );
    assert!(h.inbox.go.iter().all(|go| go.spell_id != LIGHTNING_BOLT));
}

#[test]
fn census_tail_377_native_shimmer_updates_channel_only_on_its_recipient() {
    const DISINTEGRATE: u32 = 356_995;
    let mut h = harness(Vec::new(), &[SHIMMER, DISINTEGRATE, 370898], 13).unwrap();
    h.set_power(PowerType::Essence, 5.0);
    h.cast_and_deliver(DISINTEGRATE, Some(h.npc));
    let caster = h.caster;
    let npc = h.npc;
    hit_native(&mut h, SHIMMER, 1, npc);
    assert!(
        !h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .unwrap()
            .cast_while_moving
    );
    hit_native(&mut h, SHIMMER, 1, caster);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .unwrap()
            .cast_while_moving
    );
    set_tail_moving(&mut h);
    h.advance(2);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_some()
    );
    remove_flight_aura(&mut h, caster, SHIMMER);
    h.advance(1);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_none()
    );
    assert!(crate::auras::has_aura(h.app.world(), npc, SHIMMER));
}

#[test]
fn census_tail_377_native_prohibition_attribute_wins_over_all_spell_aura() {
    let mut h = harness(Vec::new(), &[SHIMMER, FIREBALL, 21651], 8).unwrap();
    let caster = h.caster;
    hit_native(&mut h, SHIMMER, 1, caster);
    let opening = world_db().unwrap().store.get(21651).unwrap();
    assert!(!super::aura_allows_moving(h.app.world(), caster, opening));
    assert!(super::aura_allows_moving(
        h.app.world(),
        caster,
        world_db().unwrap().store.get(FIREBALL).unwrap()
    ));
}

const MANA_CONFLUENCE: u32 = 1_270_845;
const HEART_OF_THE_WILD: u32 = 108_294;
const MANA_BREAK: u32 = 203_704;
const FORGEBORNE_REVERIES: u32 = 327_140;

fn mana_cost_fixture() -> Harness {
    let mut h = harness(
        Vec::new(),
        &[
            MANA_CONFLUENCE,
            HEART_OF_THE_WILD,
            MANA_BREAK,
            FORGEBORNE_REVERIES,
            FIREBALL,
            1752,
        ],
        8,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    h.advance(3);
    let caster = h.caster;
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |_| true,
    );
    h
}

fn native_mana_cost(h: &Harness) -> f32 {
    let spell = world_db().unwrap().store.get(FIREBALL).unwrap();
    super::applicable_costs(h.app.world(), h.caster, spell)
        .unwrap()
        .into_iter()
        .find_map(|(pool, amount, _)| {
            (pool == super::CostPool::Power(PowerType::Mana)).then_some(amount)
        })
        .unwrap()
}

fn pay_fireball_cost(h: &mut Harness, expected: f32) {
    let spell = world_db().unwrap().store.get(FIREBALL).unwrap();
    let costs = super::spell_costs(h.app.world(), h.caster, spell).unwrap();
    let before = h.app.world().get::<Mana>(h.caster).unwrap().current;
    super::spend_costs(h.app.world_mut(), h.caster, &costs);
    assert_eq!(
        h.app.world().get::<Mana>(h.caster).unwrap().current,
        before - expected
    );
}

#[test]
fn census_tail_423_native_mana_confluence_changes_paid_cost_then_restores() {
    let mut h = mana_cost_fixture();
    let baseline = native_mana_cost(&h);
    assert!(baseline > 0.0);
    let caster = h.caster;
    hit_native(&mut h, MANA_CONFLUENCE, 0, caster);
    pay_fireball_cost(&mut h, (baseline * 0.95).trunc());
    remove_flight_aura(&mut h, caster, MANA_CONFLUENCE);
    pay_fireball_cost(&mut h, baseline);
}

#[test]
fn census_tail_423_native_independent_percentages_add_instead_of_multiply() {
    let mut h = mana_cost_fixture();
    let baseline = native_mana_cost(&h);
    let caster = h.caster;
    hit_native(&mut h, MANA_CONFLUENCE, 0, caster);
    hit_native(&mut h, HEART_OF_THE_WILD, 2, caster);
    hit_native(&mut h, MANA_BREAK, 2, caster);
    assert!(crate::auras::has_aura(h.app.world(), caster, MANA_BREAK));
    pay_fireball_cost(&mut h, (baseline * 0.75).trunc());
}

#[test]
fn census_tail_423_native_full_reduction_clamps_cost_sign_without_credit() {
    let mut h = mana_cost_fixture();
    let caster = h.caster;
    hit_native(&mut h, FORGEBORNE_REVERIES, 4, caster);
    hit_native(&mut h, MANA_CONFLUENCE, 0, caster);
    pay_fireball_cost(&mut h, 0.0);
    remove_flight_aura(&mut h, caster, MANA_CONFLUENCE);
    pay_fireball_cost(&mut h, 0.0);
}

#[test]
fn census_tail_423_native_arcane_surge_optional_mana_uses_the_same_multiplier() {
    const ARCANE_SURGE: u32 = 365_350;
    let mut h = mana_cost_fixture();
    let caster = h.caster;
    // Large concrete pool exposes the actual authored2900000 optional cost,
    // rather than hiding either calculation behind available-power clamping.
    *h.app.world_mut().get_mut::<Mana>(caster).unwrap() = Mana {
        current: 5_000_000.0,
        max: 5_000_000.0,
    };
    hit_native(&mut h, MANA_CONFLUENCE, 0, caster);
    let spell = world_db().unwrap().store.get(ARCANE_SURGE).unwrap();
    let costs = super::spell_costs(h.app.world(), caster, spell).unwrap();
    super::spend_costs(h.app.world_mut(), caster, &costs);
    assert_eq!(
        h.app.world().get::<Mana>(caster).unwrap().current,
        2_245_000.0
    );
}

#[test]
fn census_tail_423_native_mana_modifier_leaves_other_powers_and_owners_unchanged() {
    let mut h = mana_cost_fixture();
    let baseline_mana = native_mana_cost(&h);
    let npc = h.npc;
    hit_native(&mut h, MANA_CONFLUENCE, 0, npc);
    assert_eq!(native_mana_cost(&h), baseline_mana);
    // A real Rogue owns the native Energy pool; a Mage does not.
    let mut h = harness(Vec::new(), &[MANA_CONFLUENCE, 1752], 4).unwrap();
    h.set_power(PowerType::Energy, 100.0);
    let spell = world_db().unwrap().store.get(1752).unwrap();
    let before = super::applicable_costs(h.app.world(), h.caster, spell).unwrap();
    let caster = h.caster;
    hit_native(&mut h, MANA_CONFLUENCE, 0, caster);
    assert_eq!(
        super::applicable_costs(h.app.world(), caster, spell).unwrap(),
        before
    );
    let costs = super::spell_costs(h.app.world(), caster, spell).unwrap();
    let expected = costs
        .iter()
        .filter_map(|(pool, amount)| {
            (*pool == super::CostPool::Power(PowerType::Energy)).then_some(*amount)
        })
        .sum::<f32>();
    super::spend_costs(h.app.world_mut(), caster, &costs);
    assert_eq!(h.power(PowerType::Energy), 100.0 - expected);
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
