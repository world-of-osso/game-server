use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_howl_earns_real_wyvern_and_stampede_hits_the_pack() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 3).expect("native literal BM fixture");
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(19574, run.actor, Some(run.targets[0])),
    )
    .expect("ordinary Bestial Wrath");
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471878),
        "selected Stampede must earn the next real Howl readiness after Bestial Wrath"
    );
    preparation::finish_pending_setup(&mut run).unwrap();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        34026,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary Kill Command consumes earned Howl");
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471881),
        "Howl must create a real native Wyvern before it can earn Cry"
    );
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 471878));
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 471877));
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 1258338));
    let launched = crate::spell_cast::now(run.app.world());
    while crate::spell_cast::now(run.app.world()) < launched + 1.35 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for target in &run.targets {
        assert!(
            events.iter().any(|event| {
                event.spell_id == Some(201594)
                    && event.target == Some(target.to_bits())
                    && matches!(event.kind, shared::protocol::CombatLogKind::Damage)
                    && event.amount > 0
            }),
            "earned native Stampede must damage every pack recipient: {target:?}"
        );
    }
}
