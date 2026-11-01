//! Purgatory's lethal-damage rescue and heal-back window.
use bevy::prelude::*;
use shared::components::Health;

use super::combat;
use combat::auras::{self, AuraInstance, scripts::{AuraScript, AuraScripts}};
use combat::spell_cast::{self, script_registry::{LethalAbsorbHook, SpellScriptHook}};

const PURGATORY: u32 = 114556;
const SHROUD: u32 = 116888;
const PERDITION: u32 = 123981;
const EXPIRED_KILL: u32 = 123982;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LethalAbsorb(lookup),
    SpellScriptHook::SupportAuraRegistration(register),
];

fn lookup(spell: u32) -> Option<LethalAbsorbHook> {
    (spell == PURGATORY).then_some(rescue)
}

// Native70448 spell114556 description: prevented damage becomes a $116888d
// heal absorb; the effect cannot recur for $123981d.
fn rescue(world: &mut World, unit: Entity, loss: f32) -> Option<f32> {
    if auras::has_aura(world, unit, PERDITION) {
        return None;
    }
    let before = world.get::<Health>(unit)?.current;
    let absorbed = loss - (before - 1.0).max(0.0);
    if !spell_cast::try_cast_triggered(world, unit, PERDITION, Some(unit), 1) {
        return None;
    }
    spell_cast::cast_triggered_with_effect0_amount(world, unit, SHROUD, Some(unit), 1, absorbed);
    if !auras::has_aura(world, unit, SHROUD) {
        auras::remove_auras(world, unit, auras::RemoveReason::Cancelled, |aura| aura.spell_id == PERDITION);
        return None;
    }
    world.get_mut::<Health>(unit)?.current = 1.0;
    Some(absorbed)
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(SHROUD, AuraScript { on_natural_end: Some(expire_shroud), ..default() });
}

fn expire_shroud(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.effects.iter().any(|effect| effect.index == 0 && effect.amount > 0.0) {
        spell_cast::cast_triggered(world, unit, EXPIRED_KILL, Some(unit), 1);
    }
}
