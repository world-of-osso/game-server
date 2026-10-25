//! SimC76f8118 action.cpp4742–4763: run switches lists, call returns.
use super::Program;
use std::collections::BTreeMap;

#[test]
fn arms_apl_run_selects_its_child_before_later_parent_actions() {
    let program = Program::parse(&[
        (
            "default",
            "run_action_list,name=slayer_st,if=talent.slayers_dominance",
        ),
        ("default", "slam"),
        ("slayer_st", "mortal_strike"),
    ])
    .unwrap();
    let facts = BTreeMap::from([("talent.slayers_dominance".into(), 1.0)]);
    let selected = program
        .select("default", &facts, &mut program.state(), |action| {
            Ok(matches!(action.name.as_str(), "mortal_strike" | "slam"))
        })
        .unwrap()
        .unwrap();
    assert_eq!(selected.name, "mortal_strike");
}

#[test]
fn arms_apl_run_does_not_return_when_its_selected_child_is_exhausted() {
    let program = Program::parse(&[
        (
            "default",
            "run_action_list,name=slayer_st,if=talent.slayers_dominance",
        ),
        ("default", "slam"),
        ("slayer_st", "mortal_strike"),
    ])
    .unwrap();
    let facts = BTreeMap::from([("talent.slayers_dominance".into(), 1.0)]);
    let selected = program
        .select("default", &facts, &mut program.state(), |action| {
            Ok(action.name == "slam")
        })
        .unwrap();
    assert_eq!(
        selected, None,
        "run cannot fall through to the parent's Slam"
    );
}

#[test]
fn arms_apl_false_run_condition_and_exhausted_call_resume_parent() {
    for control in [
        "run_action_list,name=slayer_st,if=talent.slayers_dominance",
        "call_action_list,name=slayer_st",
    ] {
        let program = Program::parse(&[
            ("default", control),
            ("default", "slam"),
            ("slayer_st", "mortal_strike"),
        ])
        .unwrap();
        let facts = BTreeMap::from([("talent.slayers_dominance".into(), 0.0)]);
        let selected = program
            .select("default", &facts, &mut program.state(), |action| {
                Ok(action.name == "slam")
            })
            .unwrap()
            .unwrap();
        assert_eq!(selected.name, "slam");
    }
}

#[test]
fn arms_apl_run_rejects_an_unknown_child_list() {
    let result = Program::parse(&[("default", "run_action_list,name=missing")]);
    assert!(result.is_err(), "run must validate its list just like call");
}

#[test]
fn arms_apl_run_target_if_requires_an_explicit_target_adapter() {
    let program = Program::parse(&[
        (
            "default",
            "run_action_list,name=slayer_execute,target_if=min:target.health.pct",
        ),
        ("slayer_execute", "execute"),
    ])
    .unwrap();
    let result = program.select(
        "default",
        &BTreeMap::from([("target.health.pct".into(), 15.0)]),
        &mut program.state(),
        |_| Ok(true),
    );
    assert!(
        result.is_err(),
        "an actor-wide health scalar cannot select a target"
    );
}
