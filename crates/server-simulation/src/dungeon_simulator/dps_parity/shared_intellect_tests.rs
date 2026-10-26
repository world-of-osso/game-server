//! Pinned complete-profile intellect, before combat procs; diagnose native sources.
use super::*;
use crate::auras::ActiveAuras;
use crate::followers::Follower;
use crate::stats::{CharacterStats, StatModifiers};
use server::test_support::ReferenceRace;
use shared::components::{Player, UnitLevel};

fn infer_equipped_intellect(world: &World, actor: Entity, base: f32) -> f32 {
    let stats = world.get::<CharacterStats>(actor).unwrap();
    let mods = world.get::<StatModifiers>(actor).unwrap();
    let total = stats.primary.intellect / (1.0 + mods.primary_pct.intellect / 100.0);
    let create = base * (1.0 + mods.primary_base_pct.intellect / 100.0);
    (total - create) / (1.0 + mods.primary_bonus_pct.intellect / 100.0)
        - mods.primary_flat.intellect
}

fn read_primary_aura_sources(world: &World, actor: Entity) -> Vec<(u32, u32, f32, i32)> {
    let auras = world.get::<ActiveAuras>(actor).unwrap();
    auras
        .iter()
        .flat_map(|aura| {
            aura.effects
                .iter()
                .filter(|effect| matches!(effect.aura_type, 29 | 80 | 137))
                .map(move |effect| {
                    (
                        aura.spell_id,
                        effect.aura_type,
                        effect.amount * f32::from(aura.stacks),
                        effect.misc_values[0],
                    )
                })
        })
        .collect()
}

fn read_actor_identity(world: &World, actor: Entity) -> (u8, u8) {
    if let Some(player) = world.get::<Player>(actor) {
        return (player.class, player.race);
    }
    let follower = world
        .get::<Follower>(actor)
        .expect("native reference actor is a player or follower");
    let race = world
        .get::<ReferenceRace>(actor)
        .expect("native reference follower retains its declared race")
        .0;
    (follower.profile.class_id, race)
}

fn assert_profile_intellect(spec: u32, expected: f32) {
    let _seed = crate::random::seed_scope(70_448);
    let run = prepare_measurement(spec, 1).unwrap();
    let world = run.app.world();
    let actor = run.actor;
    let (class, race) = read_actor_identity(world, actor);
    let level = world.get::<UnitLevel>(actor).unwrap().0;
    let base = crate::stats::base_primary_stats(class, race, level)
        .unwrap()
        .intellect;
    let stats = world.get::<CharacterStats>(actor).unwrap();
    let mods = world.get::<StatModifiers>(actor).unwrap();
    let gear = infer_equipped_intellect(world, actor, base);
    let sources = read_primary_aura_sources(world, actor);
    // SimC reports integer attributes; tolerate only that final quantization.
    assert!(
        (stats.primary.intellect - expected).abs() < 1.0,
        "native spec{spec} intellect={} expected{expected}; base={base}; gear_with_enchants_and_gems={gear}; flat={}; total_pct={}; base_pct={}; bonus_pct={}; aura_sources={sources:?}",
        stats.primary.intellect,
        mods.primary_flat.intellect,
        mods.primary_pct.intellect,
        mods.primary_base_pct.intellect,
        mods.primary_bonus_pct.intellect
    );
}

#[test]
#[ignore = "requires frozen63/64/265 inputs and private native database"]
fn shared_intellect_fire_complete_profile_matches_pinned_sources() {
    assert_profile_intellect(63, 3600.0);
}

#[test]
#[ignore = "requires frozen63/64/265 inputs and private native database"]
fn shared_intellect_frost_complete_profile_matches_pinned_sources() {
    assert_profile_intellect(64, 3599.0);
}

#[test]
#[ignore = "requires frozen63/64/265 inputs and private native database"]
fn shared_intellect_affliction_complete_profile_matches_pinned_sources() {
    assert_profile_intellect(265, 3530.0);
}
