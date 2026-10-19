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
#[ignore = "SWIFTMEND_NATIVE_PREREQUISITE_15: qualify backend before any follower fix"]
fn follower_swiftmend_qualification_native_without_hot_refuses() {
    let (mut h, healer) = level32(CRENNA);
    assert_native_restriction(&h);
    let patient = h.caster;
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
#[ignore = "SWIFTMEND_NATIVE_PREREQUISITE_15: retained positive native control"]
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
#[ignore = "SWIFTMEND_PATIENT_ELIGIBILITY: backend prerequisite must qualify first"]
fn follower_swiftmend_qualification_two_injured_patients_prefer_eligible_hot() {
    let (mut h, healer) = level32(CRENNA);
    assert_native_restriction(&h);
    let lower_without_hot = h.caster;
    apply_real_rejuvenation(&mut h, healer, healer);
    wound(&mut h, lower_without_hot, 0.1);
    wound(&mut h, healer, 0.3);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        lower_without_hot,
        REJUVENATION
    ));
    assert!(crate::auras::has_aura(h.app.world(), healer, REJUVENATION));
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
        vec![Some(healer.to_bits())],
        "actual follower must heal the eligible HoT patient, not lower no-HoT patient {lower_without_hot:?}"
    );
}
