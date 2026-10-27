//! Native displacement must not strand a literal melee actor outside combat.
use super::*;
use shared::components::Position;
use shared::protocol::CombatLogKind;

struct RetreatThenBeam {
    cast: bool,
}

impl SpecDriver for RetreatThenBeam {
    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        if self.cast || world.get::<shared::casting::CastState>(actor).is_some() {
            return Ok(());
        }
        if world
            .get::<crate::spell_cast::SpellHistory>(actor)
            .is_some_and(|history| history.has_active_gcd(crate::spell_cast::history_now(world)))
        {
            return Ok(());
        }
        literal_rotation::execution::execute(
            world,
            actor,
            literal_rotation::execution::Execution::Spell(198013, actor, Some(targets[0])),
        )?;
        self.cast = true;
        Ok(())
    }

    fn label(&self) -> &'static str {
        "RETREAT_THEN_BEAM_TEST"
    }
}

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn shared_fixture_retreat_then_beam_returns_before_channel() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 1).unwrap();
    havoc_tests::start_havoc_swings(&mut run);
    havoc_tests::earn_havoc_fury(&mut run, 80.0);
    let actor = run.actor;
    let boss = run.targets[0];
    let original = *run.app.world().get::<Position>(boss).unwrap();
    run.app
        .world_mut()
        .init_resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(198793, actor, Some(boss)),
    )
    .unwrap();
    run.spec_driver = Some(Box::new(RetreatThenBeam { cast: false }));
    let mut max_distance = 0.0_f32;
    for _ in 0..240 {
        run.app.update();
        drive_player(&mut run).unwrap();
        let from = run.app.world().get::<Position>(actor).unwrap();
        let to = run.app.world().get::<Position>(boss).unwrap();
        max_distance = max_distance.max((from.x - to.x).hypot(from.z - to.z));
    }
    let launches = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>()
        .0
        .iter()
        .filter(|(caster, spell, _)| *caster == actor && matches!(*spell, 198030 | 1287949))
        .count();
    let hits = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(boss.to_bits())
                && event.kind == CombatLogKind::Damage
                && matches!(event.spell_id, Some(198030 | 1287949))
        })
        .count();
    let position = *run.app.world().get::<Position>(boss).unwrap();
    assert_eq!(
        (position.x, position.y, position.z),
        (original.x, original.y, original.z),
        "the boss must remain fixed"
    );
    assert!(
        max_distance > 20.0,
        "real Retreat must move the player outside beam range: {max_distance}"
    );
    assert_eq!(
        launches, 12,
        "the real native channel must launch all ticks"
    );
    assert_eq!(
        hits, launches,
        "every actual beam tick must hit the stationary boss after physical return"
    );
}

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn shared_fixture_displacement_preserves_unowned_movement_without_melee_intent() {
    let mut run = prepare_literal(577, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::combat::AutoAttack>(run.actor)
            .is_none()
    );
    run.app
        .world_mut()
        .get_mut::<crate::networking::Moving>(run.actor)
        .unwrap()
        .0 = true;
    run.app.update();
    assert!(
        run.app
            .world()
            .get::<crate::networking::Moving>(run.actor)
            .unwrap()
            .0,
        "the melee return controller must not rewrite unrelated native movement"
    );
}

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn shared_fixture_displacement_returns_to_melee_after_real_retreat() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 1).unwrap();
    havoc_tests::start_havoc_swings(&mut run);
    let start = *run.app.world().get::<Position>(run.actor).unwrap();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(198793, run.actor, Some(run.targets[0])),
    )
    .unwrap();
    let mut displaced = false;
    let mut walked_back = false;
    for frame in 0..240 {
        let before = *run.app.world().get::<Position>(run.actor).unwrap();
        let forced = run
            .app
            .world()
            .get::<shared::components::MovementControl>(run.actor)
            .is_some_and(|control| control.controlled);
        run.app.update();
        let position = run.app.world().get::<Position>(run.actor).unwrap();
        let step = (position.x - before.x).hypot(position.z - before.z);
        if !forced && step > 0.0 {
            let speed = super::super::movement::read_speed(run.app.world(), run.actor);
            let delta = run.app.world().resource::<Time>().delta_secs();
            assert!(
                step <= speed * delta + 0.001,
                "return movement must use native run speed, not snap: {step}"
            );
            walked_back = true;
        }
        displaced |= (position.x - start.x).hypot(position.z - start.z) > 5.0;
        if frame == 140 {
            run.app
                .world_mut()
                .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .clear();
        }
    }
    assert!(
        displaced,
        "the real native Retreat must still displace the actor"
    );
    assert!(
        walked_back,
        "the fixture must physically walk back after native control releases"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.source == Some(run.actor.to_bits())
                && event.spell_id.is_none()
                && event.amount > 0),
        "native melee swings must resume after the actual flight and finite return walk"
    );
    assert!(
        events
            .iter()
            .any(|event| event.kind == CombatLogKind::Energize
                && event.spell_id == Some(203796)
                && event.amount > 0),
        "real Demon Blades must again fund Fury after returning"
    );
}
