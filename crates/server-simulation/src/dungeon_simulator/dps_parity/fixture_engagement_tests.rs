use super::*;

#[test]
#[ignore = "requires private Marksmanship profile and native world data"]
fn active_adds_remain_engaged_after_training_dummy_timeout() {
    let mut run = prepare_measurement(254, 3).unwrap();
    run.spec_driver = None;
    engage(&mut run);
    let owner = fixture_player(&run);
    let start = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, start);
    let mut retired = Vec::new();
    for tick in 1..=160 {
        advance_fixture_frame(
            &mut run,
            start + Duration::from_millis(50 * tick),
            start,
            &mut retired,
            &mut |_| {},
        )
        .unwrap();
    }
    let world = run.app.world();
    for &target in &run.targets {
        assert!(
            crate::combat_refs::has_active_reference_to(world, owner, target),
            "active target {target:?} must remain engaged after the dummy timeout"
        );
    }
    assert!(
        crate::followers::action_safe(world, run.actor, run.targets[0], Some(19434)),
        "Aimed Shot's real splash reach must not treat active adds as outsiders"
    );
}
