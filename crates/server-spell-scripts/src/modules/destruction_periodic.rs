//! Destruction's landed Wither/Immolate ticks fund the shared native shard pool.
use super::combat;
use bevy::prelude::*;
use combat::random as rand;
use combat::spell_cast::effects::{EffectCtx, energize_unit};
use combat::spell_cast::powers::PowerTypeTable;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use shared::components::{ActiveSpec, PowerType};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::PeriodicPrimaryDamage(after_tick)];

fn after_tick(world: &mut World, ctx: &EffectCtx<'_>, _target: Entity, _amount: f32, crit: bool) {
    if world.get::<ActiveSpec>(ctx.caster).is_none_or(|spec| spec.0 != 267) {
        return;
    }
    let direct = match ctx.spell.id {
        157736 => 348,
        445474 => 445468,
        _ => return,
    };
    // SimC76f8118 sc_warlock_actions.cpp1108–1117/4047–4059 grants
    //0.1 displayed shard per landed Destruction tick; only a critical tick
    // rolls the native direct spell's effect1 chance for another fragment.
    let divisor = world.resource::<PowerTypeTable>().get(PowerType::SoulShards)
        .expect("native Destruction requires Soul Shards metadata").display_modifier;
    assert!(divisor > 0, "native Soul Shards display divisor must be positive");
    let fragment = 0.1 * divisor as f32;
    let extra = crit && rand::random::<f32>() * 100.0 < script_effect_value(world, ctx.caster, direct, 1);
    let amount = if extra { fragment * 2.0 } else { fragment };
    energize_unit(world, ctx, ctx.caster, PowerType::SoulShards, amount, true);
}
