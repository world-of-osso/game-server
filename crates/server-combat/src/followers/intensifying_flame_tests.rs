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
const FIRE_BLAST: u32 = 108853;

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
    // Native Ignite lasts only9s. Use the actual two starting Fire Blast charges
    // after two Fireballs so all four banks coexist without refreshing their clocks.
    for (index, &target) in targets.iter().enumerate() {
        let spell = if index < 2 { FIREBALL } else { FIRE_BLAST };
        complete_cast(&mut h, caster, spell, target);
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
            "ordinary native hits must produce this caster's actual Ignite on every target: target={}, now={}, own events={:?}",
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
fn fire_intensifying_flame_native70_final_tick_script_isolation_credits_natural_expiry() {
    let _seed = crate::random::seed_scope(1);
    let (mut h, caster, targets) = prepare_ignites(1);
    let target = targets[0];
    // Test-only script isolation, after the ordinary Fireball earned this bank.
    // Unmodified TC-native child→Ignite feedback is asserted separately below.
    let mastery = h
        .app
        .world()
        .get::<ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 12846)
        .unwrap()
        .instance_id;
    crate::auras::remove_instance(
        h.app.world_mut(),
        caster,
        mastery,
        crate::auras::RemoveReason::Cancelled,
    );
    assert!(!has_aura(h.app.world(), caster, 12846));
    assert!(has_aura(h.app.world(), caster, TALENT));
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
    h.advance(frames);
    assert!(
        !has_aura(h.app.world(), caster, 12846),
        "test-only isolation must remain in force"
    );
    assert!(!has_aura(h.app.world(), target, IGNITE));
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
fn fire_intensifying_flame_native70_unmodified_child_reprocs_ignite_and_refreshes_deadline() {
    let _seed = crate::random::seed_scope(1);
    let (mut h, caster, targets) = prepare_ignites(1);
    let target = targets[0];
    let original_expiry = h
        .app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == IGNITE && aura.caster == caster)
        .unwrap()
        .expires_at
        .unwrap();
    let first = h.inbox.log.len();
    h.advance(45);
    assert!(has_aura(h.app.world(), caster, 12846));
    assert!(has_aura(h.app.world(), caster, TALENT));
    let refreshed_expiry = h
        .app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == IGNITE && aura.caster == caster)
        .unwrap()
        .expires_at
        .unwrap();
    let events: Vec<_> = h.inbox.log[first..]
        .iter()
        .filter(|event| {
            event.source == Some(caster.to_bits()) && event.target == Some(target.to_bits())
        })
        .collect();
    assert!(events.iter().any(|event| event.spell_id == Some(DAMAGE)
        && event.kind == CombatLogKind::Damage
        && event.amount > 0));
    assert!(
        !events.iter().any(|event| event.spell_id == Some(FIREBALL)),
        "no further root Fireball may account for this native refresh"
    );
    assert!(
        refreshed_expiry > original_expiry,
        "TC autogenerated native12846 has no419800 exclusion: original{original_expiry}, refreshed{refreshed_expiry}, events{events:?}"
    );
}

#[test]
#[ignore = "native Intensifying Flame source gate"]
fn fire_intensifying_flame_native70_four_actual_ignites_block_child_damage() {
    let _seed = crate::random::seed_scope(1);
    let (mut h, caster, targets) = prepare_ignites(4);
    let first = h.inbox.log.len();
    h.advance(45);
    assert!(
        targets.iter().all(|target| {
            h.app
                .world()
                .get::<ActiveAuras>(*target)
                .unwrap()
                .iter()
                .any(|aura| aura.spell_id == IGNITE && aura.caster == caster)
        }),
        "four actual own Ignite banks must remain live throughout the cap control"
    );
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
