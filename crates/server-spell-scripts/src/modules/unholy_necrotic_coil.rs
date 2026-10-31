//! Forbidden Knowledge's real Necrotic Coil damage children.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::{HitHook, SpellScriptHook},
    targets,
};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::EffectHit(effect_hit),
    SpellScriptHook::AfterPolicyDirectDamage(extend_diseases),
];

fn declared_reach(
    world: &World,
    _caster: Entity,
    spell: &combat::spell_info::SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    if spell.id != 1242174 {
        return Default::default();
    }
    let coil = world
        .resource::<combat::spell_info::SpellStore>()
        .get(1242174)
        .expect("native Necrotic Coil");
    let effect = coil
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Necrotic Coil area effect");
    combat::spell_cast::script_registry::ScriptReach::target(targets::effect_radius(effect))
}

fn extend_diseases(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool) {
    if !matches!(ctx.spell.id, 47632 | 1242178) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp9844–9861/9869–9897/13586–13603:
    // the Coil damage or Necrotic shadow child extends diseases on each hit.
    let milliseconds = combat::spell_cast::scripts::script_effect_value(world, ctx.caster, 77575, 2)
        .max(0.0)
        .round() as u32;
    combat::auras::extend_owned_duration(
        world,
        ctx.caster,
        target,
        &[1240996, 191587],
        milliseconds,
    );
}

fn effect_hit(spell: u32, index: u8) -> Option<HitHook> {
    match (spell, index) {
        (1242174, 1) => Some(physical_coils),
        _ => None,
    }
}

fn enemies(world: &mut World, caster: Entity, primary: Entity) -> Vec<Entity> {
    let center = targets::position(world, primary).expect("Necrotic Coil target position");
    let radius = {
        let spell = world
            .resource::<combat::spell_info::SpellStore>()
            .get(1242174)
            .expect("native Necrotic Coil spell");
        targets::effect_radius(
            spell
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .expect("native Necrotic Coil area effect"),
        )
    };
    let mut result =
        targets::area_units(world, caster, center, radius, targets::Relation::Enemy, 0);
    result.retain(|enemy| *enemy != primary);
    result.insert(0, primary);
    result
}

fn physical_coils(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    // SimC76f8118 sc_death_knight.cpp9879–9889/9907–9914: Necrotic Coil
    // executes the native physical child on the selected number of enemies.
    let count =
        combat::spell_cast::scripts::script_effect_value(world, caster, 1242158, 1) as usize;
    let targets = enemies(world, caster, target);
    for enemy in targets.iter().copied().take(count) {
        if !spell_cast::try_cast_triggered(world, caster, 1242172, Some(enemy), depth + 1) {
            warn!(
                ?caster,
                ?enemy,
                "native Necrotic Coil physical child refused"
            );
        }
    }
    // SimC76f8118 sc_death_knight.cpp9869–9889: the physical action's
    // execute_action is one shadow AoE, not one AoE per physical impact.
    for enemy in targets {
        if !spell_cast::try_cast_triggered(world, caster, 1242178, Some(enemy), depth + 1) {
            warn!(?caster, ?enemy, "native Necrotic Coil shadow child refused");
        }
    }
}
