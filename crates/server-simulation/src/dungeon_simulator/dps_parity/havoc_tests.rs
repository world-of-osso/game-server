//! Exact-profile regression for Havoc's literal controller, not the dungeon kit.
use super::*;

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
            - f64::from(
                world
                    .get::<shared::casting::CastState>(actor)
                    .unwrap()
                    .duration
            ))
        .abs()
            < 0.001,
        "Demonic must retain native5s plus actual channel duration, got {remaining}"
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
    assert!((deadline(run.app.world()) - before - 5.0 - channel).abs() < 0.001);
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec577 exact profile/traits"]
fn havoc_literal_fixture_executes_source_actions() {
    let result = measure(577, 1).expect("native Havoc measurement");
    assert_eq!(result["driver"], "LITERAL_SIMC_HAVOC_76f8118");
    let actions = result["combat"]["actions"].as_array().unwrap();
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
