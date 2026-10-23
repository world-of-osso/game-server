//! Native selected416714 and naturally produced12654; no granted proc or stats.
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

use super::kit_clearcasting_tests::frame_seconds;
use super::kit_proc_fidelity_tests::complete_cast;
use super::rotation_dummy_tests::{prepare_dummies, prepare_follower};
use crate::auras::{ActiveAuras, aura_now, has_aura};
use crate::class_progression::KnownSpells;
use crate::spell_cast::test_harness::Harness;

const TALENT: u32 = 416714;
const IGNITE: u32 = 12654;
const DAMAGE: u32 = 419800;
const FIREBALL: u32 = 133;

fn prepare_ignites(target_count: usize) -> (Harness, Entity, Vec<Entity>) {
    let (mut h, caster) = prepare_follower(209059, 63, 70);
    assert!(
        h.app
            .world()
            .get::<KnownSpells>(caster)
            .unwrap()
            .contains(TALENT)
    );
    assert!(has_aura(h.app.world(), caster, TALENT));
    let targets = prepare_dummies(&mut h, target_count);
    for &target in &targets {
        complete_cast(&mut h, caster, FIREBALL, target);
        h.advance(20);
    }
    for &target in &targets {
        assert!(
            h.app
                .world()
                .get::<ActiveAuras>(target)
                .unwrap()
                .iter()
                .any(|aura| { aura.spell_id == IGNITE && aura.caster == caster }),
            "ordinary Fireball must produce this caster's actual Ignite on every target: target={}, now={}, own events={:?}",
            target.to_bits(),
            aura_now(h.app.world()),
            h.inbox
                .log
                .iter()
                .filter(|event| event.source == Some(caster.to_bits()))
                .collect::<Vec<_>>()
        );
    }
    (h, caster, targets)
}

fn assert_native_tick_produces_intensifying_flame(target_count: usize) {
    let _seed = crate::random::seed_scope(1);
    let (mut h, caster, targets) = prepare_ignites(target_count);
    let percent = h
        .app
        .world()
        .get::<ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == TALENT)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount;
    let expected: Vec<_> = targets
        .iter()
        .map(|&target| {
            let tick_amount = h
                .app
                .world()
                .get::<ActiveAuras>(target)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == IGNITE && aura.caster == caster)
                .unwrap()
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .unwrap()
                .amount;
            (target, tick_amount * percent / 100.0)
        })
        .collect();
    let first = h.inbox.log.len();
    h.advance(45);
    for (target, expected_damage) in expected {
        let events = &h.inbox.log[first..];
        assert!(events.iter().any(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(IGNITE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }));
        let child = events
            .iter()
            .find(|event| {
                event.source == Some(caster.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(DAMAGE)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .unwrap_or_else(|| {
                panic!(
                    "actual Ignite tick must produce native419800 at {target_count} active own dots"
                )
            });
        assert!(!child.crit, "native419800 has CANT_CRIT");
        assert!(
            (child.amount as f32 - expected_damage).abs() <= 1.0,
            "native419800 must use actual416714 effect1 percentage of the base tick: actual{}, expected{expected_damage}",
            child.amount
        );
    }
}

#[test]
#[ignore = "native Intensifying Flame source gate"]
fn fire_intensifying_flame_native70_one_actual_ignite_produces_child_damage() {
    assert_native_tick_produces_intensifying_flame(1);
}

#[test]
#[ignore = "native Intensifying Flame source gate"]
fn fire_intensifying_flame_native70_three_actual_ignites_allow_child_damage() {
    assert_native_tick_produces_intensifying_flame(3);
}

#[test]
#[ignore = "native Intensifying Flame source gate"]
fn fire_intensifying_flame_native70_final_tick_is_credited_before_natural_expiry() {
    let _seed = crate::random::seed_scope(1);
    let (mut h, caster, targets) = prepare_ignites(1);
    let target = targets[0];
    let ignite = h
        .app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == IGNITE && aura.caster == caster)
        .unwrap();
    let period_ms = ignite
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .period_ms;
    let native_ticks = ignite.duration_ms / period_ms;
    let original_expiry = ignite.expires_at.unwrap();
    let remaining = original_expiry - aura_now(h.app.world());
    let frames = (remaining / frame_seconds(&h)).ceil() as usize + 2;
    let first = h.inbox.log.len();
    let mut previous_expiry = original_expiry;
    let mut expiry_changes = Vec::new();
    for _ in 0..frames {
        h.advance(1);
        let expiry = h.app.world().get::<ActiveAuras>(target).and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == IGNITE && aura.caster == caster)
                .and_then(|aura| aura.expires_at)
        });
        if let Some(expiry) = expiry
            && expiry != previous_expiry
        {
            expiry_changes.push((aura_now(h.app.world()), expiry));
            previous_expiry = expiry;
        }
    }
    assert!(
        !has_aura(h.app.world(), target, IGNITE),
        "Ignite survives original expiry{original_expiry}; expiry_changes={expiry_changes:?}; own events={:?}",
        h.inbox.log[first..]
            .iter()
            .filter(|event| event.source == Some(caster.to_bits()))
            .collect::<Vec<_>>()
    );
    let count = |spell_id| {
        h.inbox.log[first..]
            .iter()
            .filter(|event| {
                event.source == Some(caster.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(spell_id)
            })
            .count()
    };
    assert_eq!(count(IGNITE), native_ticks as usize);
    assert_eq!(
        count(DAMAGE),
        native_ticks as usize,
        "every actual Ignite tick, including its expiry-frame tick, must credit its child"
    );
}

#[test]
#[ignore = "native Intensifying Flame source gate"]
fn fire_intensifying_flame_native70_four_actual_ignites_block_child_damage() {
    let _seed = crate::random::seed_scope(1);
    let (mut h, caster, _) = prepare_ignites(4);
    let first = h.inbox.log.len();
    h.advance(45);
    let own_damage: Vec<_> = h.inbox.log[first..]
        .iter()
        .filter(|event| {
            event.source == Some(caster.to_bits()) && event.kind == CombatLogKind::Damage
        })
        .collect();
    assert!(
        own_damage
            .iter()
            .any(|event| event.spell_id == Some(IGNITE))
    );
    assert!(
        !own_damage
            .iter()
            .any(|event| event.spell_id == Some(DAMAGE)),
        "native416714 effect0 caps this trigger at three active own Ignites"
    );
}
