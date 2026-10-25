//! Shared hero-tree expression reads the actual native trait selection.
use super::*;
use server::simc_apl::Facts;

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec577 exact profile/traits"]
fn havoc_literal_hero_tree_reads_native_selection() {
    for count in [1, 3] {
        let run = prepare_literal(577, count).expect("native Havoc profile");
        let bindings = crate::simc_apl_native::Bindings::default();
        let facts = crate::simc_apl_native::NativeFacts::new(
            run.app.world(),
            run.actor,
            &bindings,
            crate::simc_apl_native::Encounter::Fixture {
                started_at: crate::spell_cast::now(run.app.world()),
                duration: 300.0,
                style: crate::simc_apl_native::FightStyle::Patchwerk,
            },
            run.targets.len(),
        );
        assert_eq!(facts.value("spell_targets").unwrap(), count as f64);
        assert_eq!(facts.value("hero_tree.felscarred").unwrap(), 1.0);
        assert_eq!(facts.value("hero_tree.aldrachi_reaver").unwrap(), 0.0);
        assert!(facts.value("hero_tree.misspelled").is_err());
    }
}
