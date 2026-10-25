//! Source-order variables and call lists from pinned Ret/WW APL rows.
use super::*;

#[test]
fn simc_apl_native_readiness_precedes_if_expression() {
    let program =
        Program::parse(&[("default", "missing_item,if=buff.unequipped_proc.up")]).unwrap();
    let facts = BTreeMap::<String, f64>::new();
    let mut state = program.state();
    assert!(
        program
            .select("default", &facts, &mut state, |_| Ok(false))
            .unwrap()
            .is_none()
    );
    assert!(
        program
            .select("default", &facts, &mut state, |_| Ok(true))
            .is_err()
    );
}

fn facts(rows: &[(&str, f64)]) -> BTreeMap<String, f64> {
    rows.iter()
        .map(|&(key, value)| (key.into(), value))
        .collect()
}

#[test]
fn simc_apl_ret_variables_preserve_both_setif_branches_and_initial_default() {
    //76f8118 apl_paladin.cpp21/23/27 and generators variable updates.
    let program = Program::parse(&[
        ("precombat", "variable,name=trinket_1_buffs,value=trinket.1.has_buff.strength|trinket.1.has_buff.mastery|trinket.1.has_buff.versatility|trinket.1.has_buff.haste|trinket.1.has_buff.crit"),
        ("precombat", "variable,name=trinket_1_sync,op=setif,value=1,value_else=0.5,condition=variable.trinket_1_buffs&(trinket.1.cooldown.duration%%cooldown.avenging_wrath.duration=0|cooldown.avenging_wrath.duration%%trinket.1.cooldown.duration=0)"),
        ("precombat", "variable,name=pre_es_wake,op=reset,default=0"),
        ("update", "variable,name=pre_es_wake,value=1"),
        ("reset", "variable,name=pre_es_wake,op=reset"),
    ]).unwrap();
    let mut state = program.state();
    assert_eq!(state.value("pre_es_wake"), Some(0.0));
    let mut rows = facts(&[
        ("trinket.1.has_buff.strength", 1.0),
        ("trinket.1.has_buff.mastery", 0.0),
        ("trinket.1.has_buff.versatility", 0.0),
        ("trinket.1.has_buff.haste", 0.0),
        ("trinket.1.has_buff.crit", 0.0),
        ("trinket.1.cooldown.duration", 120.0),
        ("cooldown.avenging_wrath.duration", 60.0),
    ]);
    assert_eq!(
        program
            .select("precombat", &rows, &mut state, |_| Ok(false))
            .unwrap(),
        None
    );
    assert_eq!(state.value("trinket_1_buffs"), Some(1.0));
    assert_eq!(state.value("trinket_1_sync"), Some(1.0));
    rows.insert("trinket.1.cooldown.duration".into(), 130.0);
    program
        .select("precombat", &rows, &mut state, |_| Ok(false))
        .unwrap();
    assert_eq!(state.value("trinket_1_sync"), Some(0.5));
    program
        .select("update", &rows, &mut state, |_| Ok(false))
        .unwrap();
    assert_eq!(state.value("pre_es_wake"), Some(1.0));
    program
        .select("reset", &rows, &mut state, |_| Ok(false))
        .unwrap();
    assert_eq!(state.value("pre_es_wake"), Some(0.0));
}

#[test]
fn simc_apl_ww_target_count_and_native_castability_select_source_order() {
    //MID2_Monk_Windwalker.simc default call rows and default_st spell rows.
    let program = Program::parse(&[
        (
            "default",
            "call_action_list,name=default_st,if=active_enemies=1",
        ),
        (
            "default",
            "call_action_list,name=multitarget,if=active_enemies>1",
        ),
        ("default_st", "rising_sun_kick,if=combo_strike"),
        ("default_st", "tiger_palm,if=combo_strike&chi<2"),
        (
            "multitarget",
            "spinning_crane_kick,if=combo_strike&buff.dance_of_chiji.up",
        ),
    ])
    .unwrap();
    let mut state = program.state();
    let mut rows = facts(&[
        ("active_enemies", 1.0),
        ("combo_strike", 1.0),
        ("chi", 1.0),
        ("buff.dance_of_chiji.up", 1.0),
    ]);
    let selected = program
        .select("default", &rows, &mut state, |_| Ok(true))
        .unwrap()
        .unwrap();
    assert_eq!(selected.name, "rising_sun_kick");
    let selected = program
        .select("default", &rows, &mut state, |action| {
            Ok(action.name != "rising_sun_kick")
        })
        .unwrap()
        .unwrap();
    assert_eq!(selected.name, "tiger_palm");
    rows.insert("active_enemies".into(), 3.0);
    let selected = program
        .select("default", &rows, &mut state, |_| Ok(true))
        .unwrap()
        .unwrap();
    assert_eq!(selected.name, "spinning_crane_kick");
    assert_eq!(
        program
            .select("default", &rows, &mut state, |_| Ok(false))
            .unwrap(),
        None
    );
}

#[test]
fn simc_apl_ret_finisher_variable_updates_before_native_action_selection() {
    //76f8118 apl_paladin.cpp46–49.
    let program = Program::parse(&[
        ("default", "call_action_list,name=finishers"),
        ("default", "judgment"),
        ("finishers", "variable,name=ds_castable,value=(active_enemies>=2|buff.divine_arbiter_verdict.up)&!buff.empyrean_legacy.up&!buff.divine_arbiter_divine_storm.up"),
        ("finishers", "divine_storm,if=variable.ds_castable"),
        ("finishers", "templars_verdict"),
    ]).unwrap();
    let mut state = program.state();
    let mut rows = facts(&[
        ("active_enemies", 3.0),
        ("buff.divine_arbiter_verdict.up", 0.0),
        ("buff.empyrean_legacy.up", 0.0),
        ("buff.divine_arbiter_divine_storm.up", 0.0),
    ]);
    assert_eq!(
        program
            .select("default", &rows, &mut state, |_| Ok(true))
            .unwrap()
            .unwrap()
            .name,
        "divine_storm"
    );
    assert_eq!(state.value("ds_castable"), Some(1.0));
    rows.insert("active_enemies".into(), 1.0);
    assert_eq!(
        program
            .select("default", &rows, &mut state, |_| Ok(true))
            .unwrap()
            .unwrap()
            .name,
        "templars_verdict"
    );
    assert_eq!(state.value("ds_castable"), Some(0.0));
    assert_eq!(
        program
            .select("default", &rows, &mut state, |action| Ok(
                action.name == "judgment"
            ))
            .unwrap()
            .unwrap()
            .name,
        "judgment"
    );
}

#[test]
fn simc_apl_explicit_empty_list_returns_to_the_next_caller_action() {
    let program = Program::parse(&[
        ("empty", ""),
        ("default", "call_action_list,name=empty"),
        ("default", "tiger_palm"),
    ])
    .unwrap();
    let selected = program
        .select("default", &facts(&[]), &mut program.state(), |_| Ok(true))
        .unwrap()
        .unwrap();
    assert_eq!(selected.name, "tiger_palm");
}

#[test]
fn simc_apl_invalid_control_does_not_become_an_empty_list_or_zero_variable() {
    assert!(Program::parse(&[("default", "variable,name=x,op=unknown,value=1")]).is_err());
    let program = Program::parse(&[("default", "tiger_palm,if=variable.undeclared")]).unwrap();
    let mut state = program.state();
    let error = program
        .select("default", &facts(&[]), &mut state, |_| Ok(true))
        .unwrap_err();
    assert!(error.contains("variable.undeclared"), "{error}");
    //76f8118 action.cpp4669–4676 rejects an unknown list during initialization.
    assert!(Program::parse(&[("default", "call_action_list,name=missing")]).is_err());
    //76f8118 player.cpp14251–14260 throws on a visited call-list cycle.
    let program = Program::parse(&[("default", "call_action_list,name=default")]).unwrap();
    let error = program
        .select("default", &facts(&[]), &mut program.state(), |_| Ok(true))
        .unwrap_err();
    assert!(error.contains("recursive APL call_action_list"), "{error}");
}
