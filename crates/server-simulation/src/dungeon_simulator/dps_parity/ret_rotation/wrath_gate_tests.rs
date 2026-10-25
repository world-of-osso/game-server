//! SimC Judgment action readiness while native Hammer mode is active.
use super::super::literal_rotation::{Driver, execution::Prepared};
use super::*;

#[test]
#[ignore = "literal Judgment is unavailable while native Hammer mode is active"]
fn dps_parity_ret_batch_hammer_mode_excludes_judgment() {
    let mut run = super::super::ret_tier_tests::native_ret(4);
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 1277026, Some(run.actor), 0);
    assert!(super::super::ret_tier_tests::has(&run, 1277026));
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
    let facts = with_equipment(super::tests::facts(&run, &bindings), &equipment);
    let program = server::simc_apl::Program::parse(&[("default", "judgment")]).unwrap();
    let mut driver = Driver::new(program);
    assert!(
        driver
            .select("default", &facts, &prepared, &equipment)
            .unwrap()
            .is_none(),
        "native mode must make the actual source Judgment row unavailable"
    );
}
