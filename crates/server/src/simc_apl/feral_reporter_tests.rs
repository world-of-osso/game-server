//! Exact SimC76f8118 Feral rows: missing generic surfaces, no native fixture dependency.
use super::*;

struct FeralFacts;

impl Facts for FeralFacts {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "buff.shadowmeld.up" => Ok(0.0),
            "cooldown.convoke_the_spirits.charges_fractional" => Ok(0.0),
            "cooldown.convoke_the_spirits.duration" => Ok(120.0),
            "fight_remains" => Ok(300.0),
            "combo_points" => Ok(5.0),
            "energy" => Ok(25.0),
            "energy.max" => Ok(100.0),
            "dot.rake.refreshable" => Ok(1.0),
            _ => Err(format!("unknown APL operand {name}")),
        }
    }
}

impl ActionFacts for FeralFacts {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name == "dot.rake.refreshable" {
            return Ok(if action.target == Some(42) { 1.0 } else { 0.0 });
        }
        self.value(name)
    }

    fn target_ids(&self, _action: &Action) -> Result<Vec<u64>, String> {
        Ok(vec![41, 42])
    }
}

#[test]
fn feral_reporter_boolean_target_if_selects_first_refreshable_target() {
    let program = Program::parse(&[(
        "aoe_builder",
        "prowl,target_if=dot.rake.refreshable,if=!buff.shadowmeld.up",
    )])
    .expect("pinned boolean target_if must parse");
    let selected = program
        .select("aoe_builder", &FeralFacts, &mut program.state(), |_| {
            Ok(true)
        })
        .unwrap()
        .unwrap();
    assert_eq!(selected.name, "prowl");
    assert_eq!(selected.target, Some(42));
}

#[test]
fn feral_reporter_floor_sets_remaining_convoke_uses() {
    let program = Program::parse(&[(
        "cd_variable",
        "variable,name=convokeCountRemaining,value=floor(cooldown.convoke_the_spirits.charges_fractional+fight_remains%cooldown.convoke_the_spirits.duration-0.05)",
    )])
    .expect("pinned floor expression must parse");
    let mut state = program.state();
    assert!(
        program
            .select("cd_variable", &FeralFacts, &mut state, |_| Ok(false))
            .unwrap()
            .is_none()
    );
    assert_eq!(state.value("convokeCountRemaining"), Some(2.0));
}

#[test]
fn feral_reporter_pool_for_next_waits_instead_of_dispatching_control_row() {
    let program = Program::parse(&[
        ("finisher", "pool_resource,for_next=1"),
        ("finisher", "ferocious_bite,max_energy=1,if=combo_points>=5"),
    ])
    .unwrap();
    // Concrete affordability:25 Energy admits the ordinary25-cost Bite, but
    // not its native25 extra-Energy cap. Pooling is not a castable action.
    let selected = program
        .select("finisher", &FeralFacts, &mut program.state(), |_| Ok(true))
        .unwrap();
    assert_eq!(
        selected, None,
        "pool row must wait for the next max-Energy action"
    );
}

#[test]
fn feral_reporter_max_energy_refuses_bite_below_extra_energy_cap() {
    let program =
        Program::parse(&[("finisher", "ferocious_bite,max_energy=1,if=combo_points>=5")]).unwrap();
    let selected = program
        .select("finisher", &FeralFacts, &mut program.state(), |_| Ok(true))
        .unwrap();
    assert_eq!(
        selected, None,
        "max_energy must not disappear while ordinary castability is true at25 Energy"
    );
}
