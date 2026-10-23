//! Native selected416714 and naturally produced12654; no granted proc or stats.
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

use super::kit_proc_fidelity_tests::complete_cast;
use super::rotation_dummy_tests::{prepare_dummies, prepare_follower};
use crate::auras::{ActiveAuras, has_aura};
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
            "ordinary Fireball must produce this caster's actual Ignite on every target"
        );
    }
    (h, caster, targets)
}

fn assert_native_tick_produces_intensifying_flame(target_count: usize) {
    let _seed = crate::random::seed_scope(1);
    let (mut h, caster, targets) = prepare_ignites(target_count);
    let first = h.inbox.log.len();
    h.advance(45);
    for target in targets {
        let events = &h.inbox.log[first..];
        assert!(events.iter().any(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(IGNITE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }));
        assert!(
            events.iter().any(|event| {
                event.source == Some(caster.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(DAMAGE)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
            "actual Ignite tick must produce native419800 at {target_count} active own dots"
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
