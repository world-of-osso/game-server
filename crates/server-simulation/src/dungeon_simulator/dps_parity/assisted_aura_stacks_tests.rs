//! Native assisted rule 19596 reads the player's actual Dance of Chi-Ji stacks.
use super::*;
use game_engine_core::assisted_combat::evaluate_rule;
use game_engine_core::cooldown_assist::AssistedRule;
use std::collections::BTreeSet;

#[test]
#[ignore = "requires native 269 player profile"]
fn assisted_269_rule19596_reads_live_aura_stacks() {
    let mut run = prepare(269, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let rule = AssistedRule {
        id: 19596,
        step_spell: 101546,
        condition_type: 54,
        values: [325202, 2, 0],
        field_11_1_7_60520_002: 0,
    };
    for expected_stacks in 0..=2 {
        let facts = crate::spell_cast::test_harness::read_assisted_parity_facts(
            run.app.world_mut(),
            actor,
            target,
            &BTreeSet::new(),
        )
        .unwrap();
        assert_eq!(
            facts
                .player_aura_stacks
                .as_ref()
                .unwrap()
                .get(&325202)
                .copied()
                .unwrap_or(0),
            expected_stacks,
        );
        assert_eq!(evaluate_rule(&rule, &facts), Ok(expected_stacks >= 2));
        if expected_stacks < 2 {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: actor,
                    target: actor,
                    spell_id: 325202,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
    }
}
