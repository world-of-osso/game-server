use super::super::literal_rotation::{Driver, execution::Prepared};
use super::*;
use server::simc_apl::Facts;

#[test]
#[ignore = "native Ret source variables: real equipment facts without invented inventory"]
fn dps_parity_ret_literal_native_source_variables_read_real_equipment() {
    let mut run = super::super::ret_tier_tests::native_ret(4);
    let mut bindings = bindings();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let mut driver = Driver::new(read_program().unwrap());
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        execution_actions(),
        driver.snapshot.is_none(),
    )
    .unwrap();
    let initial = {
        let facts = with_equipment(super::tests::facts(&run, &bindings), &equipment);
        driver
            .select("precombat", &facts, &prepared, &equipment)
            .unwrap()
            .expect("actual source snapshot must precede its variables")
    };
    driver
        .execute(run.app.world_mut(), run.actor, initial.0, initial.1)
        .unwrap();
    let snapshot = driver.snapshot.clone();
    assert!(snapshot.is_some());
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        execution_actions(),
        driver.snapshot.is_none(),
    )
    .unwrap();
    let facts = with_equipment(super::tests::facts(&run, &bindings), &equipment);
    let selected = driver
        .select("precombat", &facts, &prepared, &equipment)
        .expect("pinned source variables must read actual native trinket facts");
    assert_eq!(facts.value("equipped.algethar_puzzle_box").unwrap(), 0.0);
    assert!(
        selected.is_none(),
        "unowned Puzzle Box must not create an item execution"
    );
    // SimC76f8118 player.cpp11974–11993: an absent equipped name
    // is false, while an unknown stat operand remains invalid.
    assert_eq!(facts.value("equipped.unknown_native_item").unwrap(), 0.0);
    assert!(facts.value("trinket.1.has_buff.unknown_stat").is_err());
    assert_eq!(
        driver.snapshot, snapshot,
        "equipment facts cannot replace the snapshot"
    );
}
