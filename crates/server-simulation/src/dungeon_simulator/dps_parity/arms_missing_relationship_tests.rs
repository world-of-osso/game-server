//! Independent native boundaries for selected Arms cast/hit relationships.
use super::literal_rotation::execution::{Execution, execute};
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::PowerType;
use shared::protocol::{CombatLogEvent, CombatLogKind};

fn advance(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        run.app.update();
    }
}

fn owned_stacks(run: &Run, target: Entity, spell: u32) -> u8 {
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

fn damage(run: &Run, spell: u32) -> Vec<&CombatLogEvent> {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(spell)
                && event.amount > 0
        })
        .collect()
}

fn fund(run: &mut Run) {
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        run.actor,
        PowerType::Rage,
        1000.0,
    )
    .unwrap();
}

#[test]
#[ignore = "private current native DB and exact spec71 player inputs"]
fn arms_tactical_edge_colossus_smash_grants_native_sudden_death_uses() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(71, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(owned_stacks(&run, actor, 1261051) > 0);
    // Isolate the deterministic CS grant from the unrelated natural SD RPPM.
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| matches!(aura.spell_id, 52437 | 29725),
    );
    let expected =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 1261051, 0) as u8;
    assert!(expected > 0);
    execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(167105, target, Some(target)),
    )
    .unwrap();
    advance(&mut run, 2);
    assert!(
        !damage(&run, 167105).is_empty(),
        "the actual Colossus Smash must land"
    );
    assert_eq!(
        owned_stacks(&run, actor, 52437),
        expected,
        "selected Tactical Edge grants its native number of SD uses after CS"
    );
}

#[test]
#[ignore = "private current native DB and exact spec71 player inputs"]
fn arms_fatality_execute_consumes_owned_mark_for_noncritical_stack_scaled_damage() {
    let mut amounts = Vec::new();
    for stacks in [1, 3] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_measurement(71, 1).unwrap();
        engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        assert!(owned_stacks(&run, actor, 383703) > 0);
        assert!(crate::spell_cast::cast_triggered_with_overrides(
            run.app.world_mut(),
            actor,
            383704,
            Some(target),
            1,
            crate::spell_cast::TriggeredSpellOverrides {
                aura_stacks: Some(stacks),
                ..Default::default()
            },
        ));
        assert_eq!(owned_stacks(&run, target, 383704), stacks);
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 52437, Some(actor), 1);
        fund(&mut run);
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(281000, target, Some(target)),
        )
        .unwrap();
        advance(&mut run, 8);
        assert!(
            !damage(&run, 260798).is_empty(),
            "the real Execute damage child must land"
        );
        let hits = damage(&run, 383706);
        assert_eq!(
            hits.len(),
            1,
            "Execute must resolve one native Fatality hit for {stacks} marks"
        );
        assert_eq!(hits[0].target, Some(target.to_bits()));
        assert!(!hits[0].crit, "SimC Fatality cannot crit");
        amounts.push(hits[0].amount as f64);
        assert_eq!(
            owned_stacks(&run, target, 383704),
            0,
            "Fatality consumes the caster-owned marks"
        );
    }
    assert!(
        (amounts[1] / amounts[0] - 3.0).abs() < 0.2,
        "native damage scales with actual mark stacks, retaining native variance: {amounts:?}"
    );
}

#[test]
#[ignore = "private current native DB and exact spec71 player inputs"]
fn arms_mid2_concussive_slam_hits_only_secondary_targets_for_slam_and_heroic() {
    for action in [1464, 1269383] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_measurement(71, 3).unwrap();
        engage(&mut run);
        let actor = run.actor;
        let primary = run.targets[0];
        assert!(
            owned_stacks(&run, actor, 1296643) > 0,
            "the actual MID2 2pc is equipped"
        );
        fund(&mut run);
        if action == 1269383 {
            crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1269391, Some(actor), 1);
        }
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(action, primary, Some(primary)),
        )
        .unwrap();
        advance(&mut run, 40);
        assert!(
            !damage(&run, action).is_empty(),
            "real Slam-family action{action} must land"
        );
        let mut recipients: Vec<_> = damage(&run, 1300660)
            .iter()
            .map(|event| event.target.unwrap())
            .collect();
        recipients.sort_unstable();
        let mut expected: Vec<_> = run.targets[1..]
            .iter()
            .map(|target| target.to_bits())
            .collect();
        expected.sort_unstable();
        assert_eq!(
            recipients, expected,
            "MID2 Concussive Slam excludes the primary and hits each secondary once: action{action}"
        );
    }
}
