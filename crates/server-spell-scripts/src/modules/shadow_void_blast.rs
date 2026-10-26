//! SimC76f8118 sc_priest.cpp344–413: Rift replaces ordinary Mind Blast.
//! Native450404 supplies the override and shared charge-category modifier.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, RemoveReason};
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use shared::casting::CastState;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterAuraApplication(earn_override),
    SpellScriptHook::ActionAvailable(available),
];
const TALENT: u32 = 450405;
const OVERRIDE: u32 = 450404;
const RIFT: u32 = 459314;
const TORRENT: u32 = 263165;
const MIND_BLAST: u32 = 8092;
const VOID_BLAST: u32 = 450983;

fn earn_override(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.spell_id != RIFT || aura.caster != unit {
        return;
    }
    if !auras::has_aura(world, unit, TALENT) {
        return;
    }
    // Native332 maps known8092 to450983; never insert either into KnownSpells.
    spell_cast::cast_triggered(world, unit, OVERRIDE, Some(unit), 1);
}

pub(super) fn expire_override(world: &mut World, caster: Entity) {
    auras::remove_auras(world, caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == OVERRIDE && aura.caster == caster
    });
}

fn available(world: &World, caster: Entity, spell: u32) -> bool {
    if world
        .get::<ActiveSpec>(caster)
        .is_none_or(|spec| spec.0 != 258)
    {
        return true;
    }
    if spell != MIND_BLAST && spell != VOID_BLAST {
        return true;
    }
    // The pinned action_ready predicate admits either an actual Rift or Torrent channel.
    let channel = world
        .get::<CastState>(caster)
        .is_some_and(|cast| cast.spell_id == TORRENT);
    let window = auras::has_aura(world, caster, RIFT) || channel;
    let replacement = window && auras::has_aura(world, caster, TALENT);
    if spell == VOID_BLAST {
        replacement
    } else {
        !replacement
    }
}
