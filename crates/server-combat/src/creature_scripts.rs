//! Creature behaviour selected by `creature_template.ScriptName`.
//! Training dummies follow TC a352b1fa `npc_training_dummy` / `NullCreatureAI`.

use std::collections::HashMap;

use bevy::prelude::*;

pub struct CreatureScriptsPlugin;

impl Plugin for CreatureScriptsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            expire_training_dummy_combat.after(crate::combat_refs::expire_combat_refs_system),
        );
    }
}

/// TC npcs_special.cpp1382–1421: DamageTaken prevents health loss and restarts
/// only the non-DOT attacker's timer. Expiry ends its PvE reference, not an evade.
#[derive(Component, Debug, Default)]
pub struct TrainingDummy {
    combat_timers: HashMap<Entity, f64>,
}

impl TrainingDummy {
    fn take_expired_attackers(&mut self, now: f64) -> Vec<Entity> {
        self.combat_timers
            .extract_if(|_, deadline| now >= *deadline)
            .map(|(attacker, _)| attacker)
            .collect()
    }
}

pub(crate) const TRAINING_DUMMY_SCRIPT: &str = "npc_training_dummy";
const TRAINING_DUMMY_COMBAT_TIMEOUT_SECS: f64 = 5.0;

pub(crate) fn insert_script_components(entity: &mut EntityCommands, script_name: &str) {
    if script_name == TRAINING_DUMMY_SCRIPT {
        entity.insert(TrainingDummy::default());
    }
}

/// Health a hit of `damage` takes: none from a training dummy.
pub(crate) fn health_loss(training_dummy: bool, damage: f32) -> f32 {
    if training_dummy { 0.0 } else { damage }
}

/// Called on active combat entry and non-DOT DamageTaken, never on a DoT tick
/// or merely refreshing an existing reference with a nondamaging spell.
pub(crate) fn refresh_training_dummy_timer(world: &mut World, dummy: Entity, attacker: Entity) {
    if world.get::<TrainingDummy>(dummy).is_none() {
        return;
    }
    let deadline = world.resource::<Time>().elapsed_secs_f64() + TRAINING_DUMMY_COMBAT_TIMEOUT_SECS;
    world
        .get_mut::<TrainingDummy>(dummy)
        .expect("training dummy")
        .combat_timers
        .insert(attacker, deadline);
}

fn expire_training_dummy_combat(world: &mut World) {
    let now = world.resource::<Time>().elapsed_secs_f64();
    let expired: Vec<(Entity, Entity)> = world
        .query::<(Entity, &mut TrainingDummy)>()
        .iter_mut(world)
        .flat_map(|(dummy, mut state)| {
            state
                .take_expired_attackers(now)
                .into_iter()
                .map(move |attacker| (dummy, attacker))
        })
        .collect();
    for (dummy, attacker) in expired {
        crate::combat_refs::end_pve_reference(world, dummy, attacker);
    }
}

#[cfg(test)]
#[path = "creature_scripts_tests.rs"]
mod tests;
