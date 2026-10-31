//! Purified Spirit spends the remaining Harmony pool through real periodic damage.

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn refreshed_spender_splits_purified_spirit_between_targets() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 3).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1241059, Some(actor), false)
        .unwrap();
    let spender = run
        .app
        .world()
        .get::<crate::auras::harmony::HarmonyPools>(actor)
        .unwrap()
        .spender;
    assert!(spender > 0.0);
    run.app
        .world_mut()
        .entity_mut(actor)
        .remove::<crate::combat::AutoAttack>();
    for _ in 0..24 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1241059, Some(actor), false)
        .unwrap();
    for _ in 0..200 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let amounts: Vec<i32> = run
        .targets
        .iter()
        .map(|target| {
            events
                .iter()
                .filter(|e| {
                    e.kind == CombatLogKind::Damage
                        && e.spell_id == Some(450820)
                        && e.target == Some(target.to_bits())
                })
                .map(|e| e.amount)
                .sum()
        })
        .collect();
    assert!(
        amounts
            .iter()
            .all(|amount| (*amount as f32 - spender / 3.0).abs() < 5.0),
        "three targets share exactly one pool: {amounts:?}, pool {spender}"
    );
}

use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn purifying_brew_clarity_funds_purified_spirit() {
    use crate::class_progression::TraitLoadout;
    use crate::stats::{CharacterStats, StatModifiers, WeaponAttackType};

    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let world = run.app.world();
    assert!(
        world
            .get::<TraitLoadout>(actor)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 451017)
    );
    let stats = world.get::<CharacterStats>(actor).unwrap();
    let expected =
        3.0 * stats.total_attack_power(
            WeaponAttackType::MainHand,
            world.get::<StatModifiers>(actor),
        ) * (1.0 + stats.versatility_pct / 100.0);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 119582, Some(actor), false)
        .unwrap();
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1241059, Some(actor), false)
        .unwrap();
    run.app
        .world_mut()
        .entity_mut(actor)
        .remove::<crate::combat::AutoAttack>();
    for _ in 0..400 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let damage_for = |spell| -> i32 {
        events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(spell)
                    && event.target == Some(target.to_bits())
            })
            .map(|event| event.amount)
            .sum()
    };
    let harmony = damage_for(450763);
    let purified = damage_for(450820);
    assert!(purified > 0, "Clarity must fund Purified Spirit");
    assert!(
        ((harmony + purified) as f32 - expected).abs() < 5.0,
        "Harmony {harmony} + Purified Spirit {purified}, expected {expected}"
    );
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn expired_harmony_spender_deposits_remaining_pool_as_purified_spirit() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let accumulator = run
        .app
        .world()
        .get::<crate::auras::harmony::HarmonyPools>(actor)
        .unwrap()
        .accumulator;
    assert!(accumulator > 0.0);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1241059, Some(actor), false)
        .unwrap();
    let spender = run
        .app
        .world()
        .get::<crate::auras::harmony::HarmonyPools>(actor)
        .unwrap()
        .spender;
    assert!(spender > 0.0);
    run.app
        .world_mut()
        .entity_mut(actor)
        .remove::<crate::combat::AutoAttack>();
    for _ in 0..400 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let damage: i32 = events
        .iter()
        .filter(|e| {
            e.kind == CombatLogKind::Damage
                && e.spell_id == Some(450820)
                && e.target == Some(target.to_bits())
        })
        .map(|e| e.amount)
        .sum();
    println!("Purified Spirit spender={spender} damage={damage}");
    assert!(
        (damage as f32 - spender).abs() < 5.0,
        "expired spender must deliver exactly its unpaid pool: {damage} versus {spender}"
    );
    assert_eq!(
        run.app
            .world()
            .get::<crate::auras::harmony::HarmonyPools>(actor)
            .unwrap()
            .spender,
        0.0
    );
}
