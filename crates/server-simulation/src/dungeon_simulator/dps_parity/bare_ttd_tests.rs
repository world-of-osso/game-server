//! Actor bare TTD and explicit target TTD keep the pinned fixed-time distinction.
use super::*;
use server::simc_apl::{Action, ActionFacts, Facts};

#[test]
#[ignore = "requires private267 profile and native world DB"]
fn native_bare_time_to_die_keeps_actor_clock_when_action_scores_adds() {
    let mut run = prepare_literal(267, 3).unwrap();
    engage(&mut run);
    let now = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, now);
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: now.as_secs_f64(),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let bindings = crate::simc_apl_native::Bindings::default();
    let action = Action {
        source_row: None,
        name: "shadowburn".into(),
        target: Some(run.targets[1].to_bits()),
        options: Default::default(),
    };
    let read = |run: &Run| {
        let facts = crate::simc_apl_native::NativeFacts::new(
            run.app.world(),
            run.actor,
            &bindings,
            encounter,
            3,
        )
        .with_targets(Some(run.targets[0]), &run.targets);
        let actor = facts.value("time_to_die").unwrap();
        assert_eq!(facts.value_for(&action, "time_to_die").unwrap(), actor);
        (
            actor,
            facts.value_for(&action, "target.time_to_die").unwrap(),
        )
    };
    assert_eq!(read(&run), (300.0, 30.0));
    for _ in 0..100 {
        run.app.update();
    }
    let elapsed = (run.app.world().resource::<Time>().elapsed() - now).as_secs_f64();
    let (actor, add) = read(&run);
    assert!(elapsed > 0.0 && elapsed < 30.0);
    assert!((actor - (300.0 - elapsed)).abs() < 1e-6);
    assert!((add - (30.0 - elapsed)).abs() < 1e-6);
}
