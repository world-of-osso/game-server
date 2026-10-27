//! Native displacement must not strand a literal melee actor outside combat.
use super::*;
use shared::components::Position;
use shared::protocol::CombatLogKind;

struct RetreatThenBeam {
    driver: literal_rotation::Driver,
    equipment: literal_rotation::equipment::Equipment,
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
        let prepared = literal_rotation::execution::Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            [literal_rotation::execution::NativeAction {
                name: "eye_beam",
                spell: 198013,
                talent: None,
                self_target: true,
            }],
            false,
        )?;
        if let Some((action, execution)) = self.driver.select(
            "default",
            &BTreeMap::<String, f64>::new(),
            &prepared,
            &self.equipment,
        )? {
            self.driver.execute(world, actor, action, execution)?;
            self.cast = true;
        }
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
    run.app.world_mut().get_mut::<Position>(actor).unwrap().x = original.x - 3.0;
    run.app
        .world_mut()
        .init_resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(198793, actor, Some(boss)),
    )
    .unwrap();
    let equipment = literal_rotation::equipment::Equipment::load(
        run.app.world(),
        actor,
        &mut crate::simc_apl_native::Bindings::default(),
    )
    .unwrap();
    let program = server::simc_apl::Program::parse(&[("default", "eye_beam")]).unwrap();
    run.spec_driver = Some(Box::new(RetreatThenBeam {
        driver: literal_rotation::Driver::new(program),
        equipment,
        cast: false,
    }));
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
fn shared_fixture_return_keeps_eligible_actions_and_waits_for_priority_range() {
    use literal_rotation::execution::{NativeAction, Prepared};
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 1).unwrap();
    havoc_tests::start_havoc_swings(&mut run);
    havoc_tests::earn_havoc_fury(&mut run, 80.0);
    let actor = run.actor;
    let boss = run.targets[0];
    let original = *run.app.world().get::<Position>(boss).unwrap();
    run.app.world_mut().get_mut::<Position>(actor).unwrap().x = original.x - 3.0;
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(198793, actor, Some(boss)),
    )
    .unwrap();
    for _ in 0..80 {
        run.app.update();
        let from = run.app.world().get::<Position>(actor).unwrap();
        if (from.x - original.x).hypot(from.z - original.z) > 20.5 {
            break;
        }
    }
    let from = run.app.world().get::<Position>(actor).unwrap();
    assert!(
        (from.x - original.x).hypot(from.z - original.z) > 20.0,
        "real Retreat must put boss outside Beam range"
    );
    let equipment = literal_rotation::equipment::Equipment::load(
        run.app.world(),
        actor,
        &mut crate::simc_apl_native::Bindings::default(),
    )
    .unwrap();
    let prepared = Prepared::read(
        run.app.world_mut(),
        actor,
        &run.targets,
        &equipment,
        [
            NativeAction {
                name: "eye_beam",
                spell: 198013,
                talent: None,
                self_target: true,
            },
            NativeAction {
                name: "throw_glaive",
                spell: 185123,
                talent: None,
                self_target: false,
            },
            NativeAction {
                name: "immolation_aura",
                spell: 258920,
                talent: None,
                self_target: true,
            },
            NativeAction {
                name: "metamorphosis",
                spell: 191427,
                talent: None,
                self_target: true,
            },
        ],
        false,
    )
    .unwrap();
    let facts = BTreeMap::<String, f64>::new();
    for name in ["throw_glaive", "immolation_aura", "metamorphosis"] {
        let program = server::simc_apl::Program::parse(&[("default", name)]).unwrap();
        let selected = literal_rotation::Driver::new(program)
            .select("default", &facts, &prepared, &equipment)
            .unwrap();
        assert!(
            selected.is_some(),
            "in-range or self action {name} must remain available while returning"
        );
    }
    let program =
        server::simc_apl::Program::parse(&[("default", "eye_beam"), ("default", "throw_glaive")])
            .unwrap();
    let selected = literal_rotation::Driver::new(program)
        .select("default", &facts, &prepared, &equipment)
        .unwrap();
    assert!(
        selected.is_none(),
        "out-of-range priority Beam must wait, not fall through to Throw Glaive"
    );
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        actor,
        shared::components::PowerType::Fury,
        -1000.0,
    )
    .unwrap();
    let prepared = Prepared::read(
        run.app.world_mut(),
        actor,
        &run.targets,
        &equipment,
        [
            NativeAction {
                name: "chaos_strike",
                spell: 162794,
                talent: None,
                self_target: false,
            },
            NativeAction {
                name: "throw_glaive",
                spell: 185123,
                talent: None,
                self_target: false,
            },
        ],
        false,
    )
    .unwrap();
    let program = server::simc_apl::Program::parse(&[
        ("default", "chaos_strike"),
        ("default", "throw_glaive"),
    ])
    .unwrap();
    let mut driver = literal_rotation::Driver::new(program);
    let (action, execution) = driver
        .select("default", &facts, &prepared, &equipment)
        .unwrap()
        .expect("unaffordable melee action must not block an available ranged action");
    assert_eq!(action.name, "throw_glaive");
    driver
        .execute(run.app.world_mut(), actor, action, execution)
        .unwrap();
    run.app.update();
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(185123)
                && event.kind == CombatLogKind::CastSuccess),
        "the real in-range Throw Glaive must execute during return"
    );
}

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn shared_fixture_native_blink_waits_for_melee_return() {
    use literal_rotation::execution::{NativeAction, Prepared};
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 1).unwrap();
    havoc_tests::start_havoc_swings(&mut run);
    havoc_tests::earn_havoc_fury(&mut run, 80.0);
    let actor = run.actor;
    let boss = run.targets[0];
    // Native Blink exercises instant self-displacement; it is not a DH loadout claim.
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1953, Some(actor), 1);
    run.app.update();
    let from = run.app.world().get::<Position>(actor).unwrap();
    let to = run.app.world().get::<Position>(boss).unwrap();
    assert!(
        (from.x - to.x).hypot(from.z - to.z) > crate::combat::MELEE_RANGE,
        "real Blink must leave melee range"
    );
    let equipment = literal_rotation::equipment::Equipment::load(
        run.app.world(),
        actor,
        &mut crate::simc_apl_native::Bindings::default(),
    )
    .unwrap();
    let prepared = Prepared::read(
        run.app.world_mut(),
        actor,
        &run.targets,
        &equipment,
        [
            NativeAction {
                name: "chaos_strike",
                spell: 162794,
                talent: None,
                self_target: false,
            },
            NativeAction {
                name: "throw_glaive",
                spell: 185123,
                talent: None,
                self_target: false,
            },
        ],
        false,
    )
    .unwrap();
    let program = server::simc_apl::Program::parse(&[
        ("default", "chaos_strike"),
        ("default", "throw_glaive"),
    ])
    .unwrap();
    let selected = literal_rotation::Driver::new(program)
        .select(
            "default",
            &BTreeMap::<String, f64>::new(),
            &prepared,
            &equipment,
        )
        .unwrap();
    assert!(
        selected.is_none(),
        "the higher-priority melee action must wait for real return after instant displacement"
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
