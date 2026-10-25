use super::*;

fn slayer_hits(run: &Run) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.kind == shared::protocol::CombatLogKind::Damage
                && event.spell_id == Some(445579)
                && event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
        })
        .count()
}

fn assert_native_slayers_strike(spec: u32, action: u32) {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = super::prepare_literal(spec, 1).expect("actual selected Slayer player profile");
    super::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    super::literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        super::literal_rotation::execution::Execution::AutoAttack(target),
    )
    .unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(
            |event| event.kind == shared::protocol::CombatLogKind::Damage
                && event.source == Some(run.actor.to_bits())
                && event.spell_id.is_none()
        ),
        "actual white swings must land"
    );
    assert_eq!(
        slayer_hits(&run),
        0,
        "ordinary white swings are not Slayer triggers"
    );
    let actor = run.actor;
    let target = run.targets[0];
    for _ in 0..40 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, action, Some(target), 1);
        for _ in 0..12 {
            run.app.update();
        }
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(
            |event| event.kind == shared::protocol::CombatLogKind::Damage
                && event.source == Some(actor.to_bits())
                && event.spell_id == Some(action)
        ),
        "eligible native action must actually hit"
    );
    assert!(
        slayer_hits(&run) > 0,
        "actual eligible native impacts must execute Slayer's Strike"
    );
}

#[test]
#[ignore = "requires exact Fury72 player profile and selected Slayer's Dominance"]
fn fury_slayers_strike_procs_from_native_bloodthirst_impacts() {
    assert_native_slayers_strike(72, 23881);
}

#[test]
#[ignore = "requires exact Arms71 player profile and selected Slayer's Dominance"]
fn arms_slayers_strike_procs_from_native_mortal_strike_impacts() {
    assert_native_slayers_strike(71, 12294);
}

#[test]
#[ignore = "requires exact Fury72 native Slayer player inputs"]
fn fury_imminent_demise_grants_sudden_death_on_third_slayers_strike() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = super::prepare_literal(72, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 444769));
    assert!(crate::auras::has_aura(run.app.world(), actor, 29725));
    for count in 1..=3 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 445579, Some(target), 1);
        assert_eq!(
            crate::auras::has_aura(run.app.world(), actor, 52437),
            count == 3,
            "Imminent Demise counts Slayer executions, including background executions"
        );
    }
}

#[test]
#[ignore = "requires exact Arms71 native Unhinged player inputs"]
fn arms_unhinged_bladestorm_ticks_execute_native_mortal_strike() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = super::prepare_literal(71, 1).unwrap();
    super::engage(&mut run);
    let actor = run.actor;
    super::literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        super::literal_rotation::execution::Execution::Spell(227847, actor, Some(run.targets[0])),
    )
    .expect("ordinary Arms Bladestorm");
    for _ in 0..100 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(
            |event| event.kind == shared::protocol::CombatLogKind::Damage
                && event.source == Some(actor.to_bits())
                && event.spell_id == Some(12294)
        ),
        "Arms Unhinged must execute real native Mortal Strike during Bladestorm"
    );
}
