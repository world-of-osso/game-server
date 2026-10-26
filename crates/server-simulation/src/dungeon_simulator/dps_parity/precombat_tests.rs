//! Literal handoff and profile initialization must precede native warmup.
//! Native pet precasts must finish before engagement and the fight clock.
use super::*;
use crate::auras::ActiveAuras;
use shared::components::PowerType;

#[test]
#[ignore = "requires prepared266 profile/current native source view; configured native summon and matching-pet no-op"]
fn literal_precombat_configured_demo_pet_uses_real_cast_and_does_not_resummon() {
    use literal_rotation::default_pet::ConfiguredPet;
    use literal_rotation::execution::Prepared;
    use server::simc_apl::Program;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(266, 1).unwrap();
    let configured = ConfiguredPet::read(run.app.world(), run.actor, &run.profile)
        .unwrap()
        .expect("pinned profile declares its default pet");
    let mut bindings = crate::simc_apl_native::Bindings::default();
    configured.bind(&mut bindings).unwrap();
    let equipment =
        literal_rotation::equipment::Equipment::load(run.app.world_mut(), run.actor, &mut bindings)
            .unwrap();
    let started_at = crate::spell_cast::now(run.app.world());
    let mut native_cast_duration = None;
    let mut original_pet = None;
    for pass in 0..2 {
        // Bounded shared-mechanism control: actual first two266 precombat actions,
        // not a claim that this fixture executes the later complete Demo APL.
        let mut driver = literal_rotation::Driver::new(
            Program::parse(&[("precombat", "summon_pet"), ("precombat", "snapshot_stats")])
                .unwrap(),
        );
        loop {
            let action =
                preparation::wait_for_configured_pet_action(&mut run, &configured).unwrap();
            let prepared = Prepared::read(
                run.app.world_mut(),
                run.actor,
                &[],
                &equipment,
                action,
                driver.snapshot.is_none(),
            )
            .unwrap();
            let facts = BTreeMap::<String, f64>::new();
            let Some((action, execution)) = driver
                .select("precombat", &facts, &prepared, &equipment)
                .unwrap()
            else {
                break;
            };
            driver
                .execute(run.app.world_mut(), run.actor, action, execution)
                .unwrap();
            if let Some(cast) = run.app.world().get::<shared::casting::CastState>(run.actor) {
                native_cast_duration = Some(cast.duration);
            }
            preparation::finish_pending_setup(&mut run).unwrap();
        }
        assert!(driver.snapshot.is_some());
        let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
        assert_eq!(
            run.app
                .world()
                .get::<shared::components::Npc>(pet)
                .unwrap()
                .template_id,
            17252
        );
        if pass == 0 {
            original_pet = Some(pet);
        } else {
            assert_eq!(Some(pet), original_pet, "matching pet must not be replaced");
        }
    }
    let duration = native_cast_duration.expect("native summon must start an actual cast bar");
    assert!(duration > 0.0);
    let elapsed = crate::spell_cast::now(run.app.world()) - started_at;
    assert!(
        elapsed + 0.001 >= f64::from(duration),
        "actual native cast time must elapse"
    );
    let casts = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(30146)
                && event.kind == shared::protocol::CombatLogKind::CastSuccess
        })
        .count();
    assert_eq!(
        casts, 1,
        "matching configured pet is a once-only no-op, not another summon cast"
    );
}

#[test]
#[ignore = "requires prepared269 profile/current native source view; snapshot before fixture engagement"]
fn literal_precombat_ww_snapshot_exists_before_engagement_and_fight_time() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(269, 1).unwrap();
    let owner = fixture_player(&run);
    for &target in &run.targets {
        assert!(!crate::combat_refs::has_active_reference_to(
            run.app.world(),
            owner,
            target
        ));
        assert!(!crate::combat_refs::has_active_reference_to(
            run.app.world(),
            run.actor,
            target
        ));
    }
    let rotation = (run.spec_driver.as_ref().unwrap().as_ref() as &dyn std::any::Any)
        .downcast_ref::<windwalker_rotation::Rotation>()
        .unwrap();
    assert!(rotation.report()["native_precombat_snapshot"].is_object());
    let action = server::simc_apl::Action {
        source_row: None,
        name: "tiger_palm".into(),
        target: None,
        options: Default::default(),
    };
    assert_eq!(
        rotation
            .value_for(
                run.app.world_mut(),
                run.actor,
                &run.targets,
                &action,
                "time"
            )
            .unwrap(),
        0.0,
        "the fight clock begins after native precombat, not at actor spawn"
    );
}

#[test]
#[ignore = "requires prepared269 profile/current native source view; first harmful precombat cap"]
fn literal_precombat_stops_after_the_first_ready_harmful_native_action() {
    use literal_rotation::execution::{NativeAction, Prepared};
    use server::simc_apl::{Action, Program};
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(269, 1).unwrap();
    let mut bindings = crate::simc_apl_native::Bindings::default();
    let equipment =
        literal_rotation::equipment::Equipment::load(run.app.world_mut(), run.actor, &mut bindings)
            .unwrap();
    let actions = || {
        [
            NativeAction {
                name: "tiger_palm",
                spell: 100780,
                talent: None,
                self_target: false,
            },
            NativeAction {
                name: "blackout_kick",
                spell: 100784,
                talent: None,
                self_target: false,
            },
        ]
    };
    let mut driver = literal_rotation::Driver::new(
        Program::parse(&[("precombat", "tiger_palm"), ("precombat", "blackout_kick")]).unwrap(),
    );
    let facts = BTreeMap::<String, f64>::new();
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        actions(),
        false,
    )
    .unwrap();
    let (action, execution) = driver
        .select("precombat", &facts, &prepared, &equipment)
        .unwrap()
        .expect("the actual TP precast must be ready");
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        actions(),
        false,
    )
    .unwrap();
    let blackout = Action {
        source_row: None,
        name: "blackout_kick".into(),
        target: None,
        options: Default::default(),
    };
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(100784)
        .unwrap()
        .clone();
    let native = crate::spell_cast::read_native_cast_start(
        run.app.world_mut(),
        run.actor,
        &spell,
        run.targets[0],
    )
    .map(|_| ());
    let chi = crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::Chi);
    let resolved = crate::auras::overrides::resolve_spell(run.app.world(), run.actor, 100784);
    let actor_position = run
        .app
        .world()
        .get::<shared::components::Position>(run.actor);
    let target_position = run
        .app
        .world()
        .get::<shared::components::Position>(run.targets[0]);
    assert!(
        prepared.execution(&blackout, &equipment).unwrap().is_some(),
        "later BOK must be native-ready after real frame/GCD: readiness={native:?}, chi={chi:?}, resolved={resolved}, actor={actor_position:?}, target={target_position:?}"
    );
    assert!(
        driver
            .select("precombat", &facts, &prepared, &equipment)
            .unwrap()
            .is_none()
    );
}

#[test]
#[ignore = "requires prepared269/current native view; nonliteral owner-follow control"]
fn literal_precombat_uncaptured_follower_still_follows_its_native_owner() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(269, 1).unwrap();
    let encounter = crate::simc_apl_native::Encounter::Dungeon {
        started_at: crate::spell_cast::now(run.app.world()),
    };
    let error =
        literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter).unwrap_err();
    assert!(error.contains("DUNGEON_TTD_UNAVAILABLE"));
    assert!(
        run.app
            .world()
            .get::<literal_rotation::LiteralParityFixture>(run.actor)
            .is_none()
    );
    let owner = run
        .app
        .world()
        .get::<crate::followers::Follower>(run.actor)
        .unwrap()
        .owner;
    assert_eq!(
        run.app
            .world()
            .get::<crate::creature_ai::FollowOwner>(run.actor)
            .unwrap()
            .owner,
        owner
    );
    let owner_position = *run
        .app
        .world()
        .get::<shared::components::Position>(owner)
        .unwrap();
    let distance = |position: &shared::components::Position| {
        Vec3::new(position.x, position.y, position.z).distance(Vec3::new(
            owner_position.x,
            owner_position.y,
            owner_position.z,
        ))
    };
    let before = distance(
        run.app
            .world()
            .get::<shared::components::Position>(run.actor)
            .unwrap(),
    );
    for _ in 0..60 {
        run.app.update();
    }
    let after = distance(
        run.app
            .world()
            .get::<shared::components::Position>(run.actor)
            .unwrap(),
    );
    assert!(
        after + 1.0 < before,
        "nonliteral follower must really move toward its owner: {before}->{after}"
    );
    assert!(
        run.app
            .world()
            .get::<crate::creature_ai::FollowOwner>(run.actor)
            .is_some()
    );
    assert!(
        run.app
            .world()
            .get::<shared::components::Npc>(run.actor)
            .is_some()
    );
}

#[test]
#[ignore = "requires prepared103 profile/current native source view; literal precombat handoff RED"]
fn literal_precombat_captures_feral_before_legacy_kit_warmup() {
    let _seed = crate::random::seed_scope(70448);
    let run = prepare_literal(103, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<literal_rotation::LiteralParityFixture>(run.actor)
            .is_some()
    );
    let auras = run.app.world().get::<ActiveAuras>(run.actor).unwrap();
    assert!(
        !auras.has_spell(1126),
        "literal handoff must precede native kit Mark of the Wild, not delete it after warmup"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        !events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1126)
                && event.kind == shared::protocol::CombatLogKind::CastSuccess),
        "an absent final aura cannot conceal an earlier legacy kit cast"
    );
}

#[test]
#[ignore = "requires prepared103 profile/current native source view; native controller control"]
fn literal_precombat_leaves_nonliteral_native_feral_kit_unchanged() {
    let _seed = crate::random::seed_scope(70448);
    let run = prepare(103, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<literal_rotation::LiteralParityFixture>(run.actor)
            .is_none()
    );
    assert!(
        run.app
            .world()
            .get::<ActiveAuras>(run.actor)
            .unwrap()
            .has_spell(1126),
        "ordinary native fixture kit still applies its actual class buff"
    );
}

#[test]
#[ignore = "requires prepared266 profile/current native source view; native Demo ST pull regression"]
fn literal_precombat_demo_st_deals_damage_within_ten_seconds_after_pull() {
    const POST_PULL_TICKS: u32 = 200;
    const FIRST_DAMAGE_DEADLINE_SECONDS: f64 = 10.0;
    let _seed = crate::random::seed_scope(70448);
    let (profile, traits) = read_reference(266).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    engage(&mut run);
    let pulled_at = crate::spell_cast::now(run.app.world());
    for _ in 0..POST_PULL_TICKS {
        run.app.update();
        let elapsed = crate::spell_cast::now(run.app.world()) - pulled_at;
        let combat = report::combat(run.app.world(), &run.damage_targets, elapsed);
        if combat["total_damage"].as_u64().unwrap() > 0 {
            assert!(elapsed <= FIRST_DAMAGE_DEADLINE_SECONDS + 0.001);
            println!("Demo ST first native damage at pull+{elapsed:.3}s: {combat}");
            return;
        }
    }
    panic!(
        "Demo ST must deal native damage within {FIRST_DAMAGE_DEADLINE_SECONDS}s after pull, \
         without add-wave re-engagement; final state {}",
        report::engagement(run.app.world(), run.actor, &run.targets)
    );
}

#[test]
#[ignore = "requires prepared266 profile/current native source view; declared initial shards RED"]
fn literal_precombat_initializes_declared_demo_shards_before_warmup() {
    let _seed = crate::random::seed_scope(70448);
    let profile: serde_json::Value =
        inputs::read_json(&inputs::directory().unwrap().join("266.profile.json")).unwrap();
    assert_eq!(
        profile["warlock.soul_shards"], "0",
        "actual pinned start option"
    );
    assert_eq!(
        profile["warlock.default_pet"], "felguard",
        "actual pinned default pet"
    );
    let run = prepare_literal(266, 1).unwrap();
    let (current, maximum) =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::SoulShards)
            .expect("native warlock secondary pool");
    assert!(maximum > 0.0, "native soul-shard maximum must exist");
    assert_eq!(
        current, 0.0,
        "profile-declared starting state precedes precombat; do not reset/grant after casts"
    );
}

#[test]
#[ignore = "requires prepared267 profile/current native source view; player initial pool ordering"]
fn literal_precombat_player_declared_shards_exist_before_first_frame() {
    let (profile, traits) = read_reference(267).unwrap();
    let identity = inputs::read_actor(&profile).unwrap();
    let mut run = spawn_reference(profile, &traits, 1, identity).unwrap();
    assert!(run.app.world().get::<shared::components::Player>(run.actor).is_some());
    let read_power = |run: &Run| {
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::SoulShards)
            .expect("native player class pool must precede declared-power validation")
    };
    let initial = read_power(&run);
    assert_eq!(initial.0, 0.0, "honor the pinned warlock.soul_shards=0 before warmup");
    assert!(initial.1 > 0.0, "native class metadata supplies the maximum");
    run.app.update();
    assert_eq!(read_power(&run), initial, "the first ordinary native frame cannot replace the configured pool");
}

#[test]
#[ignore = "requires prepared266 profile/current native source view; native default pet pull boundary"]
fn literal_precombat_native_demo_default_pet_is_complete_at_pull() {
    let _seed = crate::random::seed_scope(70448);
    // Native-controller control, independent of the literal adapter's later precasts.
    let run = prepare(266, 1).unwrap();
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor)
        .expect("native default pet at pull");
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none(),
        "the real summon cast must finish before the pull"
    );
    let world = run.app.world();
    let summon = world
        .resource::<crate::spell_info::SpellStore>()
        .get(30146)
        .unwrap();
    assert!(
        world
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .is_some_and(|history| !history.on_gcd(summon, crate::spell_cast::history_now(world))),
        "native summon GCD must also finish before the pull"
    );
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::Npc>(pet)
            .unwrap()
            .template_id,
        17252,
        "native selected Felguard, not a fabricated pet"
    );
}
