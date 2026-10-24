//! Literal Elemental decisions must reach real native casts, not just parsed rows.
use super::*;
use crate::class_progression::KnownSpells;

fn measure_native_elemental(count: usize, required_spell: u32) -> Value {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = elemental_rotation::prepare_measurement(count)
        .expect("pinned native Elemental profile, active traits and real precombat");
    assert!(
        run.app
            .world()
            .get::<KnownSpells>(run.actor)
            .unwrap()
            .contains(required_spell),
        "the selected native profile must actually know spell{required_spell} before decision qualification"
    );
    engage(&mut run);
    let (seconds, _, _) = advance_combat(&mut run).unwrap();
    assert_eq!(seconds, 300.0);
    report::combat(run.app.world(), &run.damage_targets, seconds)
}

fn assert_native_action_damage(combat: &Value, spell: u32) {
    let actions = combat["actions"].as_array().unwrap();
    let damage: u64 = actions
        .iter()
        .filter(|action| action["spell_id"].as_u64() == Some(u64::from(spell)))
        .map(|action| action["damage"].as_u64().unwrap())
        .sum();
    assert!(
        damage > 0,
        "literal Elemental spell{spell} must actually damage a native fixture target; observed {actions:?}"
    );
}

#[test]
#[ignore = "literal Elemental native ST decision RED/GREEN pending"]
fn literal_ele_native_st_executes_selected_elemental_blast() {
    let combat = measure_native_elemental(1, 117014);
    assert_native_action_damage(&combat, 117014);
}

#[test]
#[ignore = "literal Elemental native3T decision RED/GREEN pending"]
fn literal_ele_native_three_targets_execute_chain_lightning() {
    let combat = measure_native_elemental(3, 188443);
    assert_native_action_damage(&combat, 188443);
}
