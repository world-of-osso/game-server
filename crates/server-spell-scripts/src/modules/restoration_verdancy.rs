//! 70448 Verdancy 392325: when Lifebloom blooms (33778), up to s1 allies within
//! the caster's Efflorescence are healed with 392329.
use super::combat;
use bevy::prelude::*;
use combat::random::{self as rand, seq::SliceRandom};
use combat::spell_cast::{
    script_registry::{HitHook, SpellScriptHook},
    scripts::script_effect_value,
    summon::SummonedCreature,
    targets::{self, Relation},
};
use combat::spell_info::SpellStore;
use shared::components::{Health, UnitIsPet};

const TALENT: u32 = 392325;
const BLOOM: u32 = 33778;
const HEAL: u32 = 392329;
/// Efflorescence's heal names the blossom's area.
const EFFLORESCENCE_HEAL: u32 = 81269;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::Hit(hit_hook)];

fn hit_hook(spell: u32) -> Option<HitHook> {
    (spell == BLOOM).then_some(heal_within_efflorescence)
}

fn heal_within_efflorescence(world: &mut World, caster: Entity, _target: Entity, depth: u8) {
    if !combat::auras::has_aura(world, caster, TALENT) {
        return;
    }
    let Some(center) = own_efflorescence(world, caster).and_then(|unit| targets::position(world, unit))
    else {
        return;
    };
    let limit = script_effect_value(world, caster, TALENT, 0) as usize;
    for ally in select_allies(world, caster, center, limit) {
        combat::spell_cast::cast_triggered(world, caster, HEAL, Some(ally), depth + 1);
    }
}

fn own_efflorescence(world: &mut World, caster: Entity) -> Option<Entity> {
    world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .find(|(_, summon)| {
            summon.owner == caster
                && combat::spell_cast::efflorescence::is_efflorescence(summon.spell_id)
        })
        .map(|(unit, _)| unit)
}

/// Living party members (no summons or pets) inside the blossom, injured first,
/// in random order.
fn select_allies(world: &mut World, caster: Entity, center: Vec3, limit: usize) -> Vec<Entity> {
    let radius = targets::radius(
        &world
            .resource::<SpellStore>()
            .get(EFFLORESCENCE_HEAL)
            .expect("Efflorescence heal missing")
            .effects[0],
        1,
    );
    let mut allies = targets::area_units(world, caster, center, radius, Relation::Raid, 0);
    allies.retain(|&unit| {
        world.get::<SummonedCreature>(unit).is_none()
            && !world.get::<UnitIsPet>(unit).is_some_and(|pet| pet.0)
            && world.get::<Health>(unit).is_some_and(|health| health.current > 0.0)
    });
    allies.shuffle(&mut rand::thread_rng());
    allies.sort_by_key(|&unit| {
        world
            .get::<Health>(unit)
            .is_none_or(|health| health.current >= health.max)
    });
    allies.truncate(limit);
    allies
}
