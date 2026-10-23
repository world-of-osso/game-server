//! Native selected hero-subtree facts, without inferring from learned passives.
use super::{Run, prepare};
use crate::class_progression::TraitLoadout;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use server::simc_apl::Facts;

fn hero_tree_value(run: &Run, operand: &str) -> Result<f64, String> {
    let bindings = Bindings::default();
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1).value(operand)
}

#[test]
#[ignore = "opt-in native Havoc selected hero tree; requires private parity inputs"]
fn havoc_native_hero_tree_reads_selected_subtree_not_learned_passives() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare(577, 1).expect("native Havoc BiS and active traits must load");
    assert_eq!(hero_tree_value(&run, "hero_tree.felscarred").unwrap(), 1.0);
    assert_eq!(
        hero_tree_value(&run, "hero_tree.aldrachi_reaver").unwrap(),
        0.0
    );
    assert!(hero_tree_value(&run, "hero_tree.no_such_native_subtree").is_err());
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 452408));
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(TraitLoadout::default());
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 452408));
    assert_eq!(hero_tree_value(&run, "hero_tree.felscarred").unwrap(), 0.0);
}

#[test]
#[ignore = "opt-in native Havoc selected hero tree; requires private parity inputs"]
fn havoc_native_hero_tree_reports_missing_loadout_instead_of_inferring_it() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare(577, 1).expect("native Havoc BiS and active traits must load");
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .remove::<TraitLoadout>();
    assert!(hero_tree_value(&run, "hero_tree.felscarred").is_err());
}
