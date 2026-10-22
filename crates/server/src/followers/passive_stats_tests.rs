//! Real spec66 passive application must update the NPC's derived class stats.
use bevy::prelude::*;
use shared::components::{CombatRatings, DerivedStats, Health, Level, UnitLevel, UnitStats};

use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use crate::auras::{AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::Harness;
use crate::stats::{CharacterStats, StatModifiers, StatsPlugin};

const GARRICK: u32 = 209057;
const AEGIS_OF_LIGHT: u32 = 353367;
const RIGHTEOUS_FURY: u32 = 25780;

fn mastery_follower(entry: u32) -> (Harness, Entity) {
    let (mut h, unit, profile) = spawn_profile_fixture_with_plugins(entry, |app| {
        app.add_plugins(StatsPlugin);
    });
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(90), UnitLevel(90)));
    super::spawning::initialize(h.app.world_mut(), h.caster, unit, profile).unwrap();
    h.advance(6);
    assert!(
        h.app
            .world()
            .get::<shared::components::Player>(unit)
            .is_none()
    );
    (h, unit)
}

fn apply_native_aura(h: &mut Harness, unit: Entity, spell_id: u32) {
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap();
    let effect_mask = crate::auras::aura_effect_mask(spell);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id,
            effect_mask,
            source: AuraSource::KnownSpell,
        },
    );
    h.advance(6);
}

fn assert_native_mastery_attack_power(entry: u32, mastery: u32) {
    let (mut h, unit) = mastery_follower(entry);
    crate::auras::remove_auras(h.app.world_mut(), unit, RemoveReason::Cancelled, |aura| {
        aura.spell_id == mastery
    });
    h.advance(6);
    let base = h
        .app
        .world()
        .get::<CharacterStats>(unit)
        .unwrap()
        .attack_power;
    apply_native_aura(&mut h, unit, mastery);
    let current = h.app.world().get::<CharacterStats>(unit).unwrap();
    let expected = base * (1.0 + current.mastery_points / 100.0);
    assert!(
        (current.attack_power - expected).abs() < 0.01,
        "native follower{entry} mastery{mastery}: AP{} expected{expected} from base{base}, points{}",
        current.attack_power,
        current.mastery_points
    );
    //70448 Elemental Blast: Mastery173184 supplies3 actual raw mastery points.
    // Existing mastery auras must recalculate without being removed/reapplied.
    apply_native_aura(&mut h, unit, 173184);
    let current = h.app.world().get::<CharacterStats>(unit).unwrap();
    let expected = base * (1.0 + current.mastery_points / 100.0);
    assert!(
        (current.attack_power - expected).abs() < 0.01,
        "live mastery refresh: AP{} expected{expected}",
        current.attack_power
    );
}

#[test]
#[ignore = "DPS parity RED: follower mastery is incorrectly Player-only"]
fn dps_parity_follower_native_paladin_mastery_updates_attack_power() {
    //70448 Mastery: Divine Bulwark76671 e1:166, coefficient1 per raw point.
    assert_native_mastery_attack_power(209057, 76671);
}

#[test]
#[ignore = "DPS parity RED: follower mastery is incorrectly Player-only"]
fn dps_parity_follower_native_warrior_mastery_updates_attack_power() {
    //70448 Mastery: Critical Block76857 e4:166, coefficient1 per raw point.
    assert_native_mastery_attack_power(215517, 76857);
}

#[test]
#[ignore = "DPS parity RED: follower mastery is incorrectly Player-only"]
fn dps_parity_follower_native_elemental_mastery_modifies_damage_and_refreshes() {
    let (mut h, unit) = mastery_follower(209053);
    crate::auras::remove_auras(h.app.world_mut(), unit, RemoveReason::Cancelled, |aura| {
        aura.spell_id == 168534
    });
    h.advance(6);
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(188196)
        .unwrap()
        .clone();
    let modify = |h: &Harness| {
        crate::auras::spell_mods::modify(
            h.app.world(),
            unit,
            &spell,
            crate::auras::SpellModOp::HealingAndDamage,
            100.0,
        )
    };
    let base = modify(&h);
    let coefficient = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(168534)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .unwrap()
        .bonus_coefficient;
    apply_native_aura(&mut h, unit, 168534);
    let points = h
        .app
        .world()
        .get::<CharacterStats>(unit)
        .unwrap()
        .mastery_points;
    let expected = base * (1.0 + points * coefficient / 100.0);
    assert!(
        (modify(&h) - expected).abs() < 0.01,
        "native Lightning Bolt damage modifier{} expected{expected}",
        modify(&h)
    );
    apply_native_aura(&mut h, unit, 173184);
    let points = h
        .app
        .world()
        .get::<CharacterStats>(unit)
        .unwrap()
        .mastery_points;
    let expected = base * (1.0 + points * coefficient / 100.0);
    assert!(
        (modify(&h) - expected).abs() < 0.01,
        "live Lightning Bolt modifier{} expected{expected}",
        modify(&h)
    );
}

fn spawn_level32_garrick() -> (Harness, Entity) {
    let (mut h, old_unit, profile) = spawn_profile_fixture_with_plugins(GARRICK, |app| {
        app.add_plugins(StatsPlugin);
    });
    h.app.world_mut().despawn(old_unit);
    let native_world = crate::world_data::load_world_data(
        &crate::spell_cast::test_harness::world_db()
            .unwrap()
            .powers_db,
    );
    h.app.insert_resource(native_world);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(32), UnitLevel(32)));
    let unit = super::spawning::spawn(h.app.world_mut(), h.caster, profile).unwrap();
    assert!(crate::auras::has_aura(h.app.world(), unit, AEGIS_OF_LIGHT));
    (h, unit)
}

#[test]
fn follower_passive_stats_real_spec66_stamina_recomputes_max_health() {
    let (mut h, unit) = spawn_level32_garrick();
    h.advance(3);
    let modifiers = h.app.world().get::<StatModifiers>(unit).unwrap();
    // Native70448: independent Aegis353367 +30% and Plate86102 +5%,
    // both137/mask4, no SAME_EFFECT group: 1.30*1.05=1.365 (TC GetTotalAuraMultiplier).
    // Base class2/level32 stamina980 ->1337.7; floor1337*gtHpPerSta32=8 ->10696HP.
    // No fixture stat or health overrides.
    assert_eq!(modifiers.primary_pct.stamina, 36.5);
    let health = h.app.world().get::<Health>(unit).unwrap();
    assert_eq!(
        health.max, 10696.0,
        "native passive modifiers were stored but not applied"
    );
    assert_eq!(health.current, health.max);
    assert_eq!(
        h.app
            .world()
            .get::<CharacterStats>(unit)
            .unwrap()
            .primary
            .stamina,
        1337.7
    );
}

#[test]
fn follower_passive_stats_real_aegis_removal_and_reapply_refresh_max_health() {
    let (mut h, unit) = spawn_level32_garrick();
    crate::auras::remove_auras(h.app.world_mut(), unit, RemoveReason::Expired, |aura| {
        aura.spell_id == AEGIS_OF_LIGHT
    });
    h.advance(3);
    assert_eq!(
        h.app
            .world()
            .get::<StatModifiers>(unit)
            .unwrap()
            .primary_pct
            .stamina,
        5.0
    );
    assert_eq!(
        h.app.world().get::<Health>(unit).unwrap().max,
        8232.0,
        "remaining native5% stamina must reach the same production stat calculation"
    );
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(AEGIS_OF_LIGHT)
        .unwrap()
        .clone();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: AEGIS_OF_LIGHT,
            effect_mask: crate::auras::aura_effect_mask(&spell),
            source: AuraSource::KnownSpell,
        },
    );
    h.advance(3);
    let health = h.app.world().get::<Health>(unit).unwrap();
    assert_eq!(health.max, 10696.0);
    assert_eq!(health.current, health.max);
}

#[test]
fn follower_passive_stats_owned_evade_preserves_native_spec_and_threat_auras() {
    let (mut h, unit) = spawn_level32_garrick();
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), unit, RIGHTEOUS_FURY));
    let spell_power = h
        .app
        .world()
        .get::<shared::components::DerivedStats>(unit)
        .unwrap()
        .spell_power;
    assert!(spell_power > 0.0);
    h.app
        .world_mut()
        .entity_mut(unit)
        .insert(crate::creature_ai::CreatureState::Pursuing {
            target: h.npc.to_bits(),
        });
    // Production no-path/leash lifecycle entry, followed by real aura/stat systems.
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), unit);
    h.advance(3);
    assert_eq!(h.app.world().get::<Health>(unit).unwrap().max, 10696.0);
    assert!(crate::auras::has_aura(h.app.world(), unit, AEGIS_OF_LIGHT));
    assert!(crate::auras::has_aura(h.app.world(), unit, RIGHTEOUS_FURY));
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::DerivedStats>(unit)
            .unwrap()
            .spell_power,
        spell_power
    );
}

#[test]
fn follower_passive_stats_native_base_stat80_does_not_change_non_player_bridge() {
    const BASE_INTELLECT: u32 = 13326;
    let (mut h, unit) = spawn_level32_garrick();
    // Normal player stat-query components: StatsPlugin derives the Mage owner's
    // actual native values; no caster or recipient stat fields are overwritten.
    {
        let mut owner = h.app.world_mut().entity_mut(h.caster);
        owner.entry::<UnitStats>().or_default();
        owner.entry::<CombatRatings>().or_default();
        owner.entry::<DerivedStats>().or_default();
        owner.insert(crate::class_progression::KnownSpells::fixture(vec![
            BASE_INTELLECT,
        ]));
    }
    h.advance(3);
    let before = h
        .app
        .world()
        .get::<CharacterStats>(unit)
        .unwrap()
        .primary
        .intellect;
    assert!(before > 0.0);
    assert!(
        h.app
            .world()
            .get::<CharacterStats>(h.caster)
            .unwrap()
            .primary
            .intellect
            > 0.0
    );
    h.cast_and_deliver(BASE_INTELLECT, Some(unit));
    h.advance(2);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), unit, BASE_INTELLECT));
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        BASE_INTELLECT
    ));
    assert_eq!(
        h.app
            .world()
            .get::<CharacterStats>(unit)
            .unwrap()
            .primary
            .intellect,
        before,
        "TC HandleModPercentStat returns before BASE_PCT for non-Player recipients"
    );
}
