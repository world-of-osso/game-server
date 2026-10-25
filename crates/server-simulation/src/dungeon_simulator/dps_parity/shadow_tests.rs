//! Spec258 uses the literal pinned Shadow APL, not the native follower kit.
use super::*;

#[test]
fn parity_combat_reports_observed_hit_and_crit_amounts() {
    use shared::protocol::{CombatLogEvent, CombatLogKind};
    let mut world = World::new();
    let actor = world.spawn_empty().id();
    let target = world.spawn_empty().id();
    let event = |amount, crit| CombatLogEvent {
        source: Some(actor.to_bits()),
        target: Some(target.to_bits()),
        spell_id: Some(8092),
        school_mask: 32,
        amount,
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit,
        glancing: false,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: 0,
        kind: CombatLogKind::Damage,
    };
    world.insert_resource(crate::spell_cast::test_harness::ObservedCombatEvents(vec![
        event(20, false),
        event(40, true),
    ]));
    let combat = report::combat(&world, &[target], 10.0);
    let action = &combat["actions"][0];
    assert_eq!(action["crit_hits"], 1);
    assert_eq!(action["crit_rate"], 0.5);
    assert_eq!(action["average_noncrit"], 20.0);
    assert_eq!(action["average_crit"], 40.0);
    assert_eq!(combat["dps"], 6.0);
}

#[test]
fn native_apl_unshielded_target_has_no_absorb() {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    use server::simc_apl::Facts;
    let mut world = World::new();
    let actor = world.spawn_empty().id();
    let target = world.spawn_empty().id();
    let targets = [target];
    let bindings = Bindings::default();
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let facts = NativeFacts::new(&world, actor, &bindings, encounter, 1)
        .with_targets(Some(target), &targets);
    assert_eq!(facts.value("target.has_absorb").unwrap(), 0.0);
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and spec258 profile/traits; actor=player"]
fn native_apl_target_absorb_tracks_real_shield_and_depletion() {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    use server::simc_apl::Facts;
    let mut run = prepare(258, 1).unwrap();
    let actor = run.actor;
    let application = crate::auras::AuraApplication {
        caster: actor,
        target: actor,
        spell_id: 17,
        effect_mask: 1,
        source: crate::auras::AuraSource::Cast,
    };
    crate::auras::apply_aura(run.app.world_mut(), &application);
    let bindings = Bindings::default();
    let targets = [actor];
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let has_absorb = |world: &World| {
        NativeFacts::new(world, actor, &bindings, encounter, 1)
            .with_targets(Some(actor), &targets)
            .value("target.has_absorb")
            .unwrap()
    };
    assert_eq!(has_absorb(run.app.world()), 1.0);
    let capacity = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 17)
        .unwrap()
        .effects[0]
        .amount;
    // Keep the damage near real shield capacity: f32::MAX loses this subtraction.
    let (_, absorbed) =
        crate::auras::absorb_damage(run.app.world_mut(), actor, 0x7f, capacity + 1.0);
    assert_eq!(absorbed, capacity);
    assert_eq!(has_absorb(run.app.world()), 0.0);
    crate::auras::apply_aura(run.app.world_mut(), &application);
    assert_eq!(has_absorb(run.app.world()), 1.0);
    // A stale, not-yet-cleaned aura must not count after its native duration.
    run.app
        .world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs(60));
    assert_eq!(has_absorb(run.app.world()), 0.0);
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec258 profile/traits"]
fn shadow_literal_fixture_executes_profile_for_300s() {
    let report = measure(258, 1).expect("native Shadow fixture");
    assert_eq!(report["driver"], "LITERAL_SIMC_SHADOW_76f8118");
    assert!((report["simulated_seconds"].as_f64().unwrap() - 300.0).abs() < 0.001);
    assert_eq!(report["actor_kind"], "player");
    assert!(report["combat"]["dps"].as_f64().unwrap() > 0.0);
    let actions = report["combat"]["actions"].as_array().unwrap();
    assert_eq!(report["assisted_decisions"], 0);
    for spell in [8092, 34914, 15407, 228260, 1242189] {
        assert!(
            actions.iter().any(|action| action["spell_id"] == spell),
            "literal Shadow must actually execute native spell{spell}"
        );
    }
}
