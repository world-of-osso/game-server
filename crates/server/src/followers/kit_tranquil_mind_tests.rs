//! Finite-lifetime acquisition confidence, not a gameplay timer or frequency test.
use super::kit_clearcasting_tests::{
    complete_real_cast, frame_seconds, mana_cost, own_aura, wait_for_gcd,
};
use super::kit_tests::at_level;
use crate::auras::{aura_now, has_aura};
use crate::spell_cast::test_harness::Harness;
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use shared::components::Mana;

const CRENNA: u32 = 209072;
const OMEN: u32 = 113043;
const TRANQUIL_MIND: u32 = 403521;
const CLEARCASTING: u32 = 16870;
const LIFEBLOOM: u32 = 33763;
const REGROWTH: u32 = 8936;
const MISS_BUDGET: f64 = 1e-6;

fn native_effect(h: &Harness, spell: u32, index: u8) -> f32 {
    h.app
        .world()
        .resource::<SpellStore>()
        .get(spell)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .unwrap()
        .base_points
}

fn block_failure_probability(ticks: usize, needed_procs: usize, chance: f64) -> f64 {
    let mut choose = 1.0;
    (0..needed_procs)
        .map(|successes| {
            if successes > 0 {
                choose *= (ticks + 1 - successes) as f64 / successes as f64;
            }
            choose * chance.powi(successes as i32) * (1.0 - chance).powi((ticks - successes) as i32)
        })
        .sum()
}

fn count_natural_refreshes(
    h: &mut Harness,
    caster: Entity,
    cap: u8,
    needed_window: f64,
    buff_duration: f64,
) -> usize {
    complete_real_cast(h, caster, LIFEBLOOM);
    let hot = own_aura(h, caster, LIFEBLOOM).unwrap();
    let mut qualified_procs = 0;
    while own_aura(h, caster, LIFEBLOOM).is_some() {
        let before = own_aura(h, caster, CLEARCASTING).and_then(|aura| aura.expires_at);
        h.advance(1);
        if let Some(buff) = own_aura(h, caster, CLEARCASTING) {
            assert_eq!(buff.max_stacks, cap);
            assert!(
                buff.stacks <= cap,
                "real procs must never exceed native stack cap"
            );
            let enough_window = aura_now(h.app.world()) + buff_duration - hot.expires_at.unwrap()
                > needed_window + frame_seconds(h);
            if buff.expires_at != before && enough_window {
                qualified_procs += 1;
            }
        }
    }
    qualified_procs
}

fn earn_qualified_stack_block(level: u8, needed_procs: usize) -> (Harness, Entity) {
    let (mut h, caster) = at_level(CRENNA, level, |_| {});
    let talented = has_aura(h.app.world(), caster, TRANQUIL_MIND);
    assert_eq!(talented, level == 25);
    assert_eq!(native_effect(&h, TRANQUIL_MIND, 0), 1.0);
    assert_eq!(native_effect(&h, TRANQUIL_MIND, 2), 5.0);
    let chance = if talented {
        native_effect(&h, TRANQUIL_MIND, 2)
    } else {
        native_effect(&h, OMEN, 0)
    };
    let cap = if talented { 2 } else { 1 };
    let store = h.app.world().resource::<SpellStore>();
    let bloom = store.get(LIFEBLOOM).unwrap();
    let duration = f64::from(bloom.misc.as_ref().unwrap().duration.unwrap().duration_ms) / 1000.0;
    let period = f64::from(
        bloom
            .effects
            .iter()
            .find(|effect| effect.aura == 8)
            .unwrap()
            .aura_period_ms,
    ) / 1000.0;
    let buff_duration = f64::from(
        store
            .get(CLEARCASTING)
            .unwrap()
            .misc
            .as_ref()
            .unwrap()
            .duration
            .unwrap()
            .duration_ms,
    ) / 1000.0;
    let cast_time = f64::from(
        store
            .get(REGROWTH)
            .unwrap()
            .misc
            .as_ref()
            .unwrap()
            .cast_time
            .unwrap()
            .base_ms,
    ) / 1000.0;
    let needed_window = 2.0 * cast_time + frame_seconds(&h);
    let first_tick = ((duration - buff_duration + needed_window + frame_seconds(&h)) / period)
        .floor()
        .max(0.0) as usize
        + 1;
    let ticks_per_block = (duration / period).floor() as usize + 1 - first_tick;
    assert!(ticks_per_block >= needed_procs);
    let failure =
        block_failure_probability(ticks_per_block, needed_procs, f64::from(chance) / 100.0);
    let block_bound = (MISS_BUDGET.ln() / failure.ln()).ceil() as usize;
    println!(
        "TRANQUIL level{level}:native chance{chance}%,cap{cap},needed{needed_procs}procs/{ticks_per_block}late ticks,block failure{failure},bound{block_bound},miss budget{MISS_BUDGET}"
    );
    for block in 1..=block_bound {
        let procs = count_natural_refreshes(&mut h, caster, cap, needed_window, buff_duration);
        let ready = own_aura(&h, caster, CLEARCASTING).is_some_and(|buff| {
            buff.stacks == cap && buff.expires_at.unwrap() - aura_now(h.app.world()) > needed_window
        });
        if procs >= needed_procs && ready {
            println!("TRANQUIL acquired block{block},qualifiedprocs{procs},cap{cap}");
            return (h, caster);
        }
    }
    panic!(
        "TRANQUIL_NATURAL_ACQUISITION_PREREQUISITE: no qualifying native block within{block_bound}"
    );
}

#[test]
#[ignore = "TRANQUIL_MIND_NATIVE: actual24 remains cap1 through repeated natural procs"]
fn follower_tranquil_mind_level24_natural_refreshes_preserve_one_stack_cap() {
    let (h, caster) = earn_qualified_stack_block(24, 2);
    assert_eq!(own_aura(&h, caster, CLEARCASTING).unwrap().stacks, 1);
}

#[test]
#[ignore = "TRANQUIL_MIND_NATIVE: actual25 caps at2 even after third natural proc"]
fn follower_tranquil_mind_level25_third_natural_proc_does_not_exceed_two() {
    let (h, caster) = earn_qualified_stack_block(25, 3);
    assert_eq!(own_aura(&h, caster, CLEARCASTING).unwrap().stacks, 2);
}

#[test]
#[ignore = "TRANQUIL_MIND_NATIVE: two earned stacks buy exactly two ordinary Regrowths"]
fn follower_tranquil_mind_two_native_stacks_consume_one_per_free_regrowth() {
    let (mut h, caster) = earn_qualified_stack_block(25, 2);
    for remaining in [1, 0] {
        assert_eq!(mana_cost(&h, caster, REGROWTH), 0.0);
        let before = h.app.world().get::<Mana>(caster).unwrap().current;
        complete_real_cast(&mut h, caster, REGROWTH);
        assert!(h.app.world().get::<Mana>(caster).unwrap().current >= before);
        let actual = own_aura(&h, caster, CLEARCASTING).map_or(0, |buff| buff.stacks);
        assert_eq!(
            actual, remaining,
            "each completed free Regrowth consumes one actual stack"
        );
    }
    assert!(mana_cost(&h, caster, REGROWTH) > 0.0);
    wait_for_gcd(&mut h, caster, REGROWTH);
    let before = h.app.world().get::<Mana>(caster).unwrap().current;
    complete_real_cast(&mut h, caster, REGROWTH);
    assert!(h.app.world().get::<Mana>(caster).unwrap().current < before);
}
