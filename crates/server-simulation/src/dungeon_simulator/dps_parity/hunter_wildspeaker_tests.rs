use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_wildspeaker_commands_existing_bear_before_consuming_next_howl() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1232739),
        "native profile selects Wildspeaker"
    );
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    run.app.update();
    for ready in [471878, 472324, 472325] {
        let deadline = crate::spell_cast::now(run.app.world()) + 31.0;
        while !crate::auras::has_aura(run.app.world(), run.actor, ready)
            && crate::spell_cast::now(run.app.world()) < deadline
        {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), run.actor, ready));
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            34026,
            Some(run.targets[0]),
            false,
        )
        .expect("ordinary KC consumes naturally earned beast");
        preparation::finish_pending_setup(&mut run).unwrap();
    }
    let bear = run
        .app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .find(|(_, summon)| summon.owner == run.actor && summon.spell_id == 471990)
        .map(|(bear, _)| bear)
        .expect("actual naturally earned Bear");
    let attempts = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(bear.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(1232922)
                    && matches!(
                        event.kind,
                        shared::protocol::CombatLogKind::Damage
                            | shared::protocol::CombatLogKind::Miss(_)
                    )
            })
            .count()
    };
    // SimC76f8118 sc_hunter.cpp6590–6604 commands existing DireCritters
    // before consuming Howl. A newly summoned Bear cannot join the earlier KC.
    assert_eq!(
        attempts(&run),
        0,
        "new Bear must not retroactively receive its summoning KC"
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        34026,
        Some(run.targets[0]),
        false,
    )
    .expect("next ordinary Kill Command");
    preparation::finish_pending_setup(&mut run).unwrap();
    assert_eq!(
        attempts(&run),
        1,
        "the existing Bear must execute one native Wildspeaker command at the actual primary"
    );
}
