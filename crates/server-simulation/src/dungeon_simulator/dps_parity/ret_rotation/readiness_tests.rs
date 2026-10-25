//! SimC76f8118 sc_paladin.cpp1956–1962: Rebuke target_ready needs actual casting.
use super::super::literal_rotation::{
    Driver,
    execution::{Execution, Prepared},
};
use super::*;

fn select(
    run: &mut super::super::Run,
    driver: &mut Driver,
    bindings: &Bindings,
    equipment: &Equipment,
    list: &str,
) -> Option<(Action, Execution)> {
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        equipment,
        execution_actions(),
        driver.snapshot.is_none(),
    )
    .unwrap();
    let facts = with_equipment(super::tests::facts(run, bindings), equipment);
    driver.select(list, &facts, &prepared, equipment).unwrap()
}

#[test]
#[ignore = "native target-ready veto must preserve idle admission and select a real casting target"]
fn dps_parity_ret_literal_native_casting_target_selects_and_receives_rebuke() {
    let mut run = super::super::ret_tier_tests::native_ret(4);
    let mut bindings = bindings();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let probe = Action {
        name: "rebuke".into(),
        target: None,
        options: Default::default(),
    };
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        execution_actions(),
        false,
    )
    .unwrap();
    assert!(
        prepared.execution(&probe, &equipment).unwrap().is_some(),
        "native Rebuke admission on an idle target must stay unchanged"
    );
    let idle = with_equipment(super::tests::facts(&run, &bindings), &equipment);
    assert!(
        !idle.target_ready(&probe).unwrap(),
        "adapter policy vetoes only selection"
    );
    // Native NPC Shadow Bolt9613 has an authored cast time and no power row.
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.targets[0],
        9613,
        Some(run.actor),
        false,
    )
    .expect("native target must begin its actual Shadow Bolt without cast/power grants");
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.targets[0])
            .is_some()
    );
    let mut driver = Driver::new(server::simc_apl::Program::parse(&[("leaf", "rebuke")]).unwrap());
    let (selected, execution) = select(&mut run, &mut driver, &bindings, &equipment, "leaf")
        .expect("actual casting target must pass the declared Rebuke target-ready predicate");
    assert_eq!(selected.name, "rebuke");
    driver
        .execute(run.app.world_mut(), run.actor, selected, execution)
        .unwrap();
    for _ in 0..20 {
        run.app.update();
        if run
            .app
            .world()
            .get::<shared::casting::CastState>(run.targets[0])
            .is_none()
        {
            break;
        }
    }
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.targets[0])
            .is_none(),
        "ordinary native Rebuke must interrupt the real target cast"
    );
}

#[test]
#[ignore = "literal Ret target_ready: idle real31146 must not select unconditional Rebuke"]
fn dps_parity_ret_literal_native_idle_target_does_not_select_rebuke() {
    let mut run = super::super::ret_tier_tests::native_ret(4);
    let started_at = run.app.world().resource::<bevy::prelude::Time>().elapsed();
    super::super::install_fixture_lifetimes(&mut run, started_at);
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(96231),
        "source actor must actually know Rebuke"
    );
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.targets[0])
            .is_none()
    );
    let mut bindings = bindings();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let mut driver = Driver::new(read_program().unwrap());
    let (action, execution) =
        select(&mut run, &mut driver, &bindings, &equipment, "precombat").unwrap();
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    assert!(select(&mut run, &mut driver, &bindings, &equipment, "precombat").is_none());
    let (action, execution) =
        select(&mut run, &mut driver, &bindings, &equipment, "default").unwrap();
    assert_eq!(action.name, "auto_attack");
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    let (next, _) = select(&mut run, &mut driver, &bindings, &equipment, "default")
        .expect("ordinary native generator must remain selectable");
    assert_ne!(
        next.name, "auto_attack",
        "the real auto attack is already running"
    );
    assert_ne!(
        next.name, "rebuke",
        "literal action target_ready must reject an idle target"
    );
}
