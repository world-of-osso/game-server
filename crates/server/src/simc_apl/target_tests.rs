//! SimC76f8118 action.cpp2435–2468,2619,5362–5453: choose target before if.
use super::{Action, ActionFacts, Facts, Program};

struct Targets {
    // Native target-list order and concrete health/debuff facts.
    units: Vec<(u64, f64, f64)>,
}

impl Facts for Targets {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name == "active_enemies" {
            return Ok(self.units.len() as f64);
        }
        Err(format!("unknown actor operand {name}"))
    }
}

impl ActionFacts for Targets {
    fn target_ids(&self, _action: &Action) -> Result<Vec<u64>, String> {
        Ok(self.units.iter().map(|unit| unit.0).collect())
    }

    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        let id = match action.target {
            Some(id) => id,
            None => self.units.first().ok_or("no current target")?.0,
        };
        let (_, health, wound) = self
            .units
            .iter()
            .find(|unit| unit.0 == id)
            .ok_or_else(|| format!("unknown selected target {id}"))?;
        match name {
            "target.health.pct" => Ok(*health),
            "debuff.burning_wound.remains" => Ok(*wound),
            _ => self.value(name),
        }
    }
}

fn targets() -> Targets {
    Targets {
        units: vec![(41, 60.0, 3.0), (42, 10.0, 7.0), (43, 30.0, 5.0)],
    }
}

#[test]
fn simc_apl_target_min_max_select_real_candidate_identity() {
    for (mode, expected) in [("min", 42), ("max", 41)] {
        let literal = format!("execute,target_if={mode}:target.health.pct");
        let program = Program::parse(&[("default", &literal)]).unwrap();
        let selected = program
            .select("default", &targets(), &mut program.state(), |_| Ok(true))
            .unwrap()
            .unwrap();
        assert_eq!(selected.target, Some(expected));
    }
}

#[test]
fn simc_apl_target_ties_preserve_native_list_order() {
    let facts = Targets {
        units: vec![(42, 15.0, 0.0), (41, 15.0, 0.0)],
    };
    let program =
        Program::parse(&[("default", "execute,target_if=min:target.health.pct")]).unwrap();
    let selected = program
        .select("default", &facts, &mut program.state(), |_| Ok(true))
        .unwrap()
        .unwrap();
    assert_eq!(selected.target, Some(42));
}

#[test]
fn simc_apl_target_condition_failure_does_not_choose_second_best() {
    let program = Program::parse(&[
        (
            "default",
            "execute,target_if=min:debuff.burning_wound.remains,if=target.health.pct<20",
        ),
        ("default", "slam"),
    ])
    .unwrap();
    let selected = program
        .select("default", &targets(), &mut program.state(), |_| Ok(true))
        .unwrap()
        .unwrap();
    assert_eq!(
        selected.name, "slam",
        "the lowest-wound target fails if; do not try the second target"
    );
}

#[test]
fn simc_apl_target_run_selector_does_not_retarget_child_actions() {
    // Real pinned SimC probe: at15s main95%/add50%, the run selects an add,
    // but its child if<80 still sees the main; first MS is60.056s on main.
    // player.cpp2467 only changes list pointers, never child action targets.
    let facts = Targets {
        units: vec![(41, 95.0, 0.0), (42, 50.0, 0.0)],
    };
    let program = Program::parse(&[
        (
            "default",
            "run_action_list,name=execute,target_if=min:target.health.pct",
        ),
        ("execute", "mortal_strike,if=target.health.pct<80"),
    ])
    .unwrap();
    let selected = program
        .select("default", &facts, &mut program.state(), |action| {
            Ok(action.name == "mortal_strike")
        })
        .unwrap();
    assert_eq!(
        selected, None,
        "run's selected add must not become the child's target"
    );
}

#[test]
fn simc_apl_target_exhausted_nested_run_never_changes_callers_target() {
    let program = Program::parse(&[
        ("default", "call_action_list,name=aux"),
        ("default", "execute,if=target.health.pct>50"),
        (
            "aux",
            "run_action_list,name=empty,target_if=min:target.health.pct",
        ),
        ("empty", "mortal_strike"),
    ])
    .unwrap();
    let selected = program
        .select("default", &targets(), &mut program.state(), |action| {
            Ok(action.name == "execute")
        })
        .unwrap()
        .unwrap();
    assert_eq!(selected.name, "execute");
    assert_eq!(
        selected.target, None,
        "caller retains its native current target"
    );
}

#[test]
#[ignore = "queued sourced FIRST selector RED; not implemented yet"]
fn simc_apl_queued_first_selector_chooses_first_matching_native_target() {
    // SimC76f8118 action.cpp2954–2964,5411–5426: bare target_if is FIRST.
    let program =
        Program::parse(&[("default", "mortal_strike,target_if=target.health.pct<50")]).unwrap();
    let selected = program
        .select("default", &targets(), &mut program.state(), |_| Ok(true))
        .unwrap()
        .unwrap();
    assert_eq!(selected.name, "mortal_strike");
    assert_eq!(selected.target, Some(42));
}

#[test]
#[ignore = "queued sourced FIRST selector RED; not implemented yet"]
fn simc_apl_queued_first_selector_rejects_false_single_candidate() {
    // Unlike min/max's single-target shortcut, FIRST must still match.
    let facts = Targets {
        units: vec![(41, 60.0, 0.0)],
    };
    let program =
        Program::parse(&[("default", "mortal_strike,target_if=target.health.pct<50")]).unwrap();
    assert_eq!(
        program
            .select("default", &facts, &mut program.state(), |_| Ok(true))
            .unwrap(),
        None
    );
}

#[test]
#[ignore = "queued sourced cycle_targets RED; not implemented yet"]
fn simc_apl_queued_cycle_selects_first_condition_and_native_ready_target() {
    // action.cpp2472–2496 recursively checks each native target's action_ready.
    let program = Program::parse(&[
        (
            "default",
            "mortal_strike,cycle_targets=1,if=target.health.pct<50",
        ),
        ("default", "slam"),
    ])
    .unwrap();
    let selected = program
        .select("default", &targets(), &mut program.state(), |action| {
            Ok(action.name == "slam" || action.target == Some(43))
        })
        .unwrap()
        .unwrap();
    assert_eq!(selected.name, "mortal_strike");
    assert_eq!(
        selected.target,
        Some(43),
        "41 fails if;42 fails native admission;43 qualifies"
    );
}

#[test]
#[ignore = "source-qualified FIRST/cycle extension RED"]
fn simc_apl_first_cycle_explicit_first_preserves_native_order() {
    let program = Program::parse(&[(
        "default",
        "mortal_strike,target_if=first:target.health.pct<50",
    )])
    .unwrap();
    let selected = program
        .select("default", &targets(), &mut program.state(), |_| Ok(true))
        .unwrap()
        .unwrap();
    assert_eq!(selected.target, Some(42));
}

#[test]
#[ignore = "source-qualified FIRST/cycle extension RED"]
fn simc_apl_first_cycle_first_if_failure_does_not_try_second_match() {
    let program = Program::parse(&[
        (
            "default",
            "mortal_strike,target_if=first:target.health.pct<50,if=target.health.pct>20",
        ),
        ("default", "slam"),
    ])
    .unwrap();
    let selected = program
        .select("default", &targets(), &mut program.state(), |_| Ok(true))
        .unwrap()
        .unwrap();
    assert_eq!(
        selected.name, "slam",
        "FIRST selects42; failed if must not try43"
    );
    assert_eq!(selected.target, None);
}

#[test]
#[ignore = "source-qualified FIRST/cycle extension RED"]
fn simc_apl_first_cycle_first_native_refusal_does_not_try_second_match() {
    let program = Program::parse(&[
        (
            "default",
            "mortal_strike,target_if=first:target.health.pct<50",
        ),
        ("default", "slam"),
    ])
    .unwrap();
    let selected = program
        .select("default", &targets(), &mut program.state(), |action| {
            Ok(action.name == "slam" || action.target == Some(43))
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        selected.name, "slam",
        "FIRST is not a cycle-on-refusal selector"
    );
}

#[test]
#[ignore = "source-qualified FIRST/cycle extension RED; pinned binary asymmetry probe"]
fn simc_apl_first_cycle_negative_first_uses_pinned_single_and_multi_rules() {
    let program = Program::parse(&[("default", "mortal_strike,target_if=first:-1")]).unwrap();
    let single = Targets {
        units: vec![(41, 60.0, 0.0)],
    };
    assert_eq!(
        program
            .select("default", &single, &mut program.state(), |_| Ok(true))
            .unwrap(),
        None
    );
    let selected = program
        .select("default", &targets(), &mut program.state(), |_| Ok(true))
        .unwrap()
        .unwrap();
    assert_eq!(
        selected.target,
        Some(41),
        "single requires>0; multi accepts!=0 in pinned action.cpp"
    );
}

#[test]
#[ignore = "source-qualified FIRST/cycle extension RED"]
fn simc_apl_first_cycle_positive_cap_counts_native_candidates_not_successes() {
    for (cap, expected) in [
        (1, None),
        (2, None),
        (3, Some(43)),
        (0, Some(43)),
        (-1, Some(43)),
    ] {
        let row = format!(
            "mortal_strike,cycle_targets=1,max_cycle_targets={cap},if=target.health.pct<50"
        );
        let program = Program::parse(&[("default", &row)]).unwrap();
        let selected = program
            .select("default", &targets(), &mut program.state(), |action| {
                Ok(action.target == Some(43))
            })
            .unwrap();
        assert_eq!(
            selected.map(|action| action.target),
            expected.map(Some),
            "cap{cap}:41failsif,42failsadmission,43qualifies"
        );
    }
}

#[test]
#[ignore = "source-qualified FIRST/cycle extension RED"]
fn simc_apl_first_cycle_zero_flag_keeps_default_target() {
    let program = Program::parse(&[
        (
            "default",
            "mortal_strike,cycle_targets=0,max_cycle_targets=3,if=target.health.pct<50",
        ),
        ("default", "slam"),
    ])
    .unwrap();
    let selected = program
        .select("default", &targets(), &mut program.state(), |_| Ok(true))
        .unwrap()
        .unwrap();
    assert_eq!(selected.name, "slam");
    assert_eq!(selected.target, None);
}

#[test]
#[ignore = "source-qualified FIRST/cycle extension RED"]
fn simc_apl_first_cycle_exhaustion_restores_callers_default_target() {
    let program = Program::parse(&[
        (
            "default",
            "mortal_strike,cycle_targets=1,if=target.health.pct<0",
        ),
        ("default", "execute,if=target.health.pct>50"),
    ])
    .unwrap();
    let selected = program
        .select("default", &targets(), &mut program.state(), |_| Ok(true))
        .unwrap()
        .unwrap();
    assert_eq!(selected.name, "execute");
    assert_eq!(
        selected.target, None,
        "failed cycles cannot leak a candidate into next row"
    );
}

#[test]
#[ignore = "source-qualified FIRST/cycle extension RED"]
fn simc_apl_first_cycle_rejects_invalid_literal_iteration_options() {
    for row in [
        "mortal_strike,cycle_targets=2",
        "mortal_strike,cycle_targets=true",
        "mortal_strike,cycle_targets=1,max_cycle_targets=oops",
    ] {
        assert!(
            Program::parse(&[("default", row)]).is_err(),
            "strict pinned opt_bool/opt_int: {row}"
        );
    }
}

#[test]
#[ignore = "source-qualified FIRST/cycle extension RED"]
fn simc_apl_first_cycle_single_enemy_keeps_native_untargeted_context() {
    let facts = Targets {
        units: vec![(41, 30.0, 0.0)],
    };
    let program = Program::parse(&[(
        "default",
        "mortal_strike,cycle_targets=1,max_cycle_targets=1,if=target.health.pct<50",
    )])
    .unwrap();
    let selected = program
        .select("default", &facts, &mut program.state(), |action| {
            Ok(action.target.is_none())
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        selected.target, None,
        "native cycle branch needs global non-sleeping count>1"
    );
}

#[test]
#[ignore = "source-qualified FIRST/cycle extension RED"]
fn simc_apl_first_cycle_missing_native_candidate_adapter_is_explicit() {
    let facts = std::collections::BTreeMap::<String, f64>::from([("active_enemies".into(), 3.0)]);
    let program = Program::parse(&[("default", "mortal_strike,cycle_targets=1")]).unwrap();
    assert!(
        program
            .select("default", &facts, &mut program.state(), |_| Ok(true))
            .is_err()
    );
}

#[test]
#[ignore = "source-qualified selector/cycle native-binary precedence control"]
fn simc_apl_first_cycle_selector_takes_precedence_over_cycle_cap() {
    let program = Program::parse(&[(
        "default",
        "mortal_strike,target_if=max:-target.health.pct,cycle_targets=1,max_cycle_targets=1,if=target.health.pct<50",
    )])
    .unwrap();
    let selected = program
        .select("default", &targets(), &mut program.state(), |action| {
            Ok(action.target == Some(42))
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        selected.target,
        Some(42),
        "target_if selects42 beyond cycle cap1; cap cannot trim selector candidates"
    );
}

#[test]
fn simc_apl_target_unknown_fact_is_an_error_not_an_absent_target() {
    let program = Program::parse(&[("default", "execute,target_if=min:target.missing")]).unwrap();
    assert!(
        program
            .select("default", &targets(), &mut program.state(), |_| Ok(true))
            .is_err()
    );
}
