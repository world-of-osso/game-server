//! TC a352b1fa spell_shaman.cpp459–522/1871–1921: launch energize, actual controller amounts.
use super::kit_proc_fidelity_tests::complete_cast;
use super::kit_tests::at_level;
use crate::auras::{ActiveAuras, RemoveReason, has_aura};
use crate::spell_cast::powers::power_value;
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use bevy::prelude::*;
use shared::components::{Health, Position, PowerType};
use shared::protocol::{CombatLogEvent, CombatLogKind};

const CONTROLLER: u32 = 343725;

#[derive(Clone, Copy)]
struct ResourceCase {
    spell: u32,
    energize: u32,
    controller_effect: u8,
    targets: usize,
}

const LIGHTNING: ResourceCase = ResourceCase {
    spell: 188196,
    energize: 214815,
    controller_effect: 0,
    targets: 1,
};
const LIGHTNING_OVERLOAD: ResourceCase = ResourceCase {
    spell: 45284,
    energize: 214816,
    controller_effect: 1,
    targets: 1,
};
const CHAIN: ResourceCase = ResourceCase {
    spell: 188443,
    energize: 195897,
    controller_effect: 4,
    targets: 3,
};
const CHAIN_OVERLOAD: ResourceCase = ResourceCase {
    spell: 45297,
    energize: 218558,
    controller_effect: 5,
    targets: 3,
};

fn spawn_resource_fixture(case: ResourceCase) -> (Harness, Entity, Vec<Entity>) {
    let (mut h, caster) = at_level(209053, 70, |_| {});
    // Isolate one cast's resources without changing controller amounts or RNG.
    crate::auras::remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == 168534
    });
    assert!(has_aura(h.app.world(), caster, CONTROLLER));
    let origin = *h.app.world().get::<Position>(caster).unwrap();
    let distance = if case.targets == 1 { 35.0 } else { 8.0 };
    let mut targets = Vec::with_capacity(case.targets);
    for index in 0..case.targets {
        let target = if index == 0 {
            h.npc
        } else {
            spawn_npc(&mut h.app, 0.0)
        };
        let mut position = origin;
        position.x += distance + index as f32 * 2.0;
        h.app.world_mut().entity_mut(target).insert(position);
        let mut health = h.app.world_mut().get_mut::<Health>(target).unwrap();
        health.max = 1_000_000_000.0;
        health.current = health.max;
        targets.push(target);
    }
    (h, caster, targets)
}

fn controller_amount(h: &Harness, caster: Entity, index: u8) -> f32 {
    h.app
        .world()
        .get::<ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == CONTROLLER)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .unwrap()
        .amount
}

fn assert_damage_targets(
    events: &[CombatLogEvent],
    caster: Entity,
    case: ResourceCase,
    targets: &[Entity],
) {
    for target in targets {
        assert!(
            events
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.source == Some(caster.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(case.spell)),
            "native{} must actually damage each of its {} targets",
            case.spell,
            case.targets
        );
    }
}

fn energize_receipts(
    events: &[CombatLogEvent],
    caster: Entity,
    spell: u32,
) -> Vec<&CombatLogEvent> {
    events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Energize
                && event.source == Some(caster.to_bits())
                && event.target == Some(caster.to_bits())
                && event.spell_id == Some(spell)
        })
        .collect()
}

fn assert_native_launch_energize(case: ResourceCase) {
    let (mut h, caster, targets) = spawn_resource_fixture(case);
    let amount = controller_amount(&h, caster, case.controller_effect);
    let expected = amount * case.targets as f32;
    let (before, maximum) = power_value(h.app.world(), caster, PowerType::Maelstrom).unwrap();
    assert!(amount > 0.0 && before + expected <= maximum);
    let first = h.inbox.log.len();
    let target = h.npc;
    complete_cast(&mut h, caster, case.spell, target);
    let at_launch = power_value(h.app.world(), caster, PowerType::Maelstrom)
        .unwrap()
        .0;
    let launch_had_damage = h.inbox.log[first..].iter().any(|event| {
        event.kind == CombatLogKind::Damage
            && event.source == Some(caster.to_bits())
            && event.spell_id == Some(case.spell)
    });
    h.advance(40);
    let after = power_value(h.app.world(), caster, PowerType::Maelstrom)
        .unwrap()
        .0;
    assert_damage_targets(&h.inbox.log[first..], caster, case, &targets);
    assert_eq!(
        after - before,
        expected,
        "native{} controller effect{}={} ×{} targets must fund actual pool, not placeholder energize BP0",
        case.spell,
        case.controller_effect,
        amount,
        case.targets
    );
    if case.targets == 1 {
        assert!(
            !launch_had_damage,
            "35yd native bolt must still be in flight at its Go"
        );
        assert_eq!(
            at_launch - before,
            expected,
            "TC{} OnEffectLaunch grants resources before actual bolt impact",
            case.spell
        );
    }
    let receipts = energize_receipts(&h.inbox.log[first..], caster, case.energize);
    assert_eq!(
        receipts.len(),
        1,
        "one actual native energize{} receipt, not silent pool writes or one cast per Chain hit",
        case.energize
    );
    assert_eq!(receipts[0].amount, expected as i32);
    assert_eq!(receipts[0].overflow, 0);
    assert!(!receipts[0].periodic);
    println!(
        "NATIVE_LAUNCH_RESOURCE spell={} energize={} controller_effect={} amount={} targets={} pool_delta={} receipt={}",
        case.spell,
        case.energize,
        case.controller_effect,
        amount,
        case.targets,
        after - before,
        receipts[0].amount
    );
}

fn assert_removed_controller_blocks_energize(case: ResourceCase) {
    let (mut h, caster, targets) = spawn_resource_fixture(case);
    crate::auras::remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == CONTROLLER
    });
    assert!(!has_aura(h.app.world(), caster, CONTROLLER));
    let before = power_value(h.app.world(), caster, PowerType::Maelstrom)
        .unwrap()
        .0;
    let first = h.inbox.log.len();
    let target = h.npc;
    complete_cast(&mut h, caster, case.spell, target);
    h.advance(40);
    assert_damage_targets(&h.inbox.log[first..], caster, case, &targets);
    assert_eq!(
        power_value(h.app.world(), caster, PowerType::Maelstrom)
            .unwrap()
            .0,
        before
    );
    assert!(energize_receipts(&h.inbox.log[first..], caster, case.energize).is_empty());
}

#[test]
#[ignore = "native launch resource RED/GREEN pending"]
fn native_elemental_resource_lightning_launch_pool_and_receipt() {
    assert_native_launch_energize(LIGHTNING);
}
#[test]
#[ignore = "native launch resource removed-controller control pending"]
fn native_elemental_resource_lightning_without_controller() {
    assert_removed_controller_blocks_energize(LIGHTNING);
}
#[test]
#[ignore = "native launch resource RED/GREEN pending"]
fn native_elemental_resource_lightning_overload_launch_pool_and_receipt() {
    assert_native_launch_energize(LIGHTNING_OVERLOAD);
}
#[test]
#[ignore = "native launch resource removed-controller control pending"]
fn native_elemental_resource_lightning_overload_without_controller() {
    assert_removed_controller_blocks_energize(LIGHTNING_OVERLOAD);
}
#[test]
#[ignore = "native launch resource RED/GREEN pending"]
fn native_elemental_resource_chain_launch_pool_and_receipt() {
    assert_native_launch_energize(CHAIN);
}
#[test]
#[ignore = "native launch resource removed-controller control pending"]
fn native_elemental_resource_chain_without_controller() {
    assert_removed_controller_blocks_energize(CHAIN);
}
#[test]
#[ignore = "native launch resource RED/GREEN pending"]
fn native_elemental_resource_chain_overload_launch_pool_and_receipt() {
    assert_native_launch_energize(CHAIN_OVERLOAD);
}
#[test]
#[ignore = "native launch resource removed-controller control pending"]
fn native_elemental_resource_chain_overload_without_controller() {
    assert_removed_controller_blocks_energize(CHAIN_OVERLOAD);
}
