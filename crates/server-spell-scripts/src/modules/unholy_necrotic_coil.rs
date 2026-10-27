//! Forbidden Knowledge's real Necrotic Coil damage children.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::{HitHook, SpellScriptHook}, targets};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EffectHit(effect_hit)];

fn effect_hit(spell: u32, index: u8) -> Option<HitHook> {
    match (spell, index) {
        (1242174, 1) => Some(physical_coils),
        (1242172, 0) => Some(shadow_coils),
        _ => None,
    }
}

fn enemies(world: &World, caster: Entity, primary: Entity) -> Vec<Entity> {
    let mut result = vec![primary];
    for enemy in combat::combat_refs::active_opponents(world, caster) {
        if enemy != primary && targets::is_alive(world, enemy)
            && targets::relation_matches(world, caster, enemy, targets::Relation::Enemy) {
            result.push(enemy);
        }
    }
    result
}

fn physical_coils(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    // SimC76f8118 sc_death_knight.cpp9879–9889/9907–9914: Necrotic Coil
    // executes the native physical child on the selected number of enemies.
    let count = combat::spell_cast::scripts::script_effect_value(world, caster, 1242158, 1) as usize;
    for enemy in enemies(world, caster, target).into_iter().take(count) {
        if !spell_cast::try_cast_triggered(world, caster, 1242172, Some(enemy), depth + 1) {
            warn!(?caster, ?enemy, "native Necrotic Coil physical child refused");
        }
    }
}

fn shadow_coils(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    // SimC76f8118 sc_death_knight.cpp9869–9889: each physical impact
    // executes the native shadow child on every enemy.
    for enemy in enemies(world, caster, target) {
        if !spell_cast::try_cast_triggered(world, caster, 1242178, Some(enemy), depth + 1) {
            warn!(?caster, ?enemy, "native Necrotic Coil shadow child refused");
        }
    }
}
