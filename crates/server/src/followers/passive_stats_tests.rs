//! Real spec66 passive application must update the NPC's derived class stats.
use bevy::prelude::*;
use shared::components::{Health, Level, UnitLevel};

use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use crate::auras::{AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::Harness;
use crate::stats::{CharacterStats, StatModifiers, StatsPlugin};

const GARRICK: u32 = 209057;
const AEGIS_OF_LIGHT: u32 = 353367;
const RIGHTEOUS_FURY: u32 = 25780;

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
