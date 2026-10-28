//! Earned Coup de Grace action admission.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    queued_triggers::{self, ScaledTrigger},
    script_registry::SpellScriptHook,
};
use shared::components::PowerType;

const COUP_TALENT: u32 = 441423;
const COUP: u32 = 441776;
const DISPATCH: u32 = 2098;
const ESCALATING: u32 = 441786;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ActionAvailable(available),
    SpellScriptHook::AfterResolvedHits(execute_coup),
];

fn execute_coup(world: &mut World, ctx: &EffectCtx) {
    if ctx.spell.id != COUP || ctx.depth != 0 {
        return;
    }
    let target = ctx
        .spell
        .cast_primary_target
        .expect("Coup has a native primary target");
    let points =
        ctx.spell
            .cast_policy_context
            .as_ref()
            .and_then(|context| {
                context.resources.iter().find_map(|(power, amount)| {
                    (*power == PowerType::ComboPoints).then_some(*amount)
                })
            })
            .expect("Coup retains its effective combo points");
    let bonus_points = spell_cast::scripts::script_effect_value(world, ctx.caster, COUP_TALENT, 2);
    let fazed = world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == 441224 && aura.caster == ctx.caster)
    });
    if fazed && auras::has_aura(world, ctx.caster, 441321) {
        let stacks =
            spell_cast::scripts::script_effect_value(world, ctx.caster, COUP_TALENT, 1) as u32;
        for _ in 0..stacks {
            spell_cast::cast_triggered(world, ctx.caster, 441326, Some(ctx.caster), ctx.depth + 1);
        }
    }
    // SimC76f8118 sc_rogue.cpp6863–6895: actual Coup schedules its three
    // native children with the frozen effective points plus the talent bonus.
    for (spell_id, delay_ms) in [(462140, 0), (462239, 300), (462240, 1000)] {
        queued_triggers::schedule_scaled_full_trigger(
            world,
            ctx,
            ScaledTrigger {
                target,
                spell_id,
                delay_ms,
                multiplier: points + bonus_points,
            },
        );
    }
    // SimC76f8118 sc_rogue.cpp6888–6891 uses Coup's bonus effective points
    // for both selected Gravedigger finisher triggers.
    super::outlaw_gravedigger::trigger_finisher(world, ctx, target, points + bonus_points);
    auras::remove_auras(world, ctx.caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == ESCALATING && aura.caster == ctx.caster
    });
}

fn available(world: &World, caster: Entity, spell: u32) -> bool {
    if spell != COUP && spell != DISPATCH {
        return true;
    }
    // SimC76f8118 sc_rogue.cpp3797–3802/6898–6903: maximum earned
    // Escalating Blade replaces Dispatch with Coup until it is consumed.
    let earned = auras::has_aura(world, caster, COUP_TALENT)
        && world.get::<ActiveAuras>(caster).is_some_and(|auras| {
            auras.iter().any(|aura| {
                aura.spell_id == ESCALATING
                    && aura.caster == caster
                    && aura.max_stacks > 0
                    && aura.stacks == aura.max_stacks
            })
        });
    if spell == COUP { earned } else { !earned }
}
