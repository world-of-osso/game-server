//! Assisted enemy counts use live combat targets.
use super::*;
use game_engine_core::assisted_combat::evaluate_rule;
use game_engine_core::cooldown_assist::AssistedRule;
use std::collections::BTreeSet;

#[test]
#[ignore = "requires native 269 player profile"]
fn assisted_269_counts_active_combat_targets() {
    for count in [1, 3] {
        let mut run = prepare(269, count).unwrap();
        engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        let facts = crate::spell_cast::test_harness::read_assisted_parity_facts(
            run.app.world_mut(),
            actor,
            target,
            &BTreeSet::new(),
        )
        .unwrap();
        assert_eq!(facts.active_enemies, Some(count as u32));
        let rule = AssistedRule {
            id: 19874,
            step_spell: 100784,
            condition_type: 50,
            values: [2, 9, 0],
            field_11_1_7_60520_002: 0,
        };
        // SimC76f8118 player.cpp4035–4038: active_enemies <= value1.
        assert_eq!(evaluate_rule(&rule, &facts), Ok(count <= 2));
    }
}
