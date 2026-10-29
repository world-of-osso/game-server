//! Native Devourer fragment-pickup admission while casting Consume.
use super::*;
use server::soul_fragments::{self, FragmentFilter, FragmentKind};

#[test]
#[ignore = "requires native spec1480 player profile and DB"]
fn spawned_fragment_counts_follow_native_activation() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(1480, 1).unwrap();
    let actor = run.actor;
    soul_fragments::spawn(run.app.world_mut(), actor, FragmentKind::Lesser, false).unwrap();
    let counts = |run: &Run| {
        let world = run.app.world();
        (
            soul_fragments::active_count(world, actor, FragmentFilter::Lesser),
            soul_fragments::inactive_count(world, actor, FragmentFilter::Lesser),
            soul_fragments::total_count(world, actor, FragmentFilter::Lesser),
        )
    };
    assert_eq!(counts(&run), (0, 1, 1));
    for _ in 0..10 {
        run.app.update();
    }
    assert_eq!(counts(&run), (1, 0, 1));
}

#[test]
#[ignore = "requires native spec581 player profile and DB"]
fn sigil_of_spite_detonates_native_damage_and_fragments_at_expiry() {
    let _seed = crate::random::seed_scope(70_448);
    let (profile, traits) = read_reference(581).unwrap();
    let identity = inputs::read_actor(&profile).unwrap();
    let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    let position = crate::spell_cast::targets::position(run.app.world(), target).unwrap();
    let health = |run: &Run| {
        run.app
            .world()
            .get::<shared::components::Health>(target)
            .unwrap()
            .current
    };
    let before = health(&run);
    crate::spell_cast::cast_for_player_with_targets(
        run.app.world_mut(),
        actor,
        390163,
        crate::spell_cast::CastTargets {
            unit: target,
            primary: Some(target),
            destination: Some(position.to_array()),
        },
    )
    .unwrap();
    run.app.update();
    assert_eq!(health(&run), before, "sigil cannot detonate on placement");
    assert_eq!(
        soul_fragments::total_count(run.app.world(), actor, FragmentFilter::Lesser),
        0
    );
    for _ in 0..50 {
        run.app.update();
    }
    let damage = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .any(|event| {
            event.spell_id == Some(389860)
                && event.target == Some(target.to_bits())
                && event.amount > 0
                && event.kind == shared::protocol::CombatLogKind::Damage
        });
    assert!(damage, "sigil must damage its native target at expiry");
    assert_eq!(
        soul_fragments::total_count(run.app.world(), actor, FragmentFilter::Lesser),
        4
    );
}

#[test]
#[ignore = "requires native spec581 player profile and DB"]
fn soul_sigils_grants_one_fragment_per_sigil_with_enemy_hits() {
    for (spell, count, empty, expected) in [
        (204596, 1, false, 1),
        (204596, 3, false, 1),
        (204596, 1, true, 0),
        (390163, 3, false, 4),
        (390163, 1, true, 3),
    ] {
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, count, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        let mut position = crate::spell_cast::targets::position(run.app.world(), target).unwrap();
        if empty {
            position.z += 20.0;
        }
        crate::spell_cast::cast_for_player_with_targets(
            run.app.world_mut(),
            actor,
            spell,
            crate::spell_cast::CastTargets {
                unit: target,
                primary: Some(target),
                destination: Some(position.to_array()),
            },
        )
        .unwrap();
        for _ in 0..70 {
            run.app.update();
        }
        assert_eq!(
            soul_fragments::observation_counts(run.app.world(), actor).0,
            expected,
            "spell {spell}, targets {count}, empty {empty}"
        );
    }
}

#[test]
#[ignore = "requires native spec581 player profile and DB"]
fn soul_splitter_rolls_once_for_each_three_fragment_batch() {
    let _seed = crate::random::seed_scope(70_448);
    let (profile, traits) = read_reference(581).unwrap();
    let identity = inputs::read_actor(&profile).unwrap();
    let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
    let actor = run.actor;
    assert!(crate::auras::has_aura(run.app.world(), actor, 1266330));
    for _ in 0..500 {
        soul_fragments::spawn_many(run.app.world_mut(), actor, FragmentKind::Lesser, 3, false).unwrap();
        for _ in 0..12 {
            run.app.update();
        }
    }
    let generated = soul_fragments::observation_counts(run.app.world(), actor).0;
    let extra = generated - 1500;
    assert!((8..=35).contains(&extra), "expected one 4% Soul Splitter roll per batch, got {extra} extra fragments");
}

#[test]
#[ignore = "requires native spec1480 player profile and DB"]
fn devourer_fragment_pickup_is_available_during_real_consume_cast() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(1480, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    for _ in 0..3 {
        soul_fragments::spawn(run.app.world_mut(), actor, FragmentKind::Lesser, false).unwrap();
    }
    for _ in 0..10 {
        run.app.update();
    }
    assert!(soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Lesser) > 0);
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        473662,
        Some(target),
    )
    .expect("real Devourer Consume cast");
    assert_eq!(
        run.app
            .world()
            .get::<shared::casting::CastState>(actor)
            .unwrap()
            .spell_id,
        473662
    );
    let mut bindings = crate::simc_apl_native::Bindings::default();
    let equipment =
        literal_rotation::equipment::Equipment::load(run.app.world(), actor, &mut bindings)
            .unwrap();
    let prepared = literal_rotation::execution::Prepared::read(
        run.app.world_mut(),
        actor,
        &run.targets,
        &equipment,
        std::iter::empty::<literal_rotation::execution::NativeAction<'_>>(),
        false,
    )
    .unwrap();
    let action = server::simc_apl::Action {
        source_row: None,
        name: "pick_up_fragment".into(),
        target: None,
        options: [("use_off_gcd".into(), "1".into())].into(),
    };
    assert!(
        prepared.permits_row(&action).unwrap(),
        "real Consume must allow pickup while casting"
    );
    assert!(
        prepared.execution(&action, &equipment).unwrap().is_some(),
        "active fragment remains selectable"
    );
}
