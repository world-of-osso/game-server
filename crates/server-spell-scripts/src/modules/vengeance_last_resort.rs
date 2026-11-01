//! Last Resort's lethal-damage rescue.
use bevy::prelude::*;
use shared::components::Health;

use super::combat;
use combat::auras::{self, RemoveReason};
use combat::spell_cast::{self, script_registry::{LethalAbsorbHook, SpellScriptHook}};

const LAST_RESORT: u32 = 209258;
const LOCKOUT: u32 = 209261;
const METAMORPHOSIS: u32 = 187827;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LethalAbsorb(lookup)];

fn lookup(spell: u32) -> Option<LethalAbsorbHook> {
    (spell == LAST_RESORT).then_some(rescue)
}

// Native70448 spell209258: fatal damage transforms the actor, once per $209261d.
fn rescue(world: &mut World, unit: Entity, loss: f32) -> Option<f32> {
    if auras::has_aura(world, unit, LOCKOUT) {
        return None;
    }
    let before = world.get::<Health>(unit)?.current;
    let absorbed = loss - (before - 1.0).max(0.0);
    if !spell_cast::try_cast_triggered(world, unit, LOCKOUT, Some(unit), 1) {
        return None;
    }
    world.get_mut::<Health>(unit)?.current = 1.0;
    if !spell_cast::try_cast_triggered(world, unit, METAMORPHOSIS, Some(unit), 1) {
        world.get_mut::<Health>(unit)?.current = before;
        auras::remove_auras(world, unit, RemoveReason::Cancelled, |aura| aura.spell_id == LOCKOUT);
        return None;
    }
    Some(absorbed)
}
