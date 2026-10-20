//! Native158478/114108 and SimC76f8118 persistent multiplier qualification.
use super::kit_tests::at_level;
use crate::auras::spell_mods::SpellModOp;
use crate::auras::{AuraStatMods, spell_mods};
use crate::spell_cast::test_harness::Harness;
use crate::spell_cast::{SpellHistory, creature_cast, effects, history_now};
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::Health;
use shared::protocol::{CombatLogEvent, CombatLogKind};

const CRENNA: u32 = 209072;
const TALENT: u32 = 158478;
const SOUL_OF_THE_FOREST: u32 = 114108;
const SWIFTMEND: u32 = 18562;
const REJUVENATION: u32 = 774;
const REGROWTH: u32 = 8936;

fn wait_for_native_cast_and_gcd(h: &mut Harness, caster: Entity) {
    for _ in 0..200 {
        h.advance(1);
        let world = h.app.world();
        let spell = world.resource::<SpellStore>().get(REJUVENATION).unwrap();
        let locked = world
            .get::<SpellHistory>(caster)
            .is_some_and(|history| history.on_gcd(spell, history_now(world)));
        if !locked && world.get::<CastState>(caster).is_none() {
            return;
        }
    }
    panic!("ordinary native cast/GCD did not finish");
}

fn cast_setup_swiftmend(h: &mut Harness, caster: Entity, patient: Entity) {
    creature_cast(
        h.app.world_mut(),
        caster,
        REJUVENATION,
        Some(patient),
        false,
    )
    .unwrap();
    wait_for_native_cast_and_gcd(h, caster);
    let mut health = h.app.world_mut().get_mut::<Health>(patient).unwrap();
    health.current = health.max * 0.3;
    creature_cast(h.app.world_mut(), caster, SWIFTMEND, Some(patient), false).unwrap();
    wait_for_native_cast_and_gcd(h, caster);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(SWIFTMEND)
                && event.source == Some(caster.to_bits())
                && event.target == Some(patient.to_bits()))
    );
}

fn native_buff_multiplier(h: &Harness) -> f32 {
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(SOUL_OF_THE_FOREST)
        .unwrap();
    let amount = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    assert_eq!(amount, 60.0, "native114108s1 source amount");
    1.0 + amount / 100.0
}

// Independent of the new named multiplier: use ordinary native spell, caster
// level/healing power, existing spell mods and stat multipliers for each actor.
fn ordinary_heal_bounds(
    h: &Harness,
    caster: Entity,
    spell: &SpellInfo,
    periodic: bool,
) -> (f32, f32) {
    let world = h.app.world();
    let effect = spell
        .effects
        .iter()
        .find(|effect| {
            if periodic {
                effect.aura == 8
            } else {
                effect.effect == 10
            }
        })
        .unwrap();
    let base = effects::scaled_amount(world, caster, spell, effect);
    // Existing periodic runtime snapshots a fixed amount; direct heals roll
    // native SpellEffect variance/2 (TC SpellInfo::CalcValue).
    let variance = if periodic {
        0.0
    } else {
        effect.variance.abs() / 2.0
    };
    let op = if periodic {
        SpellModOp::PeriodicHealingAndDamage
    } else {
        SpellModOp::HealingAndDamage
    };
    let stats = world.get::<AuraStatMods>(caster);
    let done = stats.map_or(1.0, |mods| mods.healing_done_mult);
    let taken = stats.map_or(1.0, |mods| mods.healing_taken_mult);
    let multiplier = done * taken * effects::versatility_multiplier(world, caster);
    let output = |amount| spell_mods::modify(world, caster, spell, op, amount) * multiplier;
    (
        output(base * (1.0 - variance)),
        output(base * (1.0 + variance)),
    )
}

struct HealObservation {
    amount_without_crit: f32,
    ordinary_bounds: (f32, f32),
    native_buff_multiplier: f32,
}

fn observe_next_heal(level: u8, spell_id: u32, empowered: bool) -> HealObservation {
    let (mut h, caster) = at_level(CRENNA, level, |_| {});
    assert_eq!(
        crate::auras::has_aura(h.app.world(), caster, TALENT),
        empowered
    );
    if empowered {
        cast_setup_swiftmend(&mut h, caster, caster);
        assert!(crate::auras::has_aura(
            h.app.world(),
            caster,
            SOUL_OF_THE_FOREST
        ));
    }
    let mut health = h.app.world_mut().get_mut::<Health>(caster).unwrap();
    health.current = health.max * 0.1;
    let periodic = spell_id == REJUVENATION;
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(spell_id)
        .unwrap()
        .clone();
    let ordinary_bounds = ordinary_heal_bounds(&h, caster, &spell, periodic);
    let crit_multiplier = 1.0
        + spell_mods::modify(
            h.app.world(),
            caster,
            &spell,
            SpellModOp::CritDamageAndHealing,
            1.0,
        );
    let multiplier = native_buff_multiplier(&h);
    let start = h.inbox.log.len();
    creature_cast(h.app.world_mut(), caster, spell_id, Some(caster), false).unwrap();
    let mut observed: Option<CombatLogEvent> = None;
    for _ in 0..300 {
        h.advance(1);
        observed = h.inbox.log[start..]
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(spell_id)
                    && event.source == Some(caster.to_bits())
                    && event.target == Some(caster.to_bits())
                    && event.periodic == periodic
            })
            .cloned();
        if observed.is_some() {
            break;
        }
    }
    let observed = observed.expect("ordinary native heal receipt");
    assert!(
        !crate::auras::has_aura(h.app.world(), caster, SOUL_OF_THE_FOREST),
        "the next eligible heal consumes the caster's buff"
    );
    // Heal.amount already includes overheal; do not add overflow twice.
    let amount = observed.amount as f32 / if observed.crit { crit_multiplier } else { 1.0 };
    HealObservation {
        amount_without_crit: amount,
        ordinary_bounds,
        native_buff_multiplier: multiplier,
    }
}

#[test]
#[ignore = "SOUL_OF_FOREST_NATIVE_PROC: genuine untalented level20 control"]
fn follower_soul_of_forest_level20_swiftmend_does_not_grant_buff() {
    let (mut h, caster) = at_level(CRENNA, 20, |_| {});
    assert!(!crate::auras::has_aura(h.app.world(), caster, TALENT));
    let patient = h.caster;
    cast_setup_swiftmend(&mut h, caster, patient);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        caster,
        SOUL_OF_THE_FOREST
    ));
    assert!(!crate::auras::has_aura(
        h.app.world(),
        patient,
        SOUL_OF_THE_FOREST
    ));
}

#[test]
#[ignore = "SOUL_OF_FOREST_NATIVE_PROC: actual learned level21 Swiftmend caster buff"]
fn follower_soul_of_forest_level21_swiftmend_grants_caster_buff() {
    let (mut h, caster) = at_level(CRENNA, 21, |_| {});
    assert!(crate::auras::has_aura(h.app.world(), caster, TALENT));
    let patient = h.caster;
    cast_setup_swiftmend(&mut h, caster, patient);
    assert!(crate::auras::has_aura(
        h.app.world(),
        caster,
        SOUL_OF_THE_FOREST
    ));
    assert!(!crate::auras::has_aura(
        h.app.world(),
        patient,
        SOUL_OF_THE_FOREST
    ));
}

#[test]
#[ignore = "SOUL_OF_FOREST_NATIVE_PROC: next eligible native heal consumes and snapshots60pct"]
fn follower_soul_of_forest_next_regrowth_and_rejuvenation_use_native60pct() {
    for spell in [REGROWTH, REJUVENATION] {
        let control = observe_next_heal(20, spell, false);
        let empowered = observe_next_heal(21, spell, true);
        let (control_low, control_high) = control.ordinary_bounds;
        let (talent_low, talent_high) = empowered.ordinary_bounds;
        let multiplier = empowered.native_buff_multiplier;
        println!(
            "SOUL_HEAL spell={spell} untalented20={} native20={:?} empowered21={} native21={:?} multiplier={multiplier}",
            control.amount_without_crit,
            control.ordinary_bounds,
            empowered.amount_without_crit,
            empowered.ordinary_bounds
        );
        // Native rounding is at most half a health point. Actor levels/stats
        // differ naturally; compare real receipts using each native baseline.
        assert!((control_low - 0.5..=control_high + 0.5).contains(&control.amount_without_crit));
        assert!(
            (talent_low * multiplier - 0.5..=talent_high * multiplier + 0.5)
                .contains(&empowered.amount_without_crit),
            "spell{spell}: control={}, empowered={}, native control bounds={:?}, talented bounds={:?}, native multiplier={multiplier}",
            control.amount_without_crit,
            empowered.amount_without_crit,
            control.ordinary_bounds,
            empowered.ordinary_bounds
        );
        assert!(control_low > 0.5);
        let ratio = empowered.amount_without_crit / control.amount_without_crit;
        let ratio_low = (talent_low * multiplier - 0.5) / (control_high + 0.5);
        let ratio_high = (talent_high * multiplier + 0.5) / (control_low - 0.5);
        assert!(
            (ratio_low..=ratio_high).contains(&ratio),
            "actual relative heal{ratio} outside native bounds{ratio_low}..{ratio_high}"
        );
    }
}
