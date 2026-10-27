//! Source-qualified native Void Ray channel children.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::KnownSpells;
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::{ContextHitHook, SpellScriptHook},
    targets,
};
use shared::casting::{CastState, CastType};

const VOID_RAY: u32 = 473728;
const META: u32 = 1217607;
const FOCUSED_RAY: u32 = 1240203;

#[derive(Component)]
struct VoidRayTick(u32);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OnCast(snapshot_tick),
    SpellScriptHook::ChannelPeriodic(periodic_trigger),
    SpellScriptHook::TargetContextDamageMultiplier(focused_ray),
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

fn focused_ray(world: &World, ctx: &EffectCtx<'_>, _: Entity, periodic: bool) -> f32 {
    if periodic || !matches!(ctx.spell.id, 1213649 | 1214595) {
        return 1.0;
    }
    if !world
        .get::<KnownSpells>(ctx.caster)
        .is_some_and(|known| known.contains(FOCUSED_RAY))
    {
        return 1.0;
    }
    // SimC76f8118 sc_demon_hunter.cpp6304–6321: the native talent applies
    // its effect1 percentage only through its effect2 resolved-target threshold.
    let targets = spell_cast::resolving_targets(world, ctx)
        .expect("native Void Ray tick retains resolved targets")
        .iter()
        .filter(|&&target| {
            targets::relation_matches(world, ctx.caster, target, targets::Relation::Enemy)
        })
        .count();
    let limit = spell_cast::scripts::script_effect_value(world, ctx.caster, FOCUSED_RAY, 1);
    if targets == 0 || targets as f32 > limit {
        return 1.0;
    }
    let percent = spell_cast::scripts::script_effect_value(world, ctx.caster, FOCUSED_RAY, 0);
    1.0 + percent / 100.0
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
