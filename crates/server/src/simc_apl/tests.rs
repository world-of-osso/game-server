//! Concrete frozen WW/Ret APL conditions; assertions concern numeric decisions.
use super::*;

fn facts(rows: &[(&str, f64)]) -> BTreeMap<String, f64> {
    rows.iter()
        .map(|&(name, value)| (name.to_owned(), value))
        .collect()
}

fn evaluate(source: &str, rows: &[(&str, f64)]) -> f64 {
    Expression::parse(source)
        .unwrap()
        .evaluate(&facts(rows))
        .unwrap()
}

#[test]
fn simc_apl_minmax_operators_preserve_source_precedence_and_shadow_target_cap() {
    // SimC76f8118 expressions.cpp130–145/413–416/1104–1106:
    // >? is minimum, <? is maximum; both bind below sums and above comparisons.
    for (source, expected) in [
        ("3>?12", 3.0),
        ("18>?12", 12.0),
        ("3<?12", 12.0),
        ("18<?12", 18.0),
        ("10+5>?12*2", 15.0),
        ("2+3<?4*2", 8.0),
        ("3>?12=3", 1.0),
        ("18<?12=18", 1.0),
    ] {
        assert_eq!(evaluate(source, &[]), expected, "{source}");
    }
    // expressions.cpp213–226 uses std::min/max: unordered comparisons keep left.
    for source in ["(0%0)>?12", "(0%0)<?12"] {
        assert!(evaluate(source, &[]).is_nan(), "{source}");
    }
    for source in ["12>?(0%0)", "12<?(0%0)"] {
        assert_eq!(evaluate(source, &[]), 12.0, "{source}");
    }
    // apl_priest.cpp aoe_variables: cap actual candidates, not a fixed pack size.
    for (targets, expected) in [(1.0, 1.0), (3.0, 3.0), (20.0, 12.0)] {
        assert_eq!(
            evaluate(
                "spell_targets.vampiric_touch>?12",
                &[("spell_targets.vampiric_touch", targets)]
            ),
            expected
        );
    }
}

#[test]
fn simc_apl_floor_rounds_down_preserves_grouping_and_binds_inner_facts() {
    for (literal, expected) in [
        ("floor(2.95)", 2.0),
        ("floor(-0.05)", -1.0),
        ("floor(2.95)+1", 3.0),
        ("floor(floor(2.95)+0.9)", 2.0),
    ] {
        assert_eq!(evaluate(literal, &[]), expected, "{literal}");
    }
    assert_eq!(evaluate("floor(energy%20)", &[("energy", 59.0)]), 2.0);
    assert!(Expression::parse("floor()").is_err());
    assert!(Expression::parse("floor(energy").is_err());
    assert!(
        Expression::parse("floor(unknown.energy)")
            .unwrap()
            .evaluate(&facts(&[]))
            .is_err()
    );
}

#[test]
fn simc_apl_ww_tiger_palm_literal_preserves_grouping_resources_and_stacks() {
    //76f8118 MID2_Monk_Windwalker.simc default_st Tiger Palm line69.
    let source = "combo_strike&!buff.zenith.up&chi<4+talent.ascension&energy.time_to_max<=gcd.max*3&(!buff.bloodlust.up|chi<1)&buff.combo_breaker.stack<2";
    let mut rows = facts(&[
        ("combo_strike", 1.0),
        ("buff.zenith.up", 0.0),
        ("chi", 3.0),
        ("talent.ascension", 1.0),
        ("energy.time_to_max", 4.0),
        ("gcd.max", 1.5),
        ("buff.bloodlust.up", 0.0),
        ("buff.combo_breaker.stack", 1.0),
    ]);
    let expression = Expression::parse(source).unwrap();
    assert_eq!(expression.evaluate(&rows).unwrap(), 1.0);
    rows.insert("buff.combo_breaker.stack".into(), 2.0);
    assert_eq!(expression.evaluate(&rows).unwrap(), 0.0);
    rows.insert("buff.combo_breaker.stack".into(), 1.0);
    rows.insert("buff.bloodlust.up".into(), 1.0);
    assert_eq!(expression.evaluate(&rows).unwrap(), 0.0);
    rows.insert("chi".into(), 0.0);
    assert_eq!(expression.evaluate(&rows).unwrap(), 1.0);
}

#[test]
fn simc_apl_ret_execution_sentence_literal_uses_native_fact_values() {
    //76f8118 apl_paladin.cpp43, ret-apl-condition-ledger-v1.json.
    let source = "(cooldown.avenging_wrath.remains>15|talent.radiant_glory)&(target.time_to_die>10)&(cooldown.wake_of_ashes.remains<gcd|variable.pre_es_wake=1)&(!talent.holy_flames|dot.expurgation.ticking)&(!talent.crusade|buff.avenging_wrath.stack>=7|variable.pre_es_wake=0)";
    let mut rows = facts(&[
        ("cooldown.avenging_wrath.remains", 16.0),
        ("talent.radiant_glory", 0.0),
        ("target.time_to_die", 11.0),
        ("cooldown.wake_of_ashes.remains", 0.0),
        ("gcd", 1.5),
        ("variable.pre_es_wake", 0.0),
        ("talent.holy_flames", 1.0),
        ("dot.expurgation.ticking", 1.0),
        ("talent.crusade", 1.0),
        ("buff.avenging_wrath.stack", 6.0),
    ]);
    let expression = Expression::parse(source).unwrap();
    assert_eq!(expression.evaluate(&rows).unwrap(), 1.0);
    rows.insert("dot.expurgation.ticking".into(), 0.0);
    assert_eq!(expression.evaluate(&rows).unwrap(), 0.0);
    rows.insert("dot.expurgation.ticking".into(), 1.0);
    rows.insert("target.time_to_die".into(), 10.0);
    assert_eq!(expression.evaluate(&rows).unwrap(), 0.0);
}

#[test]
fn simc_apl_ret_trinket_sync_keeps_percent_division_distinct_from_modulo() {
    //76f8118 apl_paladin.cpp23 and25.
    let modulo = "variable.trinket_1_buffs&(trinket.1.cooldown.duration%%cooldown.avenging_wrath.duration=0|cooldown.avenging_wrath.duration%%trinket.1.cooldown.duration=0)";
    let rows = [
        ("variable.trinket_1_buffs", 1.0),
        ("trinket.1.cooldown.duration", 120.0),
        ("cooldown.avenging_wrath.duration", 60.0),
    ];
    assert_eq!(evaluate(modulo, &rows), 1.0);
    assert_eq!(evaluate("120%60", &[]), 2.0);
    assert_eq!(evaluate("120%%60", &[]), 0.0);
    assert_eq!(evaluate("121%%60", &[]), 1.0);
    assert_eq!(evaluate("(120%20)*(1.5+1)*0.5", &[]), 7.5);
}

#[test]
fn simc_apl_numeric_truth_and_precedence_follow_pinned_expression_engine() {
    //expressions.cpp1081–1130: unary > multiply > add > compare > AND > OR.
    for (source, expected) in [
        ("1|0&0", 1.0),
        ("(1|0)&0", 0.0),
        ("!2+3*4=12", 1.0),
        ("2&3", 1.0),
        ("-2|0", 1.0),
        ("!0", 1.0),
        ("2!=3", 1.0),
        ("2<=2&3>=3", 1.0),
        ("-5%%3", -2.0),
    ] {
        assert_eq!(evaluate(source, &[]), expected, "{source}");
    }
}

#[test]
fn simc_apl_unknown_operand_never_becomes_false_even_in_an_inactive_branch() {
    let expression = Expression::parse("0&buff.not_a_bound_spell.up").unwrap();
    let error = expression.evaluate(&facts(&[])).unwrap_err();
    assert!(
        error.contains("unknown APL operand buff.not_a_bound_spell.up"),
        "{error}"
    );
}

#[test]
fn simc_apl_malformed_input_is_not_a_partial_or_empty_expression() {
    for source in [
        "",
        "chi<",
        "(chi>2",
        "chi>2)",
        "chi garbage",
        "buff..up",
        "2/1",
    ] {
        assert!(
            Expression::parse(source).is_err(),
            "accepted malformed/unsupported {source:?}"
        );
    }
}
