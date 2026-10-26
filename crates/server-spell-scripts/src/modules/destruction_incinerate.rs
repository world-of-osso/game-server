//! Source-qualified extra Incinerate funding; the native base grant stays intact.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::effects::{EffectCtx, energize_unit};
use combat::spell_cast::powers::PowerTypeTable;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use shared::components::{ActiveSpec, PowerType};

const INCINERATE: u32 = 29722;
const BASE_ENERGIZE: u32 = 244670;
const DIABOLIC_EMBERS: u32 = 387173;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(fund_primary_hit)];

fn fund_primary_hit(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    critical: bool,
    _amount: f32,
) {
    let ordinary_primary = ctx.spell.id == INCINERATE
        && ctx.depth == 0
        && ctx.spell.cast_primary_target == Some(target);
    let destruction = world
        .get::<ActiveSpec>(ctx.caster)
        .is_some_and(|spec| spec.0 == 267);
    if !ordinary_primary || !destruction {
        return;
    }
    let bonus = read_bonus_fraction(world, ctx.caster);
    // SimC76f8118 sc_warlock_actions.cpp3927–3931/4015–4025:
    // native base244670 already pays its base amount. Add only the selected
    // talent's extra base amount and the actual critical fragment, both using
    // native values. Do not copy the base grant or create another shard pool.
    let base = script_effect_value(world, ctx.caster, BASE_ENERGIZE, 0);
    let extra = base * bonus + read_critical_fragment(world, critical, bonus);
    if extra > 0.0 {
        energize_unit(world, ctx, ctx.caster, PowerType::SoulShards, extra, false);
    }
}

fn read_critical_fragment(world: &World, critical: bool, bonus: f32) -> f32 {
    if !critical {
        return 0.0;
    }
    let divisor = world
        .resource::<PowerTypeTable>()
        .get(PowerType::SoulShards)
        .expect("native Destruction requires Soul Shards metadata")
        .display_modifier;
    assert!(
        divisor > 0,
        "native Soul Shards display divisor must be positive"
    );
    0.1 * divisor as f32 * (1.0 + bonus)
}

fn read_bonus_fraction(world: &World, caster: Entity) -> f32 {
    let selected = world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == DIABOLIC_EMBERS)
    });
    if selected {
        script_effect_value(world, caster, DIABOLIC_EMBERS, 0) / 100.0
    } else {
        0.0
    }
}
