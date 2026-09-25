//! Creature behaviour selected by `creature_template.ScriptName`, the
//! AzerothCore CreatureScript names. Only `npc_training_dummy` has one.

use bevy::prelude::*;
use shared::components::CombatStatus;

use crate::auras::{self, RemoveReason};

pub struct CreatureScriptsPlugin;

impl Plugin for CreatureScriptsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, training_dummy_evade_system);
    }
}

/// AzerothCore `npc_training_dummy` (scripts/World/npcs_special.cpp).
/// `DamageTaken` sets the damage to 0, so the dummy never loses health and
/// never dies, while the combat log still reports the full hit (it is sent
/// before `DealDamage`). Its 5 s `resetTimer`, restarted by every hit, is the
/// shared combat grace, which only damage refreshes; when it runs out the
/// dummy evades and `_EnterEvadeMode` → `RemoveEvadeAuras` strips the auras
/// others put on it.
#[derive(Component, Debug, Default)]
pub struct TrainingDummy {
    /// `CombatStatus` seen last tick, to catch the evade.
    in_combat: bool,
}

pub(crate) fn insert_script_components(entity: &mut EntityCommands, script_name: &str) {
    if script_name == "npc_training_dummy" {
        entity.insert(TrainingDummy::default());
    }
}

/// Health a hit of `damage` takes: none from a training dummy.
pub(crate) fn health_loss(training_dummy: bool, damage: f32) -> f32 {
    if training_dummy { 0.0 } else { damage }
}

fn training_dummy_evade_system(world: &mut World) {
    let evading: Vec<Entity> = world
        .query::<(Entity, &mut TrainingDummy, Option<&CombatStatus>)>()
        .iter_mut(world)
        .filter_map(|(entity, mut dummy, status)| {
            let in_combat = status.is_some_and(|status| status.0);
            let evades = dummy.in_combat && !in_combat;
            if dummy.in_combat != in_combat {
                dummy.in_combat = in_combat;
            }
            evades.then_some(entity)
        })
        .collect();
    for dummy in evading {
        debug!("Training dummy {dummy} evades");
        auras::remove_auras(world, dummy, RemoveReason::Evade, |aura| {
            aura.caster != dummy
        });
    }
}

#[cfg(test)]
#[path = "creature_scripts_tests.rs"]
mod tests;
