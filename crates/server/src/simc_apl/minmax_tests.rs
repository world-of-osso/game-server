//! SimC76f8118 expressions.cpp132–146,213–229,1104–1106.
use super::*;

#[test]
fn simc_apl_ele_min_clamps_the_literal_target_formula() {
    let facts = BTreeMap::new();
    for (source, expected) in [
        ("3>?5", 3.0),
        ("7>?5", 5.0),
        ("2+3>?4*2", 5.0),
        ("3>?5<4", 1.0),
    ] {
        assert_eq!(
            Expression::parse(source).unwrap().evaluate(&facts).unwrap(),
            expected,
            "{source}"
        );
    }
    let expression = Expression::parse(
        "(spell_targets.chain_lightning>?5)*(2+(spell_targets.chain_lightning>?5)+2)",
    )
    .unwrap();
    for (targets, expected) in [(3.0, 21.0), (7.0, 45.0)] {
        assert_eq!(
            expression
                .evaluate(&BTreeMap::from([(
                    "spell_targets.chain_lightning".into(),
                    targets
                )]))
                .unwrap(),
            expected
        );
    }
}

#[test]
fn simc_apl_ele_max_uses_pinned_precedence() {
    let facts = BTreeMap::new();
    for (source, expected) in [
        ("3<?5", 5.0),
        ("7<?5", 7.0),
        ("2+3<?4*2", 8.0),
        ("10>?2+3<?8", 8.0),
        ("3<?5=5", 1.0),
    ] {
        assert_eq!(
            Expression::parse(source).unwrap().evaluate(&facts).unwrap(),
            expected,
            "{source}"
        );
    }
}
