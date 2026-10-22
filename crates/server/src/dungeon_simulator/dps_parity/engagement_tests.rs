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
            let retired = run.targets[1..].to_vec();
            replace_add_wave(&mut run, 30.0).unwrap();
            assert!(
                retired
                    .iter()
                    .all(|&target| run.app.world().get_entity(target).is_err())
            );
            assert_mutual_references(&run);
        }
    }
}

fn assert_native_aoe_cast(spec: u32) {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = prepare_follower(spec, 3);
    let aoe_spells: Vec<_> = run
        .app
        .world()
        .resource::<crate::followers::FollowerCatalog>()
        .kits
        .iter()
        .filter(|entry| entry.available(spec, 90) && entry.conditions.min_enemies >= 2)
        .map(|entry| entry.spell_id)
        .collect();
    assert!(
        !aoe_spells.is_empty(),
        "fixture must have authored multi-enemy entries"
    );
    engage(&mut run);
    advance_combat(&mut run).unwrap();
    let actions = report::combat(run.app.world(), &run.damage_targets, 300.0);
    assert!(
        actions["actions"].as_array().unwrap().iter().any(|action| {
            action["actor"].as_u64() == Some(run.actor.to_bits())
                && action["spell_id"]
                    .as_u64()
                    .is_some_and(|spell| aoe_spells.contains(&(spell as u32)))
                && action["count"].as_u64().unwrap() > 0
        }),
        "spec{spec} must actually cast an authored min_enemies>=2 entry {aoe_spells:?}; observed {actions}"
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
