//! SimC76f8118 sc_paladin.cpp1580–1590/1815–1935; native child identity406957.
use super::ret_tier_tests::{has, native_ret};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const TOLL: u32 = 375576;
const TOLL_JUDGMENT: u32 = 406957;
const HAMMER_MODE: u32 = 1277026;

fn has_child_damage(run: &Run, target: Entity) -> bool {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(TOLL_JUDGMENT)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
}

pub(super) fn qualify_toll(count: usize) -> Run {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    for index in 1..count {
        let target = spawn_dummy(&mut run.app, run.actor, index).unwrap();
        run.targets.push(target);
    }
    // Existing Divine Toll implicit-area policy admits actual combat units only.
    // Extra31146 targets must enter the same native combat relationship too.
    engage_fixture_opponents(&mut run);
    for _ in 0..4 {
        run.app.update();
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
    assert!(
        !has(&run, HAMMER_MODE),
        "normal Toll child branch must be active"
    );
    assert_eq!(
        literal_rotation::execution::resolve_action(run.app.world(), run.actor, TOLL).unwrap(),
        Some(TOLL)
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        TOLL,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    for _ in 0..200 {
        run.app.update();
        if run
            .targets
            .iter()
            .all(|&target| has_child_damage(&run, target))
        {
            break;
        }
    }
    let casts: Vec<_> = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
        })
        .map(|event| event.spell_id)
        .collect();
    assert!(
        casts.contains(&Some(TOLL)),
        "ordinary native parent must really cast: {casts:?}"
    );
    for &target in &run.targets {
        assert!(
            has_child_damage(&run, target),
            "native Toll must dispatch real406957 damage to each selected31146; casts={casts:?}"
        );
    }
    run
}

#[test]
#[ignore = "native Divine Toll selected single31146 must receive actual Judgment child"]
fn dps_parity_ret_toll_native_single_target_receives_real_judgment_child() {
    qualify_toll(1);
}

#[test]
#[ignore = "native Divine Toll selected three actual31146 must receive separate children"]
fn dps_parity_ret_toll_native_three_targets_receive_real_judgment_children() {
    qualify_toll(3);
}

#[test]
#[ignore = "native granted-only Ret cannot select an unpurchased Divine Toll"]
fn dps_parity_ret_toll_native_unselected_knowledge_does_not_admit_action() {
    let mut run = native_ret(4);
    inputs::install_traits(run.app.world_mut(), run.actor, &[]).unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert_eq!(
        literal_rotation::execution::resolve_action(run.app.world(), run.actor, TOLL).unwrap(),
        None
    );
}
