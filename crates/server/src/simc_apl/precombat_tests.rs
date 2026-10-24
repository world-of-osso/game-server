//! A precombat pass consumes source rows once, unlike a combat priority scan.
use super::*;

#[test]
fn simc_apl_precombat_advances_ready_actions_once_in_source_order() {
    let program = Program::parse(&[
        ("precombat", "snapshot_stats"),
        ("precombat", "cat_form"),
        ("precombat", "prowl"),
    ])
    .unwrap();
    let mut variables = program.state();
    let mut cursor = 0;
    let facts = BTreeMap::<String, f64>::new();
    for name in ["snapshot_stats", "cat_form", "prowl"] {
        let action = program
            .select_next("precombat", &mut cursor, &facts, &mut variables, |_| {
                Ok(true)
            })
            .unwrap()
            .unwrap();
        assert_eq!(action.name, name);
    }
    assert!(
        program
            .select_next("precombat", &mut cursor, &facts, &mut variables, |_| Ok(
                true
            ))
            .unwrap()
            .is_none()
    );
}

#[test]
fn simc_apl_precombat_does_not_revisit_an_unready_row_after_native_state_changes() {
    let program = Program::parse(&[
        ("precombat", "summon_pet"),
        ("precombat", "snapshot_stats"),
        ("precombat", "shadow_bolt"),
    ])
    .unwrap();
    let mut variables = program.state();
    let mut cursor = 0;
    let facts = BTreeMap::<String, f64>::new();
    let first = program
        .select_next("precombat", &mut cursor, &facts, &mut variables, |action| {
            Ok(action.name != "summon_pet")
        })
        .unwrap()
        .unwrap();
    assert_eq!(first.name, "snapshot_stats");
    let next = program
        .select_next("precombat", &mut cursor, &facts, &mut variables, |_| {
            Ok(true)
        })
        .unwrap()
        .unwrap();
    assert_eq!(next.name, "shadow_bolt");
}

#[test]
fn simc_apl_precombat_variables_sample_facts_at_their_single_source_position() {
    let program = Program::parse(&[
        (
            "precombat",
            "variable,name=opening_targets,value=active_enemies",
        ),
        ("precombat", "snapshot_stats"),
        ("precombat", "prowl,if=variable.opening_targets=1"),
    ])
    .unwrap();
    let mut variables = program.state();
    let mut cursor = 0;
    let mut facts: BTreeMap<String, f64> = BTreeMap::from([("active_enemies".into(), 1.0)]);
    let first = program
        .select_next("precombat", &mut cursor, &facts, &mut variables, |_| {
            Ok(true)
        })
        .unwrap()
        .unwrap();
    assert_eq!(first.name, "snapshot_stats");
    assert_eq!(variables.value("opening_targets"), Some(1.0));
    facts.insert("active_enemies".into(), 3.0);
    let next = program
        .select_next("precombat", &mut cursor, &facts, &mut variables, |_| {
            Ok(true)
        })
        .unwrap()
        .unwrap();
    assert_eq!(next.name, "prowl");
    assert_eq!(variables.value("opening_targets"), Some(1.0));
}
