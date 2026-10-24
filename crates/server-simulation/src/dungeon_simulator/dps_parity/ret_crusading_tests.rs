//! TC Unit.cpp2280–2350: native Aura361 replaces a swing with its source spell.
use super::ret_tier_tests::{has, native_ret};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const CRUSADING_STRIKES: u32 = 404542;
const CRUSADING_DAMAGE: u32 = 408385;

fn start_attack(run: &mut Run) {
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
}

// Native white swings have no spell ID; replacement damage retains its child ID.
fn damage(run: &Run, spell: Option<u32>) -> i64 {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == spell
        })
        .filter(|event| event.kind == CombatLogKind::Damage)
        .map(|event| i64::from(event.amount))
        .sum()
}

#[test]
#[ignore = "native selected404542/Aura361 must replace white swings with real408385"]
fn dps_parity_ret_crusading_native_auto_attack_uses_source_override_child() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    assert!(has(&run, CRUSADING_STRIKES));
    start_attack(&mut run);
    for _ in 0..200 {
        run.app.update();
        if damage(&run, Some(CRUSADING_DAMAGE)) > 0 {
            break;
        }
    }
    assert!(
        damage(&run, Some(CRUSADING_DAMAGE)) > 0,
        "actual Aura361 must execute408385 on a native swing; ordinary damage={}",
        damage(&run, None)
    );
    assert_eq!(
        damage(&run, None),
        0,
        "replacement must not also deal normal white damage"
    );
}

#[test]
#[ignore = "native granted-only Ret control retains ordinary white swings without override"]
fn dps_parity_ret_crusading_native_unselected_keeps_white_swing() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    inputs::install_traits(run.app.world_mut(), run.actor, &[]).unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert!(!has(&run, CRUSADING_STRIKES));
    start_attack(&mut run);
    for _ in 0..200 {
        run.app.update();
        if damage(&run, None) > 0 {
            break;
        }
    }
    assert!(
        damage(&run, None) > 0,
        "actual unselected actor must keep ordinary attacks"
    );
    assert_eq!(damage(&run, Some(CRUSADING_DAMAGE)), 0);
}
