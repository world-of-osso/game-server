//! Native trigger/consumer boundaries independent of follower action selection.
use super::kit_clearcasting_tests::{frame_seconds, wait_for_gcd};
use super::kit_tests::at_level;
use crate::auras::{ActiveAuras, aura_now, has_aura};
use crate::spell_cast::test_harness::Harness;
use crate::spell_cast::{creature_cast, powers::power_value};
use bevy::prelude::*;
use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

fn prepare(entry: u32) -> (Harness, Entity) {
    let (mut h, caster) = at_level(entry, 90, |_| {});
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().max = 1_000_000_000.0;
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 1_000_000_000.0;
    (h, caster)
}

fn complete_cast(h: &mut Harness, caster: Entity, spell: u32, target: Entity) {
    wait_for_gcd(h, caster, spell);
    let first = h.inbox.go.len();
    creature_cast(h.app.world_mut(), caster, spell, Some(target), false).unwrap();
    for _ in 0..200 {
        h.advance(1);
        if h.inbox.go[first..]
            .iter()
            .any(|event| event.caster == caster.to_bits() && event.spell_id == spell)
        {
            return;
        }
    }
    panic!("native spell{spell} did not finish");
}

#[test]
fn proc_fidelity_1467_native_dragonrage_azure_grants_and_spends_essence_burst() {
    let (mut h, caster) = prepare(215240);
    let target = h.npc;
    complete_cast(&mut h, caster, 375087, caster);
    assert!(has_aura(h.app.world(), caster, 375087));
    complete_cast(&mut h, caster, 362969, target);
    assert!(
        has_aura(h.app.world(), caster, 359618),
        "native Dragonrage guarantees Azure Essence Burst"
    );
    let before = power_value(h.app.world(), caster, PowerType::Essence)
        .unwrap()
        .0;
    complete_cast(&mut h, caster, 356995, target);
    assert!(
        !has_aura(h.app.world(), caster, 359618),
        "the free native Disintegrate must spend its earned charge"
    );
    let after = power_value(h.app.world(), caster, PowerType::Essence)
        .unwrap()
        .0;
    assert!(after >= before, "free channel must not reduce Essence");
}

#[test]
fn proc_fidelity_1467_two_earned_stacks_fund_two_free_spenders() {
    let (mut h, caster) = prepare(215240);
    let target = h.npc;
    complete_cast(&mut h, caster, 375087, caster);
    complete_cast(&mut h, caster, 362969, target);
    complete_cast(&mut h, caster, 362969, target);
    let burst = super::kit_clearcasting_tests::own_aura(&h, caster, 359618).unwrap();
    assert_eq!(burst.stacks, 2, "native375722 adds one maximum stack");
    for (spender, remaining) in [(356995, 1), (357211, 0)] {
        let before = power_value(h.app.world(), caster, PowerType::Essence)
            .unwrap()
            .0;
        complete_cast(&mut h, caster, spender, target);
        let current = super::kit_clearcasting_tests::own_aura(&h, caster, 359618);
        assert_eq!(
            current.as_ref().map_or(0, |aura| aura.stacks),
            remaining,
            "one completed spender must spend exactly one earned Essence Burst"
        );
        if let Some(current) = current {
            assert_eq!(current.instance_id, burst.instance_id);
            assert_eq!(current.expires_at, burst.expires_at);
            assert_eq!(current.charges, 1);
        }
        assert!(
            power_value(h.app.world(), caster, PowerType::Essence)
                .unwrap()
                .0
                >= before
        );
        for _ in 0..200 {
            if h.app
                .world()
                .get::<shared::casting::CastState>(caster)
                .is_none()
            {
                break;
            }
            h.advance(1);
        }
        assert!(
            h.app
                .world()
                .get::<shared::casting::CastState>(caster)
                .is_none()
        );
    }
}

#[test]
fn proc_fidelity_63_native_fire_blast_crit_grants_heating_up_then_hot_streak() {
    let (mut h, caster) = prepare(209059);
    assert!(
        h.app
            .world()
            .get::<crate::class_progression::KnownSpells>(caster)
            .is_some_and(|known| known.contains(195283)),
        "the real selected native driver is learned even though Effect3 creates no aura"
    );
    assert!(!has_aura(h.app.world(), caster, 195283));
    let target = h.npc;
    for expected in [48107, 48108] {
        let start = h.inbox.log.len();
        complete_cast(&mut h, caster, 108853, target);
        assert!(
            h.inbox.log[start..]
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(108853)
                    && event.source == Some(caster.to_bits())
                    && event.crit),
            "native Fire Blast must produce the real critical hit used by this oracle"
        );
        assert!(
            has_aura(h.app.world(), caster, expected),
            "critical-hit causal buff{expected} missing"
        );
    }
    assert!(!has_aura(h.app.world(), caster, 48107));
    wait_for_gcd(&mut h, caster, 11366);
    creature_cast(h.app.world_mut(), caster, 11366, Some(target), false).unwrap();
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_none(),
        "earned Hot Streak makes the next native Pyroblast instant"
    );
    assert!(
        !has_aura(h.app.world(), caster, 48108),
        "instant Pyroblast must spend Hot Streak at cast, not missile impact"
    );
}

#[test]
fn proc_fidelity_262_surge_earned_during_lava_burst_survives_that_cast() {
    let (mut h, caster) = at_level(209053, 32, |_| {});
    let target = h.npc;
    creature_cast(h.app.world_mut(), caster, 51505, Some(target), false).unwrap();
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_some()
    );
    // Controlled native proc-spell application isolates its in-flight consumer.
    // This does not claim natural acquisition; that has a separate tick test.
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, 77762, Some(caster), 1);
    assert!(has_aura(h.app.world(), caster, 77762));
    for _ in 0..100 {
        h.advance(1);
        if h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_none()
        {
            break;
        }
    }
    assert!(
        has_aura(h.app.world(), caster, 77762),
        "a proc earned during the hardcast belongs to the next Lava Burst"
    );
    wait_for_gcd(&mut h, caster, 51505);
    creature_cast(h.app.world_mut(), caster, 51505, Some(target), false).unwrap();
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_none()
    );
    assert!(!has_aura(h.app.world(), caster, 77762));
}

#[test]
fn proc_fidelity_63_hot_streak_earned_during_pyroblast_is_not_spent_by_hardcast() {
    let (mut h, caster) = prepare(209059);
    let target = h.npc;
    creature_cast(h.app.world_mut(), caster, 11366, Some(target), false).unwrap();
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_some()
    );
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, 48108, Some(caster), 1);
    assert!(has_aura(h.app.world(), caster, 48108));
    for _ in 0..200 {
        h.advance(1);
        if h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_none()
        {
            break;
        }
    }
    assert!(
        has_aura(h.app.world(), caster, 48108),
        "hardcast did not receive Hot Streak's instant-cast modifier at admission"
    );
}

#[test]
fn proc_fidelity_262_natural_flame_shock_ticks_grant_lava_surge() {
    // Level32 still knows Flame Shock; level90 replaces it with Voltaic Blaze.
    let (mut h, caster) = at_level(209053, 32, |_| {});
    let target = h.npc;
    h.app.world_mut().get_mut::<Health>(target).unwrap().max = 1_000_000_000.0;
    h.app.world_mut().get_mut::<Health>(target).unwrap().current = 1_000_000_000.0;
    let begin = aura_now(h.app.world());
    let mut ticks = 0;
    while aura_now(h.app.world()) - begin < 120.0 {
        let own_dot = h
            .app
            .world()
            .get::<ActiveAuras>(target)
            .is_some_and(|auras| {
                auras
                    .iter()
                    .any(|aura| aura.spell_id == 188389 && aura.caster == caster)
            });
        if !own_dot {
            complete_cast(&mut h, caster, 470411, target);
        }
        let start = h.inbox.log.len();
        h.advance(1);
        ticks += h.inbox.log[start..]
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.periodic
                    && event.spell_id == Some(188389)
                    && event.source == Some(caster.to_bits())
            })
            .count();
        if has_aura(h.app.world(), caster, 77762) {
            println!(
                "LAVA_SURGE natural_ticks={ticks} elapsed={} frame={}",
                aura_now(h.app.world()) - begin,
                frame_seconds(&h)
            );
            wait_for_gcd(&mut h, caster, 51505);
            creature_cast(h.app.world_mut(), caster, 51505, Some(target), false).unwrap();
            assert!(
                h.app
                    .world()
                    .get::<shared::casting::CastState>(caster)
                    .is_none()
            );
            assert!(
                !has_aura(h.app.world(), caster, 77762),
                "the next native instant Lava Burst consumes the earned charge"
            );
            return;
        }
    }
    assert!(
        ticks >= 50,
        "need real periodic attempts, not an empty fixture"
    );
    panic!("no Lava Surge from{ticks} real ticks over120s");
}
