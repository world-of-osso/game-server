use super::literal_rotation::execution::{Execution, execute};
use super::*;
use crate::spell_cast::powers::PowerPools;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_sweeping_strikes_adds_one_to_native_zero_base_chain_targets() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(71, 3).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(12294)
        .unwrap()
        .clone();
    assert_eq!(spell.effects[0].chain_targets, 0);
    let secondaries = |run: &mut Run| {
        crate::spell_cast::targets::chain_targets(
            run.app.world_mut(),
            actor,
            &spell,
            &spell.effects[0],
            primary,
            crate::spell_cast::targets::Relation::Enemy,
        )
    };
    assert!(secondaries(&mut run).is_empty(), "no unearned extra targets");
    execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(260708, actor, Some(actor)),
    )
    .unwrap();
    let recipients = secondaries(&mut run);
    assert_eq!(recipients.len(), 1, "native +1 adds one secondary to the existing explicit primary");
    assert!(run.targets.contains(&recipients[0]));
    assert_ne!(recipients[0], primary);
}

