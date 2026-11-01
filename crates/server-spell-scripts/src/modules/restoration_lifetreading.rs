//! 70448 Lifetreading 1217941: Efflorescence grows beneath the Lifebloom target.
//! The talent overrides 145205 with 1232285; WCL kD8MqXLV6CFYHN2p fight 3 actor
//! 108 has no Efflorescence casts yet 81269 pulses every ~2.1s all fight, and
//! Lifebloom is recast every ~10.6s, well inside Efflorescence's duration.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance};
use combat::spell_cast::script_registry::SpellScriptHook;

const TALENT: u32 = 1217941;
const LIFEBLOOM: u32 = 33763;
const EFFLORESCENCE: u32 = 1232285;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterAuraApplication(grow_under_lifebloom)];

fn grow_under_lifebloom(world: &mut World, target: Entity, aura: &AuraInstance) {
    if aura.spell_id != LIFEBLOOM || !auras::has_aura(world, aura.caster, TALENT) {
        return;
    }
    combat::spell_cast::cast_triggered(world, aura.caster, EFFLORESCENCE, Some(target), 1);
}
