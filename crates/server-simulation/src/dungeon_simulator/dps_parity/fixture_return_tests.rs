//! Native displacement must not strand a literal melee actor outside combat.
use super::*;
use shared::components::Position;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn havoc_real_retreat_uses_max_melee_setup_over_300_seconds() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(577, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let mut previous_time = run.app.world().resource::<Time<Virtual>>().elapsed_secs_f64();
    let mut previous_out = false;
    let mut outside = 0.0;
    let mut seen_events = 0;
    let mut casts = 0;
    advance_combat_observed(&mut run, |run| {
        let world = run.app.world();
        let now = world.resource::<Time<Virtual>>().elapsed_secs_f64();
        if previous_out { outside += now - previous_time; }
        previous_time = now;
        let from = world.get::<Position>(actor).unwrap();
        let to = world.get::<Position>(target).unwrap();
        let distance = (from.x - to.x).hypot(from.z - to.z);
        let range = crate::scripted_ai::model_reach::melee_range(world, actor, target).unwrap();
        previous_out = distance > range;
        let events = &world.resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
        for event in &events[seen_events..] {
            if event.source == Some(actor.to_bits()) && event.spell_id == Some(198793)
                && event.kind == CombatLogKind::CastSuccess
            {
                casts += 1;
                assert!((distance - range).abs() < 0.05, "Retreat must launch at melee reach: {distance} vs {range}");
                let rotation = world.get::<shared::components::Rotation>(actor).unwrap();
                let facing = Vec2::new(rotation.y.sin(), rotation.y.cos());
                let away = Vec2::new(from.x - to.x, from.z - to.z).normalize();
                assert!(facing.dot(away) > 0.99, "Retreat must face away from the target");
            }
        }
        seen_events = events.len();
    }).unwrap();
    eprintln!("RETREAT_300S casts={casts} outside_melee={outside:.3}s simc=7.275s");
    assert!(casts >= 8, "real rotation must exercise repeated Retreat: {casts}");
    assert!(outside < 20.0, "max-reach setup must improve on the prior 29.0s: {outside}");
}

#[derive(Resource, Default)]
struct BeamStartDistance(Option<(f32, f32)>);

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
            let from = world
                .get::<Position>(actor)
                .ok_or("beam actor position absent")?;
            let to = world
                .get::<Position>(targets[0])
                .ok_or("beam target position absent")?;
            let distance = (from.x - to.x).hypot(from.z - to.z);
            let range = crate::scripted_ai::model_reach::melee_range(world, actor, targets[0])
                .ok_or("beam melee reach absent")?;
            world.resource_mut::<BeamStartDistance>().0 = Some((distance, range));
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
    run.app.world_mut().init_resource::<BeamStartDistance>();
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
    let (distance, range) = run
        .app
        .world()
        .resource::<BeamStartDistance>()
        .0
        .expect("real Beam cast");
    assert!(
        distance <= range + 0.001,
        "Havoc channel must wait for actual melee return: distance={distance}, range={range}"
    );
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
        launches, 13,
        "the real native channel must launch twelve ticks and selected Final Breath"
    );
    assert_eq!(
        hits, launches,
        "every actual beam tick must hit the stationary boss after physical return"
    );
}

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn shared_fixture_return_skips_out_of_range_priority_actions() {
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
    assert_eq!(
        selected.map(|(action, _)| action.name),
        Some("throw_glaive".into()),
        "out-of-range priority Beam must not block an in-range Throw Glaive"
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
fn havoc_retreat_allows_arcane_torrent_when_melee_is_out_of_range() {
    use literal_rotation::execution::{NativeAction, Prepared};
    use shared::components::PowerType;
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 1).unwrap();
    havoc_tests::start_havoc_swings(&mut run);
    havoc_tests::earn_havoc_fury(&mut run, 80.0);
    let actor = run.actor;
    let boss = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(198793, actor, Some(boss)),
    )
    .unwrap();
    for _ in 0..80 {
        run.app.update();
        let from = run.app.world().get::<Position>(actor).unwrap();
        let to = run.app.world().get::<Position>(boss).unwrap();
        if (from.x - to.x).hypot(from.z - to.z) > 10.0 {
            break;
        }
    }
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
                name: "arcane_torrent",
                spell: 202719,
                talent: None,
                self_target: true,
            },
        ],
        false,
    )
    .unwrap();
    let program = server::simc_apl::Program::parse(&[
        ("default", "chaos_strike"),
        ("default", "arcane_torrent"),
    ])
    .unwrap();
    let mut driver = literal_rotation::Driver::new(program);
    let (action, execution) = driver
        .select(
            "default",
            &BTreeMap::<String, f64>::new(),
            &prepared,
            &equipment,
        )
        .unwrap()
        .expect("racial remains available while returning");
    assert_eq!(action.name, "arcane_torrent");
    let before = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    driver
        .execute(run.app.world_mut(), actor, action, execution)
        .unwrap();
    let after = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    assert!(
        after > before,
        "native racial must grant its real Fury: {before} -> {after}"
    );
}

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn shared_fixture_native_blink_skips_out_of_range_melee() {
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
    assert_eq!(
        selected.map(|(action, _)| action.name),
        Some("throw_glaive".into()),
        "out-of-range melee must not block an in-range action after Blink"
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
