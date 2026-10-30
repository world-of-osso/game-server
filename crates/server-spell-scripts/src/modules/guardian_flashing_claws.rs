//! SimC76f8118 sc_druid.cpp5985–5986,11893–11900: Thrash can repeat after 500ms.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{
    effects::EffectCtx, queued_triggers, script_registry::SpellScriptHook, targets,
};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::AfterResolvedHits(after_thrash),
];

fn declared_reach(
    world: &World,
    caster: Entity,
    spell: &combat::spell_info::SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    if spell.id != 77758
        || !combat::spell_cast::script_registry::reach_source_selected(world, caster, 393427)
    {
        return Default::default();
    }
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Thrash effect");
    let radius = effect.radius[1].expect("native Thrash radius").radius;
    combat::spell_cast::script_registry::ScriptReach::caster(radius)
}

fn after_thrash(world: &mut World, ctx: &EffectCtx<'_>) {
    if ctx.spell.id != 77758 {
        return;
    }
    let Some(chance) = world
        .get::<ActiveAuras>(ctx.caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 393427 && aura.caster == ctx.caster)
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map(|effect| effect.amount)
    else {
        return;
    };
    if combat::random::random::<f32>() >= chance / 100.0 {
        return;
    }
    let Some(origin) = targets::position(world, ctx.caster) else {
        return;
    };
    let radius = ctx
        .spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .and_then(|effect| effect.radius[1])
        .expect("Thrash radius")
        .radius;
    let candidates = targets::area_units(
        world,
        ctx.caster,
        origin,
        radius,
        targets::Relation::Enemy,
        0,
    );
    let Some(target) = ctx
        .primary_target()
        .filter(|target| candidates.contains(target))
        .or_else(|| candidates.first().copied())
    else {
        return;
    };
    queued_triggers::schedule_full_trigger(world, ctx, target, 77758, 500);
}
