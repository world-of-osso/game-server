//! Selected Bursting Growth from actual owned Vines loss and landed Bite.
use super::super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraInstance, scripts::AuraScripts};
use combat::spell_cast::{self, effects::EffectCtx, script_registry::EnemyContextHitHook};

const BURSTING_GROWTH: u32 = 440120;
const BURST: u32 = 440122;
const VINES: u32 = 439531;
const BITE: u32 = 22568;

pub(super) fn register(scripts: &mut AuraScripts) {
    // The existing async scheduler reports every actual expired application,
    // including the last. Do not also attach on_natural_end and double the burst.
    scripts.0.entry(VINES).or_default().on_stack_loss = Some(burst_expired_stacks);
}

fn selected(world: &World, caster: Entity) -> bool {
    combat::auras::has_aura(world, caster, BURSTING_GROWTH)
        && combat::auras::has_aura(world, caster, super::THRIVING_GROWTH)
}

// SimC76f8118 sc_druid.cpp13755–13821 bursts when Vines application
// stacks expire; the native440122 area payload owns actual damage and targets.
fn burst_expired_stacks(world: &mut World, target: Entity, aura: &AuraInstance, lost: u8) {
    if !selected(world, aura.caster) {
        return;
    }
    for _ in 0..lost {
        spell_cast::cast_triggered(world, aura.caster, BURST, Some(target), 1);
    }
}

pub(super) fn enemy_hit(spell: u32) -> Option<EnemyContextHitHook> {
    (spell == BITE).then_some(burst_bite)
}

// SimC76f8118 sc_druid.cpp4295–4307: a landed primary Bite independently
// bursts only if this caster owns ticking Vines on that actual recipient.
fn burst_bite(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if ctx.spell.cast_primary_target != Some(target) || !selected(world, ctx.caster) {
        return;
    }
    let owned = world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == VINES && aura.caster == ctx.caster)
    });
    if owned {
        spell_cast::cast_triggered(world, ctx.caster, BURST, Some(target), ctx.depth + 1);
    }
}
