//! Literal handoff and profile initialization must precede native warmup.
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
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor)
        .expect("native default pet at pull");
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
