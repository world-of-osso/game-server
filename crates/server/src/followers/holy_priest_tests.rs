//! Henry's actual 70448 starter loadout; no synthetic proc grants or cooldown resets.
use super::kit_clearcasting_tests::{complete_real_cast, mana_cost, own_aura, wait_for_gcd};
use super::kit_tests::at_level;
use crate::auras::has_aura;
use crate::spell_cast::creature_cast;
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::casting::CastState;

const HENRY: u32 = 215011;
const FLASH_HEAL: u32 = 2061;
const SERENITY: u32 = 2050;
const SURGE: u32 = 114255;
const APOTHEOSIS: u32 = 200183;
const SMITE: u32 = 585;

fn acquire_surge() -> (Harness, Entity) {
    let (mut h, priest) = at_level(HENRY, 14, |app| {
        app.init_resource::<crate::spell_cast::holy_metrics::HolyMetrics>();
    });
    assert!(has_aura(h.app.world(), priest, 109186));
    let chance = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(109186)
        .unwrap()
        .effects[0]
        .base_points;
    assert_eq!(chance, 8.0);
    // Finite natural acquisition confidence, not a gameplay proc guarantee.
    let bound = (1e-6_f64.ln() / (-f64::from(chance) / 100.0).ln_1p()).ceil() as usize;
    for attempt in 1..=bound {
        complete_real_cast(&mut h, priest, FLASH_HEAL);
        if own_aura(&h, priest, SURGE).is_some() {
            println!("Holy native Surge acquired after {attempt} casts (bound {bound})");
            return (h, priest);
        }
        // Ordinary creature mana regeneration remains enabled between attempts.
        h.advance(300);
    }
    panic!("HOLY_SURGE_NATIVE_PRODUCER_UNIMPLEMENTED: no proc in {bound} native healing casts");
}

#[test]
fn holy_priest_native_surge_is_instant_half_cost_and_spends_one_use() {
    let (mut h, priest) = acquire_surge();
    wait_for_gcd(&mut h, priest, FLASH_HEAL);
    let before = own_aura(&h, priest, SURGE).unwrap();
    let discounted = mana_cost(&h, priest, FLASH_HEAL);
    assert!(discounted > 0.0, "70448 Holy Surge is half-cost, not free");
    creature_cast(h.app.world_mut(), priest, FLASH_HEAL, Some(priest), false).unwrap();
    assert!(
        h.app.world().get::<CastState>(priest).is_none(),
        "native Surge Flash must be instant"
    );
    let remaining = own_aura(&h, priest, SURGE).map_or(0, |aura| aura.stacks);
    assert_eq!(
        remaining,
        before.stacks - 1,
        "one successful Flash spends exactly one use before another event can reproc"
    );
    if remaining == 0 {
        assert_eq!(mana_cost(&h, priest, FLASH_HEAL), discounted * 2.0);
    }
}

fn complete_targeted_smite(h: &mut Harness, priest: Entity, target: Entity) {
    wait_for_gcd(h, priest, SMITE);
    let start = h.inbox.go.len();
    creature_cast(h.app.world_mut(), priest, SMITE, Some(target), false).unwrap();
    for _ in 0..200 {
        h.advance(1);
        let completed = h.inbox.go[start..]
            .iter()
            .any(|event| event.caster == priest.to_bits() && event.spell_id == SMITE);
        if completed {
            h.advance(3);
            return;
        }
    }
    panic!("native Smite did not complete");
}

#[test]
fn holy_priest_native_two_surge_uses_keep_half_cost_then_spend_one() {
    let (mut h, priest) = at_level(HENRY, 14, |_| {});
    let paid_cost = mana_cost(&h, priest, FLASH_HEAL);
    // Two adjacent native8% rolls provide a conservative finite acquisition
    // bound; no proc chance, timer, pool or stack is changed by this fixture.
    let chance = 0.08_f64;
    let pair_bound = (1e-6_f64.ln() / (-(chance * chance)).ln_1p()).ceil() as usize;
    let mut target = h.npc;
    for attempt in 1..=pair_bound * 2 {
        if !crate::spell_cast::targets::is_alive(h.app.world(), target) {
            target = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 3.0);
        }
        complete_targeted_smite(&mut h, priest, target);
        if !own_aura(&h, priest, SURGE).is_some_and(|aura| aura.stacks == 2) {
            continue;
        }
        println!("Holy native two-use Surge acquired after {attempt} Smite casts");
        assert_eq!(
            mana_cost(&h, priest, FLASH_HEAL),
            paid_cost / 2.0,
            "two stored uses are not a doubled discount/free heal"
        );
        wait_for_gcd(&mut h, priest, FLASH_HEAL);
        creature_cast(h.app.world_mut(), priest, FLASH_HEAL, Some(priest), false).unwrap();
        assert_eq!(own_aura(&h, priest, SURGE).unwrap().stacks, 1);
        assert_eq!(mana_cost(&h, priest, FLASH_HEAL), paid_cost / 2.0);
        return;
    }
    panic!("HOLY_SURGE_TWO_USE_ACQUISITION: no native double proc in finite bound");
}

#[test]
fn holy_priest_native_proc_metrics_count_real_gains_and_consumption() {
    let (mut h, priest) = acquire_surge();
    wait_for_gcd(&mut h, priest, FLASH_HEAL);
    creature_cast(h.app.world_mut(), priest, FLASH_HEAL, Some(priest), false).unwrap();
    let metrics = h
        .app
        .world()
        .resource::<crate::spell_cast::holy_metrics::HolyMetrics>();
    let actor = metrics.actors.get(&priest.to_bits());
    assert!(actor.is_some(), "actual native proc must be observed");
    let actor = actor.unwrap();
    assert_eq!(actor.surge_roll_successes, 1);
    assert_eq!(actor.surge_effective_gains, 1);
    assert_eq!(actor.surge_uses, 1);
}

#[test]
fn holy_priest_native_apotheosis_restores_spent_serenity() {
    let (mut h, priest) = at_level(HENRY, 35, |_| {});
    complete_real_cast(&mut h, priest, SERENITY);
    complete_real_cast(&mut h, priest, APOTHEOSIS);
    assert!(has_aura(h.app.world(), priest, APOTHEOSIS));
    wait_for_gcd(&mut h, priest, SERENITY);
    assert!(
        creature_cast(h.app.world_mut(), priest, SERENITY, Some(priest), false).is_ok(),
        "HOLY_APOTHEOSIS_RESET_UNIMPLEMENTED: actual Apotheosis must restore the spent Holy Word"
    );
}
