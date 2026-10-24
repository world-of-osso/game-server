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
#[ignore = "literal Ret target_ready: idle real31146 must not select unconditional Rebuke"]
fn dps_parity_ret_literal_native_idle_target_does_not_select_rebuke() {
    let mut run = super::super::ret_tier_tests::native_ret(4);
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
