//! 70448 Ancestral Awakening transfers direct Healing Wave and Riptide healing.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
    targets::{self, Relation},
};
use shared::components::Health;

const TALENT: u32 = 382_309;
const HEAL: u32 = 382_311;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedHeal(awaken)];

fn awaken(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    _target: Entity,
    raw: f32,
    _effective: f32,
    critical: bool,
    periodic: bool,
) {
    if periodic || !matches!(ctx.spell.id, 61_295 | 77_472) || raw <= 0.0 {
        return;
    }
    let Some(aura) = world
        .get::<ActiveAuras>(ctx.caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == TALENT))
    else {
        return;
    };
    let value = |index| {
        aura.effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("Ancestral Awakening effect missing")
            .amount
    };
    let (percent, chance) = (value(0), value(if critical { 2 } else { 1 }));
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    let Some(center) = targets::position(world, ctx.caster) else {
        return;
    };
    let mut allies = targets::area_units(world, ctx.caster, center, 40.0, Relation::Ally, 0);
    allies.retain(|unit| {
        world
            .get::<Health>(*unit)
            .is_some_and(|h| h.current > 0.0 && h.current < h.max)
    });
    let Some(ally) = allies.into_iter().min_by(|a, b| {
        let a = world.get::<Health>(*a).unwrap();
        let b = world.get::<Health>(*b).unwrap();
        (a.current / a.max).total_cmp(&(b.current / b.max))
    }) else {
        return;
    };
    let spell = world
        .resource::<combat::spell_info::SpellStore>()
        .get(HEAL)
        .expect("Ancestral Awakening heal missing")
        .clone();
    let heal = EffectCtx {
        caster: ctx.caster,
        spell: &spell,
        depth: ctx.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    effects::apply_fixed_heal(world, &heal, ally, raw * percent / 100.0);
}
