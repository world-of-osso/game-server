//! Every opponent in the specified fixture fight participates without changing dummy AI.
use super::*;
use crate::followers::simulator::ParityActor;
use shared::components::{Player, ThreatTable};

fn prepare_follower(spec: u32, count: usize) -> Run {
    let (profile, traits) = read_reference(spec).unwrap();
    prepare_reference(profile, &traits, count, ParityActor::Follower).unwrap()
}

fn assert_mutual_references(run: &Run) {
    let world = run.app.world();
    let owner = world
        .get::<crate::followers::Follower>(run.actor)
        .unwrap()
        .owner;
    assert!(world.get::<Player>(owner).is_some());
    for &target in &run.targets {
        assert!(
            crate::combat_refs::has_active_reference_to(world, target, owner)
                && crate::combat_refs::has_active_reference_to(world, owner, target),
            "each fixture opponent must have active mutual CombatRefs at engagement: {target:?}"
        );
        assert!(
            world.get::<ThreatTable>(target).is_none(),
            "native31146 remains passive without a threat table"
        );
    }
}

#[test]
#[ignore = "opt-in pinned native fixture opponents; requires private profile inputs"]
fn dps_parity_engagement_all_opponents_have_mutual_combat_refs() {
    let _seed_scope = crate::random::seed_scope(70_448);
    for count in [1, 3] {
        let mut run = prepare_follower(66, count);
        engage(&mut run);
        assert_eq!(run.targets.len(), count);
        assert_mutual_references(&run);
        if count == 3 {
            let started_at = run.app.world().resource::<Time>().elapsed();
            install_fixture_lifetimes(&mut run, started_at);
            let retired = retire_add_wave(&mut run);
            spawn_add_wave(&mut run, &retired, started_at).unwrap();
            assert!(
                retired
                    .iter()
                    .all(|&target| run.app.world().get_entity(target).is_err())
            );
            assert_mutual_references(&run);
        }
    }
}

/// Production APL action spells whose own successful cast damaged at least
/// two distinct fixture opponents.
fn multi_enemy_casts(run: &Run) -> BTreeMap<u32, usize> {
    let world = run.app.world();
    let input =
        server_follower_apl::action_metadata::read_inputs(world, run.actor, run.profile.spec_id)
            .unwrap();
    let actions =
        server_follower_apl::action_metadata::action_spells(run.profile.spec_id, &input).unwrap();
    let events = &world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let source = Some(run.actor.to_bits());
    let opponents: Vec<u64> = run.damage_targets.iter().map(|unit| unit.to_bits()).collect();
    let mut hit: BTreeMap<u32, std::collections::BTreeSet<u64>> = BTreeMap::new();
    for event in events.iter().filter(|event| {
        event.source == source
            && event.kind == shared::protocol::CombatLogKind::Damage
            && event.amount > 0
    }) {
        if let (Some(spell), Some(target)) = (event.spell_id, event.target) {
            if actions.contains(&spell) && opponents.contains(&target) {
                hit.entry(spell).or_default().insert(target);
            }
        }
    }
    let cast = |spell: u32| {
        events.iter().any(|event| {
            event.source == source
                && event.kind == shared::protocol::CombatLogKind::CastSuccess
                && event.spell_id == Some(spell)
        })
    };
    hit.into_iter()
        .filter(|(spell, targets)| targets.len() >= 2 && cast(*spell))
        .map(|(spell, targets)| (spell, targets.len()))
        .collect()
}

/// The production APL follower, driven as in measurement, casts an action
/// that damages several opponents.
fn assert_native_aoe_cast(spec: u32) {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(spec, 3).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::followers::Follower>(run.actor)
            .is_some(),
        "spec{spec} multi-enemy check requires a follower actor"
    );
    assert!(
        run.spec_driver.is_some(),
        "spec{spec} follower must run its registered production APL driver"
    );
    engage(&mut run);
    advance_combat(&mut run).unwrap();
    let casts = multi_enemy_casts(&run);
    assert!(
        !casts.is_empty(),
        "spec{spec} must actually cast an APL action that damages >=2 opponents; observed {}",
        report::combat(run.app.world(), &run.damage_targets, 300.0)
    );
}

#[test]
#[ignore = "opt-in native300s Protection Warrior multi-enemy cast; requires private inputs"]
fn dps_parity_engagement_protection_warrior_casts_multi_enemy_aoe() {
    assert_native_aoe_cast(73);
}

#[test]
#[ignore = "opt-in native300s Fire multi-enemy cast; requires private inputs"]
fn dps_parity_engagement_fire_casts_multi_enemy_aoe() {
    assert_native_aoe_cast(63);
}

#[test]
#[ignore = "opt-in native300s Feral multi-enemy cast; requires private inputs"]
fn dps_parity_engagement_feral_casts_multi_enemy_aoe() {
    assert_native_aoe_cast(103);
}

