use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared native 261 profile and player actor"]
fn subtlety_vanish_admits_shadowstrike_and_resumes_weapons() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(261, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let cast = |run: &mut Run, spell, unit| {
        literal_rotation::execution::execute(
            run.app.world_mut(),
            actor,
            literal_rotation::execution::Execution::Spell(spell, unit, Some(target)),
        )
        .unwrap();
    };
    cast(&mut run, 185438, target);
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    cast(&mut run, 1856, actor);
    assert!(crate::auras::has_aura(run.app.world(), actor, 11327));
    assert!(crate::auras::has_aura(run.app.world(), actor, 158188));
    let world = run.app.world_mut();
    let shadowstrike = world
        .resource::<crate::spell_info::SpellStore>()
        .get(185438)
        .unwrap()
        .clone();
    assert!(crate::spell_cast::read_native_cast_start(world, actor, &shadowstrike, target).is_ok());
    cast(&mut run, 185438, target);
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(!crate::auras::has_aura(run.app.world(), actor, 11327));
    assert!(!crate::auras::has_aura(run.app.world(), actor, 158188));
    assert!(
        run.app
            .world()
            .get::<crate::combat::AutoAttack>(actor)
            .is_some()
    );
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(185438)
                && event.kind == CombatLogKind::CastSuccess)
            .count()
            >= 2
    );
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_literal_driver_coats_and_executes_profile_opener() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(261, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<literal_rotation::LiteralParityFixture>(run.actor)
            .is_some(),
        "Subtlety must execute literal SimC rows, not AssistedCombat"
    );
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 315584));
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
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && matches!(event.spell_id, Some(185438 | 53 | 200758 | 197835 | 196819))),
        "literal Subtlety must execute its native offensive action"
    );
}
