//! Literal handoff and profile initialization must precede native warmup.
//! Native pet precasts must finish before engagement and the fight clock.
use super::*;
use crate::auras::ActiveAuras;
use shared::components::PowerType;

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
