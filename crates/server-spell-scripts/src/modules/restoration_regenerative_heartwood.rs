//! 70448 Regenerative Heartwood 392116: each of the caster's Rejuvenation and
//! Germination ticks also heals the ally protected by the caster's Ironbark for
//! s1% of the tick's raw healing with 392117 (s2 extends Ironbark through its
//! native spell modifier). SimC76f8118 marks it NYI. WCL kD8MqXLV6CFYHN2p fight3
//! actor108: all 966 noncritical 392117 heals follow a Rejuvenation tick by
//! 6–23 ms, including ticks on the Ironbark target itself (210); median 0.80×
//! the tick's raw healing.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
    scripts::script_effect_value,
};
use combat::spell_info::SpellStore;

const TALENT: u32 = 392_116;
const HEAL: u32 = 392_117;
const IRONBARK: u32 = 102_342;
const REJUVENATION: u32 = 774;
const GERMINATION: u32 = 155_777;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedHeal(share_tick)];

fn share_tick(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    _target: Entity,
    raw: f32,
    _effective: f32,
    _critical: bool,
    periodic: bool,
) {
    if !periodic
        || raw <= 0.0
        || !matches!(ctx.spell.id, REJUVENATION | GERMINATION)
        || !combat::auras::has_aura(world, ctx.caster, TALENT)
    {
        return;
    }
    let protected = ironbark_holders(world, ctx.caster);
    if protected.is_empty() {
        return;
    }
    let percent = script_effect_value(world, ctx.caster, TALENT, 0);
    let spell = world
        .resource::<SpellStore>()
        .get(HEAL)
        .expect("Regenerative Heartwood heal missing")
        .clone();
    let heal = EffectCtx {
        caster: ctx.caster,
        spell: &spell,
        depth: ctx.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    for ally in protected {
        effects::apply_fixed_heal(world, &heal, ally, raw * percent / 100.0);
    }
}

fn ironbark_holders(world: &mut World, caster: Entity) -> Vec<Entity> {
    world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .filter(|(_, auras)| {
            auras
                .iter()
                .any(|aura| aura.spell_id == IRONBARK && aura.caster == caster)
        })
        .map(|(unit, _)| unit)
        .collect()
}
