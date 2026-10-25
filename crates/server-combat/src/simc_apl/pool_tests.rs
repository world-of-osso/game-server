//! Concrete action-bound cost/maximum admission fixtures, never native pool writes.
use super::*;

struct PoolFacts {
    energy: f64,
    combo_points: f64,
    maximum_admitted: bool,
}

impl Facts for PoolFacts {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "energy" => Ok(self.energy),
            "energy.max" => Ok(100.0),
            "maximum_energy" => Ok(50.0),
            "combo_points" => Ok(self.combo_points),
            _ => Err(format!("unknown pool fixture operand {name}")),
        }
    }
}

impl ActionFacts for PoolFacts {
    fn value_for(&self, _action: &Action, name: &str) -> Result<f64, String> {
        self.value(name)
    }

    fn primary_resource(&self) -> Result<String, String> {
        Ok("energy".into())
    }

    fn ready_at_resource_max(&self, action: &Action, resource: &str) -> Result<bool, String> {
        assert_eq!(action.name, "ferocious_bite");
        assert_eq!(resource, "energy");
        Ok(self.maximum_admitted)
    }
}

fn select(rows: &[(&str, &str)], facts: &PoolFacts, usable: bool) -> Option<Action> {
    let program = Program::parse(rows).unwrap();
    program
        .select("default", facts, &mut program.state(), |action| {
            Ok(action.name == "fallback" || usable)
        })
        .unwrap()
}

#[test]
fn simc_apl_pool_wait_propagates_through_calls_without_dispatch_or_fallback() {
    let facts = PoolFacts {
        energy: 25.0,
        combo_points: 5.0,
        maximum_admitted: true,
    };
    let rows = [
        ("default", "call_action_list,name=finisher"),
        ("default", "fallback"),
        ("finisher", "pool_resource,for_next=1"),
        ("finisher", "ferocious_bite,max_energy=1,if=combo_points>=5"),
    ];
    assert_eq!(select(&rows, &facts, true), None);
    assert_eq!(
        facts.energy, 25.0,
        "selection cannot change the caller's pool"
    );
}

#[test]
fn simc_apl_pool_skips_when_next_action_is_ready_below_actor_pool_maximum() {
    let facts = PoolFacts {
        energy: 60.0,
        combo_points: 5.0,
        maximum_admitted: true,
    };
    let selected = select(
        &[
            ("default", "pool_resource,for_next=1"),
            ("default", "ferocious_bite,max_energy=1,if=combo_points>=5"),
        ],
        &facts,
        true,
    )
    .unwrap();
    assert_eq!(selected.name, "ferocious_bite");
    assert_eq!(selected.options["max_energy"], "1");
}

#[test]
fn simc_apl_pool_preserves_non_resource_admission_and_conditions() {
    for facts in [
        PoolFacts {
            energy: 25.0,
            combo_points: 5.0,
            maximum_admitted: false,
        },
        PoolFacts {
            energy: 25.0,
            combo_points: 4.0,
            maximum_admitted: true,
        },
    ] {
        let selected = select(
            &[
                ("default", "pool_resource,for_next=1"),
                ("default", "ferocious_bite,max_energy=1,if=combo_points>=5"),
                ("default", "fallback"),
            ],
            &facts,
            false,
        )
        .unwrap();
        assert_eq!(selected.name, "fallback");
    }
}

#[test]
fn simc_apl_pool_maximum_condition_reads_local_maximum_not_live_current() {
    let facts = PoolFacts {
        energy: 25.0,
        combo_points: 5.0,
        maximum_admitted: true,
    };
    assert_eq!(
        select(
            &[
                ("default", "pool_resource,for_next=1,resource=energy"),
                ("default", "ferocious_bite,if=energy>=50"),
                ("default", "fallback"),
            ],
            &facts,
            false
        ),
        None
    );
    assert_eq!(facts.energy, 25.0);
}

#[test]
fn simc_apl_max_energy_is_action_bound_not_full_actor_pool_or_a_dropped_flag() {
    for (energy, expected) in [(25.0, false), (49.0, false), (50.0, true), (60.0, true)] {
        let facts = PoolFacts {
            energy,
            combo_points: 5.0,
            maximum_admitted: true,
        };
        assert_eq!(
            select(&[("default", "ferocious_bite,max_energy=1")], &facts, true).is_some(),
            expected
        );
    }
}

#[test]
fn simc_apl_pool_rejects_missing_next_action_and_unqualified_options() {
    for rows in [
        vec![("default", "pool_resource,for_next=1")],
        vec![
            ("default", "pool_resource,for_next=1,force_wait=1"),
            ("default", "fallback"),
        ],
        vec![
            ("default", "pool_resource,for_next=1"),
            ("default", "variable,name=x,value=1"),
        ],
    ] {
        assert!(Program::parse(&rows).is_err());
    }
}
