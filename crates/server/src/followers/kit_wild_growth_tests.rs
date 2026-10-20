//! WILD_GROWTH_FALLOFF_SIMC_SHAPE_ON_NATIVE_COEFFICIENT; Retail shape uncertified.
use super::kit_tests::at_level;
use crate::auras::spell_mods::SpellModOp;
use crate::auras::{ActiveAuras, AuraStatMods, aura_now, has_aura, spell_mods};
use crate::spell_cast::test_harness::Harness;
use crate::spell_cast::{creature_cast, effects};
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use shared::components::Health;
use shared::protocol::CombatLogKind;

const CRENNA: u32 = 209072;
const WILD_GROWTH: u32 = 48438;
const UNSTOPPABLE_GROWTH: u32 = 382559;
const SIMC_DECAY: f32 = 0.07;

struct GrowthObservation {
    ticks: Vec<f32>,
    flat_native_amount: f32,
    coefficient: f32,
    period_ms: u32,
    duration_ms: u32,
    less_falloff: f32,
}

fn ordinary_flat_tick(h: &Harness, caster: Entity, spell: &SpellInfo) -> f32 {
    let world = h.app.world();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.aura == 8)
        .unwrap();
    let amount = effects::scaled_amount(world, caster, spell, effect);
    let modded = spell_mods::modify(
        world,
        caster,
        spell,
        SpellModOp::PeriodicHealingAndDamage,
        amount,
    );
    let stats = world.get::<AuraStatMods>(caster);
    modded
        * stats.map_or(1.0, |mods| mods.healing_done_mult)
        * stats.map_or(1.0, |mods| mods.healing_taken_mult)
        * effects::versatility_multiplier(world, caster)
}

fn own_growth(h: &Harness, caster: Entity) -> Option<crate::auras::AuraInstance> {
    h.app
        .world()
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == WILD_GROWTH && aura.caster == caster)
        .cloned()
}

fn tick_amounts(h: &Harness, caster: Entity, start: usize, crit_multiplier: f32) -> Vec<f32> {
    h.inbox.log[start..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Heal
                && event.periodic
                && event.spell_id == Some(WILD_GROWTH)
                && event.source == Some(caster.to_bits())
                && event.target == Some(caster.to_bits())
        })
        .map(|event| event.amount as f32 / if event.crit { crit_multiplier } else { 1.0 })
        .collect()
}

fn observe_native_growth(level: u8) -> GrowthObservation {
    let (mut h, caster) = at_level(CRENNA, level, |_| {});
    let talented = has_aura(h.app.world(), caster, UNSTOPPABLE_GROWTH);
    assert_eq!(talented, level == 43);
    let store = h.app.world().resource::<SpellStore>();
    let spell = store.get(WILD_GROWTH).unwrap().clone();
    let coefficient = spell
        .effects
        .iter()
        .find(|effect| effect.aura == 8)
        .unwrap()
        .bonus_coefficient;
    assert!((coefficient - 0.481).abs() <= f32::EPSILON);
    let talent = store.get(UNSTOPPABLE_GROWTH).unwrap();
    let native_pct = talent
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    assert_eq!(native_pct, 40.0);
    let less_falloff = if talented { native_pct / 100.0 } else { 0.0 };
    let flat_native_amount = ordinary_flat_tick(&h, caster, &spell);
    assert!(flat_native_amount > 0.0);
    let crit_multiplier = 1.0
        + spell_mods::modify(
            h.app.world(),
            caster,
            &spell,
            SpellModOp::CritDamageAndHealing,
            1.0,
        );
    let mut health = h.app.world_mut().get_mut::<Health>(caster).unwrap();
    health.current = health.max * 0.1;
    let start = h.inbox.log.len();
    creature_cast(h.app.world_mut(), caster, WILD_GROWTH, Some(caster), false).unwrap();
    let mut applied = None;
    for _ in 0..200 {
        h.advance(1);
        applied = own_growth(&h, caster);
        if applied.is_some() {
            break;
        }
    }
    let aura = applied.expect("ordinary native Wild Growth must apply its own real HoT");
    let period_ms = aura
        .effects
        .iter()
        .find(|effect| effect.aura_type == 8)
        .unwrap()
        .period_ms;
    assert!(period_ms > 0 && aura.duration_ms >= 2 * period_ms);
    let frame = match h.app.world().resource::<TimeUpdateStrategy>() {
        TimeUpdateStrategy::ManualDuration(step) => step.as_secs_f64(),
        _ => panic!("native qualification needs a deterministic frame duration"),
    };
    let remaining = aura.expires_at.unwrap() - aura_now(h.app.world());
    h.advance((remaining / frame).ceil() as usize + 1);
    assert!(own_growth(&h, caster).is_none());
    let ticks = tick_amounts(&h, caster, start, crit_multiplier);
    h.advance((f64::from(period_ms) / 1000.0 / frame).ceil() as usize + 1);
    assert_eq!(
        tick_amounts(&h, caster, start, crit_multiplier),
        ticks,
        "native Wild Growth must emit no heal after natural expiry"
    );
    GrowthObservation {
        ticks,
        flat_native_amount,
        coefficient,
        period_ms,
        duration_ms: aura.duration_ms,
        less_falloff,
    }
}

fn assert_policy_curve(observed: &GrowthObservation) {
    let decay = SIMC_DECAY * (1.0 - observed.less_falloff);
    assert_eq!(
        observed.ticks.len(),
        (observed.duration_ms / observed.period_ms) as usize
    );
    for (index, amount) in observed.ticks.iter().enumerate() {
        let fraction = (index + 1) as f32 * observed.period_ms as f32 / observed.duration_ms as f32;
        let coefficient = observed.coefficient + decay / 2.0 - decay * fraction;
        let expected = observed.flat_native_amount * coefficient / observed.coefficient;
        assert!(
            (expected - 0.5..=expected + 0.5).contains(amount),
            "policy tick{} actual={amount}, native flat={}, expected={expected}, coefficient={coefficient}",
            index + 1,
            observed.flat_native_amount
        );
    }
}

fn normalized_drop_bounds(observed: &GrowthObservation) -> (f32, f32) {
    let first = *observed.ticks.first().unwrap();
    let last = *observed.ticks.last().unwrap();
    let span =
        (observed.ticks.len() - 1) as f32 * observed.period_ms as f32 / observed.duration_ms as f32;
    let scale = observed.coefficient / observed.flat_native_amount / span;
    // Two rounded health receipts introduce at most one health point of drop uncertainty.
    ((first - last - 1.0) * scale, (first - last + 1.0) * scale)
}

#[test]
fn follower_wild_growth_level42_native_ticks_follow_simc_policy_shape() {
    let observed = observe_native_growth(42);
    println!(
        "WG42 ticks={:?}, native flat={}, coefficient={}",
        observed.ticks, observed.flat_native_amount, observed.coefficient
    );
    assert!(
        observed.ticks.first().unwrap() > observed.ticks.last().unwrap(),
        "native tooltip requires healing to decline, not repeat a constant snapshot"
    );
    assert_policy_curve(&observed);
}

#[test]
fn follower_wild_growth_level43_native_unstoppable_reduces_normalized_falloff() {
    let control = observe_native_growth(42);
    let talented = observe_native_growth(43);
    assert!(
        control.ticks.first().unwrap() > control.ticks.last().unwrap(),
        "baseline declining curve must qualify before evaluating talent reduction"
    );
    assert_policy_curve(&talented);
    let (base_low, base_high) = normalized_drop_bounds(&control);
    let (talent_low, talent_high) = normalized_drop_bounds(&talented);
    let native_factor = 1.0 - talented.less_falloff;
    println!(
        "WG decay control={base_low}..{base_high}, talented={talent_low}..{talent_high}, native factor={native_factor}"
    );
    assert!(base_low > 0.0);
    assert!(
        talent_low <= base_high * native_factor && talent_high >= base_low * native_factor,
        "learned native40pct must reduce normalized falloff by that amount"
    );
}

#[test]
fn follower_wild_growth_native_duration_and_expiry_stop_real_heals() {
    let observed = observe_native_growth(42);
    assert_eq!(
        observed.ticks.len(),
        (observed.duration_ms / observed.period_ms) as usize
    );
    assert!(observed.ticks.iter().all(|amount| *amount > 0.0));
}
