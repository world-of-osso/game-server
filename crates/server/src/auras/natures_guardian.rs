//! TC a352b1fa spell_shaman.cpp2348–2376; SpellAuraEffects.cpp1463–1494/6126–6149.
use bevy::prelude::*;
use shared::components::Health;

use super::AuraInstance;
use super::procs::AuraEvent;
use super::scripts::{AuraScript, AuraScripts};

const PASSIVE: u32 = 30_884;
const COOLDOWN: u32 = 445_698;
const HEAL: u32 = 31_616;

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        PASSIVE,
        AuraScript {
            check_proc: Some(check_health_and_cooldown),
            on_damage_proc: Some(cast_cooldown_then_heal),
            ..Default::default()
        },
    );
}

fn check_health_and_cooldown(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    let Some(target) = event.target else {
        return false;
    };
    let Some(health) = world.get::<Health>(target) else {
        return false;
    };
    let Some(effect) = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0 && effect.aura_type == 4)
    else {
        return false;
    };
    let threshold = (health.max * effect.amount / 100.0).trunc();
    health.current < threshold && !super::has_aura(world, target, COOLDOWN)
}

fn cast_cooldown_then_heal(
    world: &mut World,
    _unit: Entity,
    _aura: &AuraInstance,
    event: &AuraEvent,
) {
    let target = event
        .target
        .expect("Nature's Guardian proc requires its actual action target");
    // Source uses IGNORE_CAST_IN_PROGRESS, not FULL_MASK, for the cooldown.
    if let Err(failure) =
        crate::spell_cast::creature_cast_ignoring_in_progress(world, target, COOLDOWN, Some(target))
    {
        warn!("Nature's Guardian cooldown445698 failed for {target:?}: {failure:?}");
    }
    // TC does not prevent the DUMMY default: its real TriggerSpell follows
    // even if the cooldown cast fails. Preserve FULL_MASK minus power/reagents.
    crate::spell_cast::cast_costed_trigger_with_context(
        world,
        target,
        HEAL,
        Some(target),
        1,
        None,
        None,
    );
}
