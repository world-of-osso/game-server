//! 70448 Ancient Teachings transfers eligible damage to injured allies.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
    targets::{self, Relation},
};
use shared::components::Health;

const PASSIVE: u32 = 388023;
const HEAL: u32 = 388024;
const CRITICAL_HEAL: u32 = 388025;
const EMPERORS_ELIXIR: u32 = 1268807;
const JADEFIRE_DAMAGE: u32 = 1248815;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(heal_from_damage)];

fn heal_from_damage(world: &mut World, ctx: &EffectCtx<'_>, _target: Entity, crit: bool, damage: f32) {
    let caster = ctx.caster;
    if damage <= 0.0 || !combat::auras::has_aura(world, caster, PASSIVE) {
        return;
    }
    let scale = match ctx.spell.id {
        100780 | 100784 | 185099 | 117953 => 1.0,
        JADEFIRE_DAMAGE if combat::auras::has_aura(world, caster, EMPERORS_ELIXIR) => {
            combat::spell_cast::scripts::script_effect_value(world, caster, EMPERORS_ELIXIR, 1) / 100.0
        }
        _ => return,
    };
    let heal_id = if crit { CRITICAL_HEAL } else { HEAL };
    let spell = world.resource::<combat::spell_info::SpellStore>().get(heal_id)
        .expect("Ancient Teachings heal child missing").clone();
    let Some(center) = targets::position(world, caster) else { return };
    let radius = targets::radius(&spell.effects[0], 1);
    let mut allies = targets::area_units(world, caster, center, radius, Relation::Ally, 0);
    allies.retain(|&unit| world.get::<Health>(unit).is_some_and(|health| health.current > 0.0 && health.current < health.max));
    allies.sort_by(|&a, &b| {
        let a = world.get::<Health>(a).unwrap();
        let b = world.get::<Health>(b).unwrap();
        (a.current / a.max).total_cmp(&(b.current / b.max))
    });
    let limit = combat::spell_cast::scripts::script_effect_value(world, caster, PASSIVE, 1).max(0.0) as usize;
    allies.truncate(limit);
    if allies.is_empty() { return; }
    let percent = combat::spell_cast::scripts::script_effect_value(world, caster, PASSIVE, 0) / 100.0;
    let amount = damage * percent * scale / allies.len() as f32;
    let heal = EffectCtx { caster, spell: &spell, depth: ctx.depth + 1, dest: None, chain_multiplier: 1.0 };
    for ally in allies {
        effects::apply_heal(world, &heal, ally, amount, crit, false);
    }
}
