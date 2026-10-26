//! SimC76f8118 sc_demon_hunter.cpp4008–4017,5555,7296–7302.
use super::{combat, havoc::has_talent};
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

const EMPOWERED: u32 = 1271144;
const ETERNAL: u32 = 1271092;
const DANCE_RESET_DELAY_MS: u32 = 500;
pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterResourceValues(after_cast),
    SpellScriptHook::LateAuraRegistration(register),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    match spell {
        370965 => Some(arm_empowered_beam),
        188499 | 210152 => Some(consume_eternal),
        _ => None,
    }
}

fn arm_empowered_beam(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if has_talent(world, caster, 1270898) {
        combat::spell_cast::cast_triggered(world, caster, EMPOWERED, Some(caster), 1);
    }
}

pub(super) fn finish_beam(world: &mut World, caster: Entity) {
    if has_talent(world, caster, 1270901) {
        combat::spell_cast::cast_triggered(world, caster, ETERNAL, Some(caster), 1);
    }
}

pub(super) fn remove_beam(
    world: &mut World,
    caster: Entity,
    aura: &AuraInstance,
    _reason: RemoveReason,
) {
    if aura.caster == caster {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |buff| {
            buff.spell_id == EMPOWERED && buff.caster == caster
        });
    }
}

#[derive(Component)]
struct PendingEternalReset;

fn consume_eternal(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    // Source delays expiry and the reset by500ms after the actual dance.
    if auras::set_owned_duration(world, caster, caster, ETERNAL, DANCE_RESET_DELAY_MS) {
        world.entity_mut(caster).insert(PendingEternalReset);
    }
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(ETERNAL).or_default().on_remove = Some(expire_eternal);
}

fn expire_eternal(world: &mut World, caster: Entity, aura: &AuraInstance, reason: RemoveReason) {
    if aura.caster != caster {
        return;
    }
    let pending = world
        .entity_mut(caster)
        .take::<PendingEternalReset>()
        .is_some();
    if !pending || reason != RemoveReason::Expired {
        return;
    }
    for spell in [188499, 210152] {
        super::havoc_recovery::reset_action(world, caster, spell, false);
    }
}
