use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn unholy_literal_driver_executes_native_profile_actions() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 1).expect("literal Unholy preparation");
    assert_eq!(driver_report(&run)["driver"], "LITERAL_SIMC_UNHOLY_76f8118");
    assert!(
        run.app
            .world()
            .get::<shared::components::Player>(run.actor)
            .is_some()
    );
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    for _ in 0..200 {
        drive_player(&mut run).expect("literal Unholy decision");
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && matches!(event.spell_id, Some(85948 | 55090 | 77575))),
        "the literal APL must launch an actual native generator or disease"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn unholy_named_masterful_cunning_installs_native_rune() {
    let (mut profile, traits) = read_reference(252).unwrap();
    profile.omnium_talents = Some("rune_of_masterful_cunning".into());
    let run = prepare_literal_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 8 },
    )
    .unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1279612),
        "named native TraitNodeEntry136818 must apply spell1279612"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn unholy_bare_health_score_keeps_actor_context_across_recipients() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 3).unwrap();
    for (target, fraction) in [
        (run.actor, 0.42),
        (run.targets[0], 0.9),
        (run.targets[1], 0.1),
    ] {
        let mut health = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current = health.max * fraction;
    }
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    for _ in 0..200 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let first = events
        .iter()
        .find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(85948)
                && event.kind == CombatLogKind::CastSuccess
        })
        .expect("the literal 3T APL must execute Festering Strike");
    assert_eq!(
        first.target,
        Some(run.targets[0].to_bits()),
        "bare health.pct scores the same actor for every candidate; do not silently rewrite it as target.health.pct"
    );
}
