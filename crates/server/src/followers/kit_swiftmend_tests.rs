//! Qualification only: native 70448 restriction15919 requires TargetAuraState15.
use super::kit_tests::level32;
use crate::spell_cast::test_harness::Harness;
use crate::spell_cast::{CastFailure, SpellHistory, creature_cast, history_now};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use shared::components::Health;
use shared::protocol::CombatLogKind;

const SWIFTMEND: u32 = 18562;
const REJUVENATION: u32 = 774;
const CRENNA: u32 = 209072;
const SWIFTMEND_TARGET_AURA_STATE: u32 = 15;

fn assert_native_restriction(h: &Harness) {
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(SWIFTMEND)
        .unwrap();
    let restriction = spell.aura_restrictions.as_ref().unwrap();
    assert_eq!(restriction.target_aura_state, SWIFTMEND_TARGET_AURA_STATE);
}

fn wound(h: &mut Harness, patient: Entity, fraction: f32) {
    let mut health = h.app.world_mut().get_mut::<Health>(patient).unwrap();
    health.current = health.max * fraction;
}

fn apply_real_rejuvenation(h: &mut Harness, healer: Entity, patient: Entity) {
    creature_cast(
        h.app.world_mut(),
        healer,
        REJUVENATION,
        Some(patient),
        false,
    )
    .unwrap();
    for _ in 0..200 {
        h.advance(1);
        let world = h.app.world();
        let spell = world.resource::<SpellStore>().get(SWIFTMEND).unwrap();
        let on_gcd = world
            .get::<SpellHistory>(healer)
            .is_some_and(|history| history.on_gcd(spell, history_now(world)));
        if !on_gcd {
            assert!(crate::auras::has_aura(world, patient, REJUVENATION));
            return;
        }
    }
    panic!("ordinary Rejuvenation GCD did not finish with its real aura active");
}

#[test]
fn follower_swiftmend_qualification_native_without_hot_refuses() {
    let (mut h, healer) = level32(CRENNA);
    assert_native_restriction(&h);
    // TC SpellInfo::CheckTarget exempts a controlled caster healing its owner.
    let patient = healer;
    assert!(!crate::auras::has_aura(
        h.app.world(),
        patient,
        REJUVENATION
    ));
    wound(&mut h, patient, 0.3);
    let result = creature_cast(h.app.world_mut(), healer, SWIFTMEND, Some(patient), false);
    assert!(
        matches!(result, Err(CastFailure::TargetAuraState)),
        "native18562 must refuse absent state15, ordinary result={result:?}"
    );
}

#[test]
fn follower_swiftmend_qualification_removed_real_rejuvenation_no_longer_qualifies() {
    let (mut h, healer) = level32(CRENNA);
    apply_real_rejuvenation(&mut h, healer, healer);
    assert_eq!(
        crate::auras::aura_state::read_computed_state(h.app.world(), healer, 15),
        Some(true)
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        healer,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == REJUVENATION,
    );
    assert!(!crate::auras::has_aura(h.app.world(), healer, REJUVENATION));
    wound(&mut h, healer, 0.3);
    let result = creature_cast(h.app.world_mut(), healer, SWIFTMEND, Some(healer), false);
    assert!(
        matches!(result, Err(CastFailure::TargetAuraState)),
        "removed prerequisite must not remain cached: {result:?}"
    );
}

#[test]
fn follower_swiftmend_qualification_uncomputed_native_state_is_not_false() {
    let (h, healer) = level32(CRENNA);
    let execute = h.app.world().resource::<SpellStore>().get(163201).unwrap();
    let state = execute
        .aura_restrictions
        .as_ref()
        .unwrap()
        .target_aura_state;
    assert_eq!(state, 2);
    assert_eq!(
        crate::auras::aura_state::read_computed_state(h.app.world(), healer, state),
        None
    );
}

#[test]
fn follower_swiftmend_qualification_owner_without_hot_is_exempt() {
    let (mut h, healer) = level32(CRENNA);
    assert_native_restriction(&h);
    let owner = h.caster;
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitSummonedBy>(healer)
            .unwrap()
            .0,
        owner.to_bits()
    );
    wound(&mut h, owner, 0.3);
    let before = h.health(owner);
    creature_cast(h.app.world_mut(), healer, SWIFTMEND, Some(owner), false).unwrap();
    h.advance(3);
    assert!(h.health(owner) > before);
}

#[test]
fn follower_swiftmend_qualification_other_casters_rejuvenation_qualifies() {
    let (mut h, healer) = level32(CRENNA);
    let profile = h
        .app
        .world()
        .get::<super::Follower>(healer)
        .unwrap()
        .profile
        .clone();
    let other = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    let owner = h.caster;
    super::spawning::initialize(h.app.world_mut(), owner, other, profile).unwrap();
    apply_real_rejuvenation(&mut h, other, healer);
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(healer)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == REJUVENATION)
        .unwrap();
    assert_eq!(aura.caster, other);
    wound(&mut h, healer, 0.3);
    let before = h.health(healer);
    creature_cast(h.app.world_mut(), healer, SWIFTMEND, Some(healer), false).unwrap();
    h.advance(3);
    assert!(h.health(healer) > before);
}

#[test]
fn follower_swiftmend_qualification_native_real_rejuvenation_heals() {
    let (mut h, healer) = level32(CRENNA);
    assert_native_restriction(&h);
    let patient = h.caster;
    apply_real_rejuvenation(&mut h, healer, patient);
    wound(&mut h, patient, 0.3);
    let before = h.health(patient);
    creature_cast(h.app.world_mut(), healer, SWIFTMEND, Some(patient), false).unwrap();
    h.advance(3);
    assert!(h.health(patient) > before);
    assert!(h.inbox.log.iter().any(|event| {
        event.kind == CombatLogKind::Heal
            && event.source == Some(healer.to_bits())
            && event.target == Some(patient.to_bits())
            && event.spell_id == Some(SWIFTMEND)
    }));
}

#[test]
fn follower_swiftmend_qualification_two_injured_patients_prefer_eligible_hot() {
    let (mut h, healer) = level32(CRENNA);
    assert_native_restriction(&h);
    let lower_without_hot = healer;
    let higher_with_hot = h.caster;
    apply_real_rejuvenation(&mut h, healer, higher_with_hot);
    wound(&mut h, lower_without_hot, 0.1);
    wound(&mut h, higher_with_hot, 0.3);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        lower_without_hot,
        REJUVENATION
    ));
    assert!(crate::auras::has_aura(
        h.app.world(),
        higher_with_hot,
        REJUVENATION
    ));
    super::update_ai(h.app.world_mut());
    h.advance(3);
    let swiftmend_targets: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Heal
                && event.source == Some(healer.to_bits())
                && event.spell_id == Some(SWIFTMEND)
        })
        .map(|event| event.target)
        .collect();
    assert_eq!(
        swiftmend_targets,
        vec![Some(higher_with_hot.to_bits())],
        "actual follower must heal the eligible HoT patient, not lower no-HoT patient {lower_without_hot:?}"
    );
}
