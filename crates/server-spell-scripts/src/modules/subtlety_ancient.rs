//! SimC76f8118 sc_rogue.cpp8668–8701: an earned Ancient Arts window
//! lets a landed damaging finisher collect stored points after payment.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::spell_cast::{
    effects::{self, EffectCtx},
    powers,
    script_registry::SpellScriptHook,
};
use shared::components::{ActiveSpec, PowerType};

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::ResolvedDamage(collect_after_finisher)];
const TALENT: u32 = 1268939;
const WINDOW: u32 = 1269163;
const STORAGE: u32 = 196911;

fn collect_after_finisher(world: &mut World, ctx: &EffectCtx, target: Entity, _: bool, _: f32) {
    let combo_spender = ctx
        .spell
        .power_costs
        .iter()
        .any(|cost| cost.power_type == PowerType::ComboPoints as i32);
    if ctx.depth != 0 || !combo_spender || !is_earned(world, ctx.caster) {
        return;
    }
    let Some((instance, stacks)) = read_storage(world, ctx.caster) else {
        return;
    };
    let (current, maximum) = powers::power_value(world, ctx.caster, PowerType::ComboPoints)
        .expect("native Ancient Arts requires the Rogue combo-point pool");
    let consumed = stacks.min((maximum - current).max(0.0) as u8);
    if consumed == 0 {
        return;
    }
    // Payment precedes native impact. The first landed AoE recipient collects
    // once; consuming the earned window prevents a second collection on siblings.
    effects::energize_unit(
        world,
        ctx,
        ctx.caster,
        PowerType::ComboPoints,
        f32::from(consumed),
        false,
    );
    for _ in 0..consumed {
        auras::spend_stack(world, ctx.caster, instance);
    }
    super::subtlety_clone::schedule(world, ctx, target, consumed);
    auras::remove_auras(world, ctx.caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == WINDOW && aura.caster == ctx.caster
    });
}

fn is_earned(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 261)
        && has_talent(world, caster, TALENT)
        && auras::has_aura(world, caster, WINDOW)
}

fn read_storage(world: &World, caster: Entity) -> Option<(u32, u8)> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == STORAGE && aura.caster == caster)
        .map(|aura| (aura.instance_id, aura.stacks))
}
