//! Exact-profile regression for Havoc's literal controller, not the dungeon kit.
use super::*;

#[test]
#[ignore = "requires native parity fixture"]
fn havoc_retreat_refuses_an_unpulled_hostile_at_the_landing() {
    let mut run = prepare_literal(577, 2).unwrap();
    let actor = run.actor;
    let primary = run.targets[0];
    let outsider = run.targets[1];
    let origin = *run
        .app
        .world()
        .get::<shared::components::Position>(actor)
        .unwrap();
    run.app.world_mut().entity_mut(outsider).insert((
        shared::components::Position {
            x: origin.x - 17.0,
            y: origin.y,
            z: origin.z,
        },
        crate::creature_aggro::AggroRange,
        crate::creature_ai::CreatureState::Idle,
        shared::components::UnitFactionTemplate(14),
    ));
    assert!(
        !crate::followers::retreat::prepare_havoc_retreat(run.app.world_mut(), actor, primary)
            .unwrap(),
        "an idle hostile at the real landing must prevent the leap"
    );
    let encounter =
        crate::simc_apl_native::Encounter::dungeon(crate::spell_cast::now(run.app.world()));
    let mut rotation =
        server_follower_apl::havoc_rotation::Rotation::load(run.app.world_mut(), actor, encounter)
            .unwrap();
    let program = server::simc_apl::Program::parse(&[
        ("default", "vengeful_retreat,use_off_gcd=1"),
        ("default", "felblade"),
    ])
    .unwrap();
    rotation.driver = server_follower_apl::literal_rotation::Driver::new(program);
    rotation
        .drive(run.app.world_mut(), actor, &[primary])
        .unwrap();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(232893)
                && event.kind == shared::protocol::CombatLogKind::CastSuccess),
        "the APL must execute the next available action after rejecting the leap"
    );
    run.app
        .world_mut()
        .get_mut::<shared::components::Position>(outsider)
        .unwrap()
        .x -= 100.0;
    assert!(
        crate::followers::retreat::prepare_havoc_retreat(run.app.world_mut(), actor, primary)
            .unwrap(),
        "the same leap must be admitted after the hostile moves away"
    );
    assert_eq!(
        *run.app
            .world()
            .get::<shared::components::Position>(actor)
            .unwrap(),
        origin,
        "admission must not walk before the cast"
    );
    run.app
        .world_mut()
        .remove_resource::<server::test_support::GroundMap>();
    assert!(
        !crate::followers::retreat::prepare_havoc_retreat(run.app.world_mut(), actor, primary)
            .unwrap(),
        "a landing without ground data must not be admitted"
    );
}

#[test]
#[ignore = "requires native parity fixture"]
fn havoc_retreat_setup_completes_during_real_follower_combat() {
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(577, 1).unwrap();
    engage(&mut run);
    let started = run.app.world().resource::<bevy::time::Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    drive_player(&mut run).unwrap();
    for _ in 0..400 {
        let end = run.app.world().resource::<bevy::time::Time>().elapsed()
            + std::time::Duration::from_millis(50);
        advance_fixture_step(&mut run, end).unwrap();
        drive_player(&mut run).unwrap();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        if events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(198793)
                && event.kind == CombatLogKind::CastSuccess
        }) {
            assert!(
                crate::auras::has_aura(run.app.world(), run.actor, 208628),
                "the actual Retreat cast must grant Exergy"
            );
            return;
        }
    }
    panic!("eligible real follower never completes Retreat and earns Exergy");
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec577 exact profile/traits"]
fn havoc_precombat_does_not_run_after_combat_starts() {
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 1).expect("native Havoc fixture");
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(shared::components::CombatStatus(true));
    let before = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    let driver = havoc_rotation::prepare_driver(&mut run).expect("production Havoc preparation");
    assert!(driver.report()["native_precombat_snapshot"].is_null());
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[before..];
    assert!(
        !events.iter().any(|event| {
            event.spell_id == Some(258920) && event.kind == CombatLogKind::CastSuccess
        }),
        "precombat Immolation must not cast after combat starts"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec577 exact profile/traits"]
fn havoc_non_meta_stat_buffs_follow_actual_metamorphosis() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(577, 1).expect("native Havoc profile");
    let actor = run.actor;
    for buff in [452550, 453314] {
        assert!(
            crate::auras::has_aura(run.app.world(), actor, buff),
            "selected buff {buff} must be active at pull"
        );
    }
    let stats = report::stats(run.app.world(), actor);
    let agility = stats["attribute"]["agility"].as_f64().unwrap();
    let swing_speed = stats["native"]["auto_attack_speed"].as_f64().unwrap();
    assert!((agility - 2904.0).abs() < 1.0, "pull agility: {agility}");
    assert!(
        (swing_speed - 0.850168).abs() < 0.001,
        "pull swing speed: {swing_speed}"
    );
    earn_eye_beam_fury(&mut run);
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        198013,
        Some(run.targets[0]),
    )
    .expect("ordinary native Eye Beam enters Meta");
    for buff in [452550, 453314] {
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, buff),
            "Meta must immediately remove {buff}"
        );
    }
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Expired,
        |aura| aura.spell_id == 162264 && aura.caster == actor,
    );
    for buff in [452550, 453314] {
        assert!(
            crate::auras::has_aura(run.app.world(), actor, buff),
            "Meta expiry must immediately restore {buff}"
        );
    }
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec577 exact profile/traits"]
fn havoc_native_metamorphosis_requires_ground_destination() {
    let mut run = prepare_literal(577, 1).expect("native Havoc profile");
    let actor = run.actor;
    let id = literal_rotation::execution::resolve_action(run.app.world(), actor, 191427)
        .expect("native Metamorphosis resolution")
        .expect("native Havoc knows Metamorphosis");
    let mut spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
        .unwrap()
        .clone();
    let result =
        crate::spell_cast::read_native_cast_start(run.app.world_mut(), actor, &spell, actor);
    assert!(matches!(
        result,
        Err(crate::spell_cast::CastFailure::NoTarget)
    ));
    let position = run
        .app
        .world()
        .get::<shared::components::Position>(actor)
        .unwrap();
    spell.cast_destination = Some([position.x, position.y, position.z]);
    let grounded =
        crate::spell_cast::read_native_cast_start(run.app.world_mut(), actor, &spell, actor);
    assert!(
        grounded.is_ok(),
        "native ground-request Metamorphosis{id}: {grounded:?}"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec577 exact profile/traits"]
fn havoc_demonic_enters_meta_before_native_eye_beam_channel() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 1).expect("native Havoc profile");
    let actor = run.actor;
    let target = run.targets[0];
    earn_eye_beam_fury(&mut run);
    assert!(!crate::auras::has_aura(run.app.world(), actor, 162264));
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        198013,
        Some(target),
    )
    .expect("ordinary native Eye Beam");
    let world = run.app.world();
    assert!(
        crate::auras::has_aura(world, actor, 162264),
        "selected native Demonic must transform before the Eye Beam channel"
    );
    let meta = world
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 162264 && aura.caster == actor)
        .unwrap();
    let remaining = meta.expires_at.unwrap() - crate::auras::aura_now(world);
    assert!(
        (remaining
            - 5.0
            // Native30 paid Fury also earns two100ms Shattered Destiny extensions.
            - 0.2
            - f64::from(
                world
                    .get::<shared::casting::CastState>(actor)
                    .unwrap()
                    .duration
            ))
        .abs()
            < 0.001,
        "Demonic must retain native5s plus actual channel and earned Shattered Destiny, got {remaining}"
    );
    assert_eq!(
        world
            .get::<shared::casting::CastState>(actor)
            .unwrap()
            .spell_id,
        198013
    );
}

fn earn_eye_beam_fury(run: &mut Run) {
    earn_havoc_fury(run, 30.0);
}

pub(super) fn start_havoc_swings(run: &mut Run) {
    let actor = run.actor;
    let target = run.targets[0];
    engage(run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::AutoAttack(target),
    )
    .unwrap();
}

pub(super) fn earn_havoc_fury(run: &mut Run, minimum: f32) {
    let actor = run.actor;
    start_havoc_swings(run);
    // Earn the requested native cast budget from real swings; no pool edits.
    for _ in 0..800 {
        let (fury, _) = crate::spell_cast::powers::power_value(
            run.app.world(),
            actor,
            shared::components::PowerType::Fury,
        )
        .unwrap();
        if fury >= minimum {
            return;
        }
        run.app.update();
    }
    panic!("native swings did not fund {minimum} Fury");
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec577 exact profile/traits"]
fn havoc_demonic_extends_existing_native_meta_without_refreshing_it() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 1).expect("native Havoc profile");
    earn_eye_beam_fury(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(191427, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let deadline = |world: &World| {
        world
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 162264 && aura.caster == actor)
            .unwrap()
            .expires_at
            .unwrap()
    };
    let before = deadline(run.app.world());
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        198013,
        Some(target),
    )
    .expect("ordinary native Eye Beam while Meta is active");
    let channel = f64::from(
        run.app
            .world()
            .get::<shared::casting::CastState>(actor)
            .unwrap()
            .duration,
    );
    // Native Eye Beam30 Fury crosses two12-Fury Shattered Destiny thresholds.
    assert!((deadline(run.app.world()) - before - 5.0 - channel - 0.2).abs() < 0.001);
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec577 exact profile/traits"]
fn havoc_literal_fixture_collects_real_demonic_appetite_fragments() {
    let result = measure(577, 1).expect("native Havoc measurement");
    let gains = result["combat"]["energize"].as_array().unwrap();
    assert!(
        gains.iter().any(|gain| gain["spell_id"] == 210041
            && gain["effective_gain"]
                .as_f64()
                .is_some_and(|amount| amount > 0.0)),
        "real Demonic Appetite fragments must reach the native Consume Soul Fury payload: {gains:?}"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec577 exact profile/traits"]
fn havoc_literal_fixture_executes_source_actions() {
    let result = measure(577, 1).expect("native Havoc measurement");
    assert_eq!(result["driver"], "LITERAL_SIMC_HAVOC_76f8118");
    let actions = result["combat"]["actions"].as_array().unwrap();
    let essence = actions
        .iter()
        .find(|row| row["spell_id"] == 258860)
        .expect("native Essence Break cast");
    assert_eq!(
        essence["count"].as_f64(),
        essence["hits"].as_f64(),
        "literal native Essence Break must not spend its cooldown on an empty cone in the one-target fixture: {essence}"
    );
    // Native overrides can replace Eye Beam with Abyssal Gaze; neither is
    // a different priority (SimC76f8118 sc_demon_hunter.cpp4078–4106).
    for ids in [&[191427][..], &[198013, 452497], &[258860]] {
        assert!(
            actions.iter().any(|row| {
                ids.iter().any(|id| row["spell_id"] == *id)
                    && row["count"].as_f64().expect("numeric mean cast count") > 0.0
            }),
            "literal source ability {ids:?} must execute: {actions:?}"
        );
    }
}
