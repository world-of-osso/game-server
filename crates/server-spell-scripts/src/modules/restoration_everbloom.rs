//! 70448 Everbloom copies a Lifebloom bloom to nearby allies.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
    targets::{self, Relation},
};
use shared::components::Health;

const TALENT: u32 = 1_244_331;
const BLOOM: u32 = 33_778;
const HEAL: u32 = 1_244_341;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedHeal(everbloom)];

fn everbloom(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    raw: f32,
    _effective: f32,
    _critical: bool,
    _periodic: bool,
) {
    if ctx.spell.id != BLOOM || raw <= 0.0 || !combat::auras::has_aura(world, ctx.caster, TALENT) {
        return;
    }
    let spell = world
        .resource::<combat::spell_info::SpellStore>()
        .get(HEAL)
        .expect("Everbloom heal missing")
        .clone();
    let Some(center) = targets::position(world, target) else {
        return;
    };
    let mut allies = targets::area_units(
        world,
        ctx.caster,
        center,
        targets::radius(&spell.effects[0], 1),
        Relation::Ally,
        0,
    );
    allies.retain(|unit| world.get::<Health>(*unit).is_some_and(|h| h.current > 0.0));
    allies.sort_by(|a, b| {
        let a = world.get::<Health>(*a).unwrap();
        let b = world.get::<Health>(*b).unwrap();
        (a.current / a.max).total_cmp(&(b.current / b.max))
    });
    let aura = world
        .get::<combat::auras::ActiveAuras>(ctx.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == TALENT)
        .unwrap();
    let value = |index| {
        aura.effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("Everbloom talent effect missing")
            .amount
    };
    let count = value(2).max(0.0) as usize;
    allies.truncate(count);
    let amount = raw * value(1) / 100.0;
    let heal = EffectCtx {
        caster: ctx.caster,
        spell: &spell,
        depth: ctx.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    for ally in allies {
        effects::apply_fixed_heal(world, &heal, ally, amount);
    }
}
