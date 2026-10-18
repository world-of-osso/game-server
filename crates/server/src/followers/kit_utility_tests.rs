//! Source-native dispel/CC conditions; no movement or alternate interrupt owner.
use super::kit_tests::{level32, uses_spell};
use super::{update_ai, utility_tests};
use crate::spell_cast::test_harness::Harness;
use bevy::time::TimeUpdateStrategy;
use shared::components::Health;
use shared::protocol::CombatLogKind;

pub(super) fn wait_for_native_school_unlock(h: &mut Harness, interrupt: u32) {
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(interrupt)
        .unwrap();
    let duration = spell.misc.as_ref().unwrap().duration.unwrap().duration_ms;
    let cooldown = spell.cooldowns.as_ref().unwrap();
    let cooldown = cooldown.recovery_ms.max(cooldown.category_recovery_ms);
    assert!(duration > 0 && (duration as u32) < cooldown);
    let frame_ms = match h.app.world().resource::<TimeUpdateStrategy>() {
        TimeUpdateStrategy::ManualDuration(step) => step.as_millis(),
        _ => panic!("native harness must use a known manual frame step"),
    };
    h.advance((duration as u128).div_ceil(frame_ms) as usize + 1);
}

fn cast_debuff(h: &mut Harness, spell: u32, spec: u32) {
    let enemy = h.npc;
    let patient = h.caster;
    utility_tests::prepare_enemy_spec(h, enemy, spec);
    crate::spell_cast::creature_cast(h.app.world_mut(), enemy, spell, Some(patient), false)
        .unwrap();
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), patient, spell));
}

fn separate_dispeller_from_native_area(
    h: &mut Harness,
    unit: bevy::prelude::Entity,
    debuff: u32,
    dispel: u32,
) {
    let store = h.app.world().resource::<crate::spell_info::SpellStore>();
    let radius = store
        .get(debuff)
        .unwrap()
        .effects
        .iter()
        .flat_map(|effect| effect.radius.iter().flatten())
        .map(|radius| {
            if radius.radius > 0.0 {
                radius.radius
            } else {
                radius.radius_max
            }
        })
        .max_by(f32::total_cmp);
    let Some(radius) = radius.filter(|radius| *radius > 0.0) else {
        return;
    };
    let reach = store
        .get(dispel)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .range
        .unwrap()
        .max_friendly;
    assert!(
        reach > radius,
        "fixture must separate the dispeller without leaving native dispel range"
    );
    let patient = *h
        .app
        .world()
        .get::<shared::components::Position>(h.caster)
        .unwrap();
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(unit)
        .unwrap()
        .x = patient.x + (radius + reach) / 2.0;
}

fn assert_party_dispel(entry: u32, spell: u32, debuff: u32, enemy_spec: u32) {
    let (mut h, unit) = level32(entry);
    // The debuffed member is NOT the lowest-health member. Targeting must test
    // native dispel eligibility before ranking the party by health.
    {
        let mut health = h.app.world_mut().get_mut::<Health>(unit).unwrap();
        health.current = health.max * 0.7;
    }
    separate_dispeller_from_native_area(&mut h, unit, debuff, spell);
    cast_debuff(&mut h, debuff, enemy_spec);
    assert!(
        !crate::auras::has_aura(h.app.world(), unit, debuff),
        "only the healthier patient should carry the fixture debuff"
    );
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == debuff)
        .unwrap();
    assert!(
        aura.harmful,
        "native dispel fixture {debuff} must be a harmful aura before testing picker eligibility"
    );
    uses_spell(&mut h, unit, spell);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, debuff));
    assert!(
        h.inbox.log.iter().any(|event| {
            event.kind == CombatLogKind::Dispel
                && event.source == Some(unit.to_bits())
                && event.target == Some(h.caster.to_bits())
                && event.spell_id == Some(spell)
                && event.extra_spell_id == Some(debuff)
        }),
        "native dispel must name the eligible patient and removed aura"
    );
}

#[test]
#[ignore = "RED: Crenna's Nature's Cure is inventoried but never selected"]
fn follower_kit_utility_crenna_dispels_party_magic() {
    assert_party_dispel(209072, 88423, 589, 257);
}

#[test]
#[ignore = "RED: Henry's Purify is inventoried but never selected"]
fn follower_kit_utility_henry_dispels_party_magic() {
    assert_party_dispel(215011, 527, 589, 257);
}

#[test]
#[ignore = "RED: native Curse of Exhaustion does not drive Remove Curse"]
fn follower_kit_utility_meredy_dispels_curse_on_healthier_member() {
    assert_party_dispel(209059, 475, 334275, 266);
}

#[test]
#[ignore = "AURA_POSITIVITY_702: native Curse of Weakness has harmful=false upstream of the picker"]
fn native_curse702_harmful_classification_gap() {
    let (mut h, _) = level32(209059);
    cast_debuff(&mut h, 702, 266);
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 702)
        .unwrap();
    assert!(
        aura.harmful,
        "native342/290 curse positivity is not a follower-picker decision"
    );
}

#[test]
fn follower_kit_utility_garrick_does_not_cleanse_unsupported_magic() {
    let (mut h, unit) = level32(209057);
    cast_debuff(&mut h, 589, 257);
    for _ in 0..80 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 589));
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| go.caster == unit.to_bits() && go.spell_id == 213644)
    );
}

#[test]
fn follower_kit_utility_crenna_does_not_dispel_an_empty_patient() {
    let (mut h, unit) = level32(209072);
    for _ in 0..80 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| go.caster == unit.to_bits() && go.spell_id == 88423)
    );
}

#[test]
#[ignore = "RED: Polymorph is inventoried but never selected when the real kick is cooling down"]
fn follower_kit_utility_meredy_controls_caster_after_spent_counterspell() {
    let (mut h, unit) = level32(209059);
    // Polymorph admits native creature types. The generic Kobold actor has no
    // template authority until this fixture loads its real row; never guess7.
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[6]);
    h.app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .extend(native.templates);
    utility_tests::start_enemy_fireball(&mut h, unit);
    update_ai(h.app.world_mut());
    wait_for_native_school_unlock(&mut h, 2139);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt && event.spell_id == Some(2139))
    );
    utility_tests::start_enemy_fireball(&mut h, unit);
    uses_spell(&mut h, unit, 118);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 118));
    for _ in 0..40 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|go| go.caster == unit.to_bits() && go.spell_id == 118)
            .count(),
        1
    );
}

#[test]
fn follower_kit_utility_meredy_does_not_control_an_idle_attacker() {
    let (mut h, unit) = level32(209059);
    for _ in 0..80 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| go.caster == unit.to_bits() && go.spell_id == 118)
    );
}

#[test]
#[ignore = "RED: Bo never uses source Leg Sweep as a caster-control fallback"]
fn follower_kit_utility_bo_controls_caster_after_spent_spear_hand() {
    let (mut h, unit) = level32(227734);
    utility_tests::start_enemy_fireball(&mut h, unit);
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt && event.spell_id == Some(116705))
    );
    wait_for_native_school_unlock(&mut h, 116705);
    utility_tests::start_enemy_fireball(&mut h, unit);
    uses_spell(&mut h, unit, 119381);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 119381));
}
