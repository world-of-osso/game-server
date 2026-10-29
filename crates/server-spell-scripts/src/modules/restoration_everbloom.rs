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

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResolvedHeal(everbloom),
    SpellScriptHook::AfterResolvedHits(blooming_frenzy),
];

fn blooming_frenzy(world: &mut World, ctx: &EffectCtx<'_>) {
    use combat::auras::ActiveAuras;
    const FRENZY: u32 = 1_244_470;
    if ctx.spell.id != 18_562 || !combat::auras::has_aura(world, ctx.caster, FRENZY) {
        return;
    }
    let count = combat::spell_cast::scripts::script_effect_value(world, ctx.caster, FRENZY, 0)
        .max(0.0) as u32;
    let targets: Vec<_> = world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .filter_map(|(unit, auras)| {
            auras
                .iter()
                .any(|aura| aura.spell_id == 33_763 && aura.caster == ctx.caster)
                .then_some(unit)
        })
        .collect();
    for target in targets {
        // WCL mR86tKbdCqF1A2NY fight4 actor4: the three blooms land at 0, ~250 and ~500ms.
        for tick in 0..count {
            combat::spell_cast::queued_triggers::schedule_full_trigger(
                world,
                ctx,
                target,
                BLOOM,
                tick * 250,
            );
        }
    }
}

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
