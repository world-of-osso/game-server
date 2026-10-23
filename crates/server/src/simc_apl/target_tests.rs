//! SimC76f8118 action.cpp2435–2468,2619,5362–5453: choose target before if.
use super::{Action, ActionFacts, Facts, Program};

struct Targets {
    // Native target-list order and concrete health/debuff facts.
    units: Vec<(u64, f64, f64)>,
}

impl Facts for Targets {
    fn value(&self, name: &str) -> Result<f64, String> {
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
fn simc_apl_target_run_inherits_selected_target_into_child_conditions() {
    let program = Program::parse(&[
        (
            "default",
            "run_action_list,name=execute,target_if=min:target.health.pct",
        ),
        ("execute", "mortal_strike,if=target.health.pct>50"),
        ("execute", "execute"),
    ])
    .unwrap();
    let selected = program
        .select("default", &targets(), &mut program.state(), |action| {
            Ok(matches!(action.name.as_str(), "mortal_strike" | "execute"))
        })
        .unwrap()
        .unwrap();
    assert_eq!(selected.name, "execute");
    assert_eq!(selected.target, Some(42));
}

#[test]
fn simc_apl_target_exhausted_nested_run_restores_callers_target_context() {
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
fn simc_apl_target_unknown_fact_is_an_error_not_an_absent_target() {
    let program = Program::parse(&[("default", "execute,target_if=min:target.missing")]).unwrap();
    assert!(
        program
            .select("default", &targets(), &mut program.state(), |_| Ok(true))
            .is_err()
    );
}
