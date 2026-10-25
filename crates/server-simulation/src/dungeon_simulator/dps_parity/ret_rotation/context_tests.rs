//! Native context controls independent of the pending shared add-event up fact.
use super::super::literal_rotation::execution::Prepared;
use super::context::Context;
use super::*;
use crate::simc_apl_native::{Encounter, FightStyle};
use server::simc_apl::{Action, ActionFacts, Facts};

#[test]
#[ignore = "native Ret Context must delegate real targets, equipment and nonmutating maximum-pool admission"]
fn dps_parity_ret_literal_native_context_preserves_native_facts_and_pool() {
    let mut run = super::super::ret_tier_tests::native_ret(4);
    let mut bindings = bindings();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        execution_actions(),
        false,
    )
    .unwrap();
    let facts = Context::read(
        run.app.world(),
        run.actor,
        &bindings,
        &equipment,
        &prepared,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: FightStyle::Patchwerk,
        },
        &run.targets,
    );
    let action = |name: &str| Action {
        name: name.into(),
        target: None,
        options: Default::default(),
    };
    let verdict = action("templars_verdict");
    // Pinned sc_paladin.cpp4402–4408 chooses HolyPower for Ret, not UI pool order.
    assert_eq!(facts.primary_resource().unwrap(), "holy_power");
    assert_eq!(facts.value("holy_power").unwrap(), 0.0);
    assert!(prepared.execution(&verdict, &equipment).unwrap().is_none());
    assert!(facts.ready_at_resource_max(&verdict, "holy_power").unwrap());
    assert_eq!(
        facts.value("holy_power").unwrap(),
        0.0,
        "readiness cannot grant power"
    );
    assert!(!facts.target_ready(&action("rebuke")).unwrap());
    assert_eq!(
        facts.target_ids(&action("judgment")).unwrap(),
        vec![run.targets[0].to_bits()]
    );
    assert_eq!(facts.value("equipped.algethar_puzzle_box").unwrap(), 0.0);
    for operand in [
        "holy_power_typo",
        "buff.art_of_war_typo.up",
        "raid_event.adds.missing",
    ] {
        assert!(
            facts.value(operand).is_err(),
            "missing native fact {operand}"
        );
    }
    assert!(
        facts
            .ready_at_resource_max(&verdict, "unknown_pool")
            .is_err()
    );
}
