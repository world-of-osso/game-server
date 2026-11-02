//! TrinityCore spell_generic.cpp:5144–5211: shared raid-haste exclusions and lockouts.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, bloodlust};
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AuraRedirect(exclude_locked_target),
    SpellScriptHook::AfterAuraApplication(apply_lockout),
];

fn exclude_locked_target(
    world: &mut World,
    _caster: Entity,
    target: Entity,
    spell: u32,
    _depth: u8,
) -> bool {
    bloodlust::lockout_for(spell).is_some() && bloodlust::has_lockout(world, target)
}

fn apply_lockout(world: &mut World, target: Entity, aura: &AuraInstance) {
    let Some(lockout) = bloodlust::lockout_for(aura.spell_id) else {
        return;
    };
    // The affected unit owns its own native ten-minute debuff.
    auras::apply_aura(
        world,
        &auras::AuraApplication {
            caster: target,
            target,
            spell_id: lockout,
            effect_mask: u32::MAX,
            source: auras::AuraSource::Cast,
        },
    );
}
