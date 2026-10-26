//! Frost Shatter and Ray followups; native70448 owns payloads and proc values.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraSource};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(after_damage)];
const HAND: u32 = 1262769;
const HAND_TALENT: u32 = 1262935;
const HAND_SECOND: u32 = 1262981;
const HAND_RAY: u32 = 1263249;
const TIER: u32 = 1296585;

fn after_damage(world: &mut World, ctx: &EffectCtx, target: Entity, _crit: bool, _amount: f32) {
    if world
        .get::<ActiveSpec>(ctx.caster)
        .is_none_or(|spec| spec.0 != 64)
    {
        return;
    }
    match ctx.spell.id {
        1246949 if ctx.spell.cast_primary_target == Some(target) => {
            let stacks = ctx
                .spell
                .cast_damage_multiplier
                .expect("native Shatter carries paid stack count") as u32;
            roll_hand(world, ctx, target, stacks);
            roll_tier_icicles(world, ctx.caster, stacks);
        }
        205021 => ray_hand(world, ctx, target),
        HAND => apply_hand_freezing(world, ctx.caster, target),
        _ => {}
    }
}

fn roll_hand(world: &mut World, ctx: &EffectCtx, target: Entity, stacks: u32) {
    if !auras::has_aura(world, ctx.caster, HAND_TALENT) {
        return;
    }
    // SimC76f8118 sc_mage.cpp7155–7159: one roll per successful paid
    // Shatter, with the native second talent's per-stack contribution.
    let mut chance = script_effect_value(world, ctx.caster, HAND_TALENT, 0);
    if auras::has_aura(world, ctx.caster, HAND_SECOND) {
        chance += stacks as f32 * 0.1 * script_effect_value(world, ctx.caster, HAND_SECOND, 0);
    }
    if combat::random::random::<f32>() * 100.0 < chance {
        launch_hand(world, ctx, target);
    }
}

fn launch_hand(world: &mut World, ctx: &EffectCtx, target: Entity) {
    spell_cast::cast_triggered(world, ctx.caster, HAND, Some(target), ctx.depth + 1);
    // sc_mage.cpp5449–5451/6458–6462: an actual Hand opens the selected
    // native buff; its authored default is one tenth of talent2 effect2.
    if !auras::has_aura(world, ctx.caster, HAND_SECOND) {
        return;
    }
    let amount = script_effect_value(world, ctx.caster, HAND_SECOND, 1) * 0.1;
    let mut buff = world
        .resource::<SpellStore>()
        .get(1263263)
        .expect("native Hand buff")
        .clone();
    for effect in buff.effects.iter_mut().filter(|effect| effect.aura == 108) {
        effect.base_points = amount;
    }
    if !spell_cast::launch_triggered_spell(
        world,
        ctx.caster,
        &buff,
        Some(ctx.caster),
        ctx.depth + 1,
    ) {
        warn!(caster = ?ctx.caster, "Native Hand buff refused");
    }
}

fn apply_hand_freezing(world: &mut World, caster: Entity, target: Entity) {
    // sc_mage.cpp5445: the actual Hand impact earns native talent1 effect2.
    let count = script_effect_value(world, caster, HAND_TALENT, 1) as u32;
    for _ in 0..count {
        auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target,
                spell_id: 1221389,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
}

fn roll_tier_icicles(world: &mut World, caster: Entity, stacks: u32) {
    if !auras::has_aura(world, caster, TIER) {
        return;
    }
    // sc_mage.cpp7171–7181/7186–7200: native tier chance per shattered
    // stack, not one roll per damage effect or per Shatter cast.
    let chance = script_effect_value(world, caster, TIER, 1);
    for _ in 0..stacks {
        if combat::random::random::<f32>() * 100.0 >= chance {
            continue;
        }
        apply_self_aura(world, caster, 205473);
        let full = world.get::<ActiveAuras>(caster).is_some_and(|auras| {
            auras.iter().any(|aura| {
                aura.spell_id == 205473
                    && aura.caster == caster
                    && aura.stacks >= aura.max_stacks.max(1)
            })
        });
        if full {
            apply_self_aura(world, caster, 1222865);
        }
    }
}

fn apply_self_aura(world: &mut World, caster: Entity, spell_id: u32) {
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn ray_hand(world: &mut World, ctx: &EffectCtx, target: Entity) {
    if !auras::has_aura(world, ctx.caster, HAND_RAY) {
        return;
    }
    // sc_mage.cpp5005–5013: calculate the authored interval from native
    // Ray duration/period and selected Hand effect1. Use the real scheduler
    // ordinal carried by this tick, including catch-up ticks, never live count.
    let ticks = ctx
        .spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration.as_ref())
        .expect("native Ray duration")
        .duration_ms as f32
        / ctx.spell.effects[0].aura_period_ms as f32;
    let count = script_effect_value(world, ctx.caster, HAND_RAY, 0);
    let every = (ticks / count).round().max(1.0) as u64;
    let ordinal = ctx
        .spell
        .cast_periodic_ordinal
        .expect("native Ray tick ordinal");
    if ordinal > 0 && (ordinal - 1) % every == 0 {
        launch_hand(world, ctx, target);
    }
}
