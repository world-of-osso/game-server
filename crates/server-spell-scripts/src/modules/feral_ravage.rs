//! Feral Ravage is earned from landed white swings.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::melee::Hand;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use shared::components::ActiveSpec;

const RAVAGE_TALENT: u32 = 441583;
const RAVAGE_BUFF: u32 = 441585;
const LIMB_FROM_LIMB: u32 = 1271540;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(owns_proc),
    SpellScriptHook::AfterWhiteHandHit(after_white_hit),
];

#[derive(Component, Default)]
struct RavageAttempts(u32);

fn owns_proc(spell: u32) -> bool {
    spell == RAVAGE_TALENT
}

fn selected_effect(world: &World, actor: Entity, spell: u32, index: u8) -> Option<f32> {
    world
        .get::<ActiveAuras>(actor)?
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == actor)?
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .map(|effect| effect.amount)
}

// SimC76f8118 sc_druid.cpp9793–9800,9832–9836,12420–12434:
// Feral Ravage uses a 5% accumulated proc distribution on landed white swings.
fn after_white_hit(world: &mut World, actor: Entity, _: Hand, _: bool) {
    if world
        .get::<ActiveSpec>(actor)
        .is_none_or(|spec| spec.0 != 103)
        || selected_effect(world, actor, RAVAGE_TALENT, 0).is_none()
    {
        return;
    }
    let limb = selected_effect(world, actor, LIMB_FROM_LIMB, 0).unwrap_or(0.0);
    let chance = 0.05 * (1.0 + f64::from(limb) / 100.0);
    let coefficient = combat::random::accumulated_proc_constant(chance);
    let attempts = world
        .get::<RavageAttempts>(actor)
        .map_or(1, |state| state.0 + 1);
    let success = combat::random::random::<f64>() < coefficient * f64::from(attempts);
    world
        .entity_mut(actor)
        .insert(RavageAttempts(if success { 0 } else { attempts }));
    if success {
        spell_cast::cast_triggered(world, actor, RAVAGE_BUFF, Some(actor), 1);
    }
}
