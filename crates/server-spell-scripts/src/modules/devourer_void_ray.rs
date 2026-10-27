//! Source-qualified native Void Ray channel children.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::{ContextHitHook, SpellScriptHook},
};
use shared::casting::{CastState, CastType};

const VOID_RAY: u32 = 473728;
const META: u32 = 1217607;

#[derive(Component)]
struct VoidRayTick(u32);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OnCast(snapshot_tick),
    SpellScriptHook::ChannelPeriodic(periodic_trigger),
];

fn snapshot_tick(world: &mut World, actor: Entity, spell: u32, _: u8) {
    if spell != VOID_RAY {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp6354–6367,6420–6427 selects
    // the native child at channel start, not again at each tick.
    let child = if combat::auras::has_aura(world, actor, META) {
        1214595
    } else {
        1213649
    };
    world.entity_mut(actor).insert(VoidRayTick(child));
}

fn periodic_trigger(spell: u32, index: u8) -> Option<ContextHitHook> {
    (spell == VOID_RAY && index == 0).then_some(cast_tick)
}

fn cast_tick(world: &mut World, ctx: &EffectCtx<'_>, _: Entity) {
    let actor = ctx.caster;
    if !world
        .get::<CastState>(actor)
        .is_some_and(|cast| cast.cast_type == CastType::Channel && cast.spell_id == VOID_RAY)
    {
        return;
    }
    let Some(child) = world.get::<VoidRayTick>(actor).map(|tick| tick.0) else {
        error!(?actor, "Void Ray channel missing its selected tick");
        return;
    };
    spell_cast::cast_triggered(world, actor, child, None, 1);
}
