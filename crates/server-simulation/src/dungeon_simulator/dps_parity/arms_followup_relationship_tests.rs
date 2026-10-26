//! Native lifecycle boundaries for the remaining selected Arms relationships.
use super::literal_rotation::execution::{Execution, execute};
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::powers::PowerPools;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::stats::CharacterStats;
use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

fn advance(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        run.app.update();
    }
}

fn prepare(targets: usize) -> (Run, f32) {
    let mut run = prepare_measurement(71, targets).unwrap();
    engage(&mut run);
    let ap = run
        .app
        .world()
        .get::<CharacterStats>(run.actor)
        .unwrap()
        .attack_power;
    (run, ap)
}

fn stacks(run: &Run, target: Entity, spell: u32) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(target)
        .map_or(0, |auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == spell && aura.caster == run.actor)
                .map_or(0, |aura| aura.stacks)
        })
}

fn rage(run: &Run) -> f32 {
    run.app
        .world()
        .get::<PowerPools>(run.actor)
        .unwrap()
        .get(PowerType::Rage)
        .unwrap()
        .current
}

fn set_rage(run: &mut Run, amount: f32) {
    run.app
        .world_mut()
        .get_mut::<PowerPools>(run.actor)
        .unwrap()
        .get_mut(PowerType::Rage)
        .unwrap()
        .current = amount;
}

fn set_probe_stats(run: &mut Run, ap: f32, critical: bool) {
    // Keep the native starting AP; force the two crit outcomes only in this fixture.
    let mut stats = run
        .app
        .world_mut()
        .get_mut::<CharacterStats>(run.actor)
        .unwrap();
    stats.attack_power = ap;
    stats.crit_pct = if critical { 1000.0 } else { -1000.0 };
}

fn heroic(run: &mut Run, ap: f32) {
    let actor = run.actor;
    let target = run.targets[0];
    set_rage(run, 1000.0);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1269391, Some(actor), 1);
    set_probe_stats(run, ap, false);
    execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(1269383, target, Some(target)),
    )
    .unwrap();
    advance(run, 40);
}

fn mortal_probe(run: &mut Run, ap: f32, critical: bool) -> f64 {
    let actor = run.actor;
    let target = run.targets[0];
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    set_probe_stats(run, ap, critical);
    let _seed = crate::random::seed_scope(70448);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 12294, Some(target), 1);
    advance(run, 2);
    let hits: Vec<_> = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(12294)
                && event.amount > 0
        })
        .collect();
    assert_eq!(hits.len(), 1, "one actual native Mortal Strike probe");
    assert_eq!(
        hits[0].crit, critical,
        "the fixture isolates the two real crit outcomes"
    );
    hits[0].amount as f64
}

fn warfare_damage(keep_modifiers: bool, critical: bool) -> (f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, ap) = prepare(1);
    heroic(&mut run, ap);
    assert_eq!(
        stacks(&run, run.actor, 1269394),
        1,
        "real Heroic grants one Warfare stack"
    );
    let native =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1269306, 0)
            / 10.0;
    if !keep_modifiers {
        // Preserve the buff's separate native armor penetration; neutralize only
        // its damage/crit effects for the paired behavioral control.
        let id = run
            .app
            .world()
            .get::<ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 1269394)
            .unwrap()
            .instance_id;
        for effect in [1, 2] {
            crate::auras::set_effect_amount(run.app.world_mut(), run.actor, id, effect, 0.0);
        }
    }
    (mortal_probe(&mut run, ap, critical), f64::from(native))
}

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_warfare_binds_native_damage_and_critical_damage_after_heroic() {
    let (normal_base, native) = warfare_damage(false, false);
    let (normal_buffed, _) = warfare_damage(true, false);
    let (crit_base, _) = warfare_damage(false, true);
    let (crit_buffed, _) = warfare_damage(true, true);
    let damage_ratio = normal_buffed / normal_base;
    let critical_ratio = crit_buffed / crit_base;
    assert!(
        (damage_ratio - (1.0 + native / 100.0)).abs() < 0.002
            && critical_ratio > damage_ratio + 0.005,
        "real Warfare damage/crit binding: normal={damage_ratio}, crit={critical_ratio}, native per-stack={native}"
    );
}

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_just_warming_up_grants_native_rage_once_after_aoe_smash() {
    let mut gains = Vec::new();
    let mut native = 0.0;
    for selected in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let (mut run, _) = prepare(3);
        let actor = run.actor;
        let target = run.targets[0];
        assert!(stacks(&run, actor, 1261048) > 0);
        native =
            crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 1261048, 0);
        if !selected {
            crate::auras::remove_auras(
                run.app.world_mut(),
                actor,
                crate::auras::RemoveReason::Consumed,
                |aura| aura.spell_id == 1261048,
            );
        }
        set_rage(&mut run, 0.0);
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(167105, target, Some(target)),
        )
        .unwrap();
        // Capture cast payment/gain before automatic white swings generate Rage.
        gains.push(rage(&run));
        advance(&mut run, 2);
        assert!(
            run.app
                .world()
                .resource::<ObservedCombatEvents>()
                .0
                .iter()
                .any(|event| {
                    event.source == Some(actor.to_bits())
                        && event.spell_id == Some(167105)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                }),
            "the actual AoE Colossus Smash must land"
        );
    }
    assert!(native > 0.0);
    assert_eq!(
        gains[1] - gains[0],
        native,
        "one native CS grant, not one per AoE victim: {gains:?}"
    );
}

fn heroic_window(accumulate_first: bool) -> (f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, ap) = prepare(1);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(stacks(&run, actor, 1269307) > 0);
    if !accumulate_first {
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(167105, target, Some(target)),
        )
        .unwrap();
        advance(&mut run, 40);
    }
    for _ in 0..2 {
        heroic(&mut run, ap);
    }
    if accumulate_first {
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(167105, target, Some(target)),
        )
        .unwrap();
        advance(&mut run, 40);
    }
    assert!(
        stacks(&run, target, 208086) > 0,
        "both controls have an active native CS wound"
    );
    let bonus = crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 1292058, 0);
    let base = crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 208086, 0);
    let expected = (100.0 + base + 2.0 * bonus) / (100.0 + base);
    (mortal_probe(&mut run, ap, false), f64::from(expected))
}

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_heroic_might_accumulates_for_next_smash_not_the_current_window() {
    let (pending, expected) = heroic_window(false);
    let (active, _) = heroic_window(true);
    let ratio = active / pending;
    assert!(
        (ratio - expected).abs() < 0.003,
        "two Heroics before CS activate the native additive bonus; Heroics after CS stay pending: {ratio} vs {expected}"
    );
}

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_winding_up_expires_all_stacks_after_slam_and_heroic() {
    for action in [1464, 1269383] {
        let _seed = crate::random::seed_scope(70448);
        let (mut run, ap) = prepare(1);
        let actor = run.actor;
        let target = run.targets[0];
        assert!(stacks(&run, actor, 1296644) > 0);
        set_rage(&mut run, 1000.0);
        for producer in [12294, 7384] {
            set_probe_stats(&mut run, ap, false);
            execute(
                run.app.world_mut(),
                actor,
                Execution::Spell(producer, target, Some(target)),
            )
            .unwrap();
            advance(&mut run, 40);
        }
        let before = stacks(&run, actor, 1300670);
        assert!(
            before >= 2,
            "real MS/Overpower must accumulate native Winding Up uses"
        );
        if action == 1269383 {
            crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1269391, Some(actor), 1);
        }
        set_rage(&mut run, 1000.0);
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(action, target, Some(target)),
        )
        .unwrap();
        advance(&mut run, 8);
        assert!(
            run.app
                .world()
                .resource::<ObservedCombatEvents>()
                .0
                .iter()
                .any(|event| {
                    event.source == Some(actor.to_bits())
                        && event.spell_id == Some(action)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                }),
            "the actual Winding Up consumer must land"
        );
        assert_eq!(
            stacks(&run, actor, 1300670),
            0,
            "Slam-family execution consumes all Winding Up stacks: action={action}, before={before}"
        );
    }
}

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_bloodletting_mortal_strike_applies_rend_only_below_native_threshold() {
    for fraction in [0.5, 0.2] {
        let _seed = crate::random::seed_scope(70448);
        let (mut run, ap) = prepare(1);
        let actor = run.actor;
        let target = run.targets[0];
        assert!(stacks(&run, actor, 383154) > 0);
        crate::auras::remove_auras(
            run.app.world_mut(),
            target,
            crate::auras::RemoveReason::Consumed,
            |aura| matches!(aura.spell_id, 772 | 388539),
        );
        {
            let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
            health.current = health.max * fraction;
        }
        let threshold =
            crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 383154, 2);
        set_rage(&mut run, 1000.0);
        set_probe_stats(&mut run, ap, false);
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(12294, target, Some(target)),
        )
        .unwrap();
        advance(&mut run, 2);
        assert!(
            run.app
                .world()
                .resource::<ObservedCombatEvents>()
                .0
                .iter()
                .any(|event| {
                    event.source == Some(actor.to_bits())
                        && event.spell_id == Some(12294)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                }),
            "the actual Bloodletting Mortal Strike must land"
        );
        assert_eq!(
            stacks(&run, target, 388539) > 0,
            fraction * 100.0 < threshold,
            "actual MS applies native Rend below Bloodletting's native threshold: {fraction}, {threshold}"
        );
    }
}
