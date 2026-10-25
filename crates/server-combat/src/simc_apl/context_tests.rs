//! The same named operand can have different values for different source actions.
use super::*;

struct NativeActionFixture {
    last_combo_action: String,
    actor: BTreeMap<String, f64>,
}

impl Facts for NativeActionFixture {
    fn value(&self, name: &str) -> Result<f64, String> {
        if matches!(name, "combo_strike" | "gcd") {
            return Err(format!("action context required for {name}"));
        }
        Facts::value(&self.actor, name)
    }
}

impl ActionFacts for NativeActionFixture {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            //sc_monk.cpp208–211/325–339: proposed action identity, not last action's state.
            "combo_strike" => Ok(if action.name != self.last_combo_action {
                1.0
            } else {
                0.0
            }),
            //action.cpp3284–3285 and4648+: control actions have trigger_gcd zero.
            "gcd" if action.name == "call_action_list" || action.name == "variable" => Ok(0.0),
            "gcd" => Facts::value(&self.actor, "native_spell_gcd"),
            _ => Facts::value(&self.actor, name),
        }
    }
}

#[test]
fn simc_apl_ww_combo_strike_compares_each_proposed_action_not_one_global_fact() {
    let program = Program::parse(&[
        ("default", "rising_sun_kick,if=combo_strike"),
        ("default", "tiger_palm,if=combo_strike&chi<2"),
    ])
    .unwrap();
    let mut facts = NativeActionFixture {
        last_combo_action: "rising_sun_kick".into(),
        actor: BTreeMap::from([("chi".into(), 1.0)]),
    };
    let mut state = program.state();
    assert_eq!(
        program
            .select("default", &facts, &mut state, |_| Ok(true))
            .unwrap()
            .unwrap()
            .name,
        "tiger_palm"
    );
    facts.last_combo_action = "tiger_palm".into();
    assert_eq!(
        program
            .select("default", &facts, &mut state, |_| Ok(true))
            .unwrap()
            .unwrap()
            .name,
        "rising_sun_kick"
    );
}

#[test]
fn simc_apl_ret_call_list_uses_its_own_zero_gcd_before_choosing_a_finisher() {
    //Literal76f8118 apl_paladin.cpp51.
    let program = Program::parse(&[
        ("default", "call_action_list,name=finishers,if=holy_power=5&cooldown.wake_of_ashes.remains|buff.hammer_of_light_free.remains<gcd*2"),
        ("default", "judgment"),
        ("finishers", "templars_verdict"),
    ]).unwrap();
    let mut facts = NativeActionFixture {
        last_combo_action: String::new(),
        actor: BTreeMap::from([
            ("holy_power".into(), 2.0),
            ("cooldown.wake_of_ashes.remains".into(), 1.0),
            ("buff.hammer_of_light_free.remains".into(), 1.0),
            ("native_spell_gcd".into(), 1.5),
        ]),
    };
    let mut state = program.state();
    assert_eq!(
        program
            .select("default", &facts, &mut state, |_| Ok(true))
            .unwrap()
            .unwrap()
            .name,
        "judgment"
    );
    facts.actor.insert("holy_power".into(), 5.0);
    assert_eq!(
        program
            .select("default", &facts, &mut state, |_| Ok(true))
            .unwrap()
            .unwrap()
            .name,
        "templars_verdict"
    );
}
