//! Frost64 owned Freezing and Shatter; native70448 supplies counts and damage.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraSource};
use combat::spell_cast::script_registry::{SpellScriptHook, TargetPlanHook};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{self, PlannedEffect, TargetPlan, effects::EffectCtx, targets};
use combat::spell_info::{SpellEffect, SpellInfo};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::BeforeDirectDamagePolicies(after_damage),
    SpellScriptHook::AfterPeriodicDamagePolicies(after_periodic_damage),
    SpellScriptHook::TargetPlan(target_plan_hook),
];

const FINGERS: u32 = 44544;
const THERMAL: u32 = 1247730;
const SHATTER: u32 = 1246769;
const FREEZING: u32 = 1221389;
const SHATTER_DAMAGE: u32 = 1246949;
const FROSTBITE: u32 = 1248825;

fn enabled(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 64)
        && auras::has_aura(world, caster, SHATTER)
}

fn apply_freezing(world: &mut World, caster: Entity, target: Entity, count: u8) {
    // SimC76f8118 sc_mage.cpp7105–7127: each landed producer adds to
    // this mage's own debuff; native aura options supply its cap and lifetime.
    for _ in 0..count {
        auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target,
                spell_id: FREEZING,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
}

pub(super) fn after_damage(world: &mut World, ctx: &EffectCtx, target: Entity, critical: bool) {
    if !enabled(world, ctx.caster) {
        return;
    }
    // sc_mage.cpp3947/4055/4248: payload impacts, not root executions.
    let effect = match ctx.spell.id {
        228597 => Some(0),
        228354 => Some(1),
        228600 => Some(2),
        _ => None,
    };
    if let Some(effect) = effect {
        let count = script_effect_value(world, ctx.caster, SHATTER, effect) as u8;
        apply_freezing(world, ctx.caster, target, count);
        apply_critical_freezing(world, ctx, target, critical);
    }
    if ctx.spell.id == 228598 {
        shatter_owned_freezing(world, ctx, target);
    }
}

fn apply_critical_freezing(world: &mut World, ctx: &EffectCtx, target: Entity, critical: bool) {
    // sc_mage.cpp4094–4095/4304–4305: Frostbite adds its native count
    // on Frostbolt/Spike critical impacts, not on every damaging spell.
    if critical
        && matches!(ctx.spell.id, 228597 | 228600)
        && auras::has_aura(world, ctx.caster, FROSTBITE)
    {
        let count = script_effect_value(world, ctx.caster, FROSTBITE, 0) as u8;
        apply_freezing(world, ctx.caster, target, count);
    }
}

pub(super) fn after_periodic_damage(world: &mut World, ctx: &EffectCtx, target: Entity) {
    // sc_mage.cpp4995: each real Ray tick applies one Freezing stack.
    if ctx.spell.id == 205021 && enabled(world, ctx.caster) {
        apply_freezing(world, ctx.caster, target, 1);
    }
}

fn shatter_owned_freezing(world: &mut World, ctx: &EffectCtx, target: Entity) {
    let owned = world.get::<ActiveAuras>(target).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == FREEZING && aura.caster == ctx.caster)
            .map(|aura| (aura.instance_id, aura.stacks, aura.duration_ms))
    });
    // sc_mage.cpp4445–4473/7129–7170: use the launched missile's earned
    // Fingers/Thermal snapshot, not buffs already spent at execution.
    let fingers = ctx.spell.cast_launch_auras.contains(&FINGERS);
    let thermal = ctx.spell.cast_launch_auras.contains(&THERMAL);
    let base_limit = script_effect_value(world, ctx.caster, SHATTER, 3) as u8;
    let limit = base_limit * if thermal { 2 } else { 1 };
    let available = owned.map_or(0, |(_, stacks, _)| stacks);
    let shatter_stacks = if fingers { limit } else { available.min(limit) };
    if shatter_stacks > 0 {
        launch_shatter(world, ctx, target, shatter_stacks);
    }
    if let Some((instance, _, duration)) = owned {
        if !fingers {
            for _ in 0..shatter_stacks {
                auras::spend_stack(world, target, instance);
            }
        }
        auras::set_owned_duration(world, ctx.caster, target, FREEZING, duration);
    }
}

fn launch_shatter(world: &mut World, ctx: &EffectCtx, target: Entity, stacks: u8) {
    let Some(mut payload) = spell_cast::load_triggered_spell(world, SHATTER_DAMAGE, ctx.depth + 1)
    else {
        return;
    };
    payload.cast_damage_multiplier = Some(f32::from(stacks));
    spell_cast::launch_triggered_spell(world, ctx.caster, &payload, Some(target), ctx.depth + 1);
}

pub(super) fn target_plan_hook(spell: u32) -> Option<(targets::Relation, TargetPlanHook)> {
    (spell == SHATTER_DAMAGE).then_some((targets::Relation::Enemy, shatter_plan))
}

fn shatter_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    // sc_mage.cpp4328–4342: native effect0 is primary-only; effect1 is
    // Frostbite-only cleave and must never hit the primary a second time.
    let cleave = auras::has_aura(world, caster, FROSTBITE);
    spell
        .effects
        .iter()
        .enumerate()
        .filter(|(_, effect)| effect.index == 0 || cleave)
        .map(|(index, effect)| plan_shatter_effect(world, caster, spell, explicit, index, effect))
        .collect()
}

fn plan_shatter_effect(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
    index: usize,
    effect: &SpellEffect,
) -> PlannedEffect {
    let mut units = targets::select_targets(world, caster, explicit, spell, effect, 0, false)
        .expect("native Shatter target selectors");
    if effect.index == 1 {
        units.retain(|unit| Some(*unit) != explicit);
    }
    let cap = script_effect_value(world, caster, FROSTBITE, 1);
    let total_targets = units.len() + 1;
    let multiplier = if effect.index == 1 && cap > 0.0 && total_targets as f32 > cap {
        (cap / total_targets.min(20) as f32).sqrt()
    } else {
        1.0
    };
    let chain_multipliers = units.iter().map(|&unit| (unit, multiplier)).collect();
    PlannedEffect {
        index,
        units,
        dest: None,
        chain_multipliers,
        selected_units: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }
}
