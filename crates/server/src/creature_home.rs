//! Ordinary home arrival shared by physical navigation and the frozen simulator.
use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::components::{Health, Mana, Position, UnitPose};

use super::{CreatureState, HomePosition, SpawnPoint};

pub(crate) fn position(
    spawn: &SpawnPoint,
    explicit: Option<&HomePosition>,
    escort: Option<Position>,
) -> Position {
    explicit
        .map(|home| home.position)
        .or(escort)
        .unwrap_or(Position {
            x: spawn.x,
            y: spawn.y,
            z: spawn.z,
        })
}

pub(crate) fn finish_return(world: &mut World, actor: Entity) {
    world.entity_mut(actor).insert(CreatureState::Idle);
    let health_changed = restore_vitals(world, actor);
    let pose_changed = restore_addons(world, actor);
    if (health_changed || pose_changed)
        && let Some(replication) = world.get_resource::<ReplicationDirtyQueue>()
    {
        replication.mark(actor);
    }
    crate::scripted_ai::dispatch(world, actor, crate::scripted_ai::Hook::JustReachedHome);
}

fn restore_vitals(world: &mut World, actor: Entity) -> bool {
    if let Some(mut mana) = world.get_mut::<Mana>(actor)
        && mana.current != mana.max
    {
        mana.current = mana.max;
    }
    // Creature::SetSpawnHealth leaves regenerating health to the ordinary timer.
    if world
        .get::<crate::spell_cast::powers::CreatureHealthRegenTimer>(actor)
        .is_some()
    {
        return false;
    }
    let Some(mut health) = world.get_mut::<Health>(actor) else {
        return false;
    };
    if health.current == health.max {
        return false;
    }
    health.current = health.max;
    true
}

fn restore_addons(world: &mut World, actor: Entity) -> bool {
    let auras = world
        .get::<crate::zones::SpawnAddonAuras>(actor)
        .map(|auras| auras.0.clone());
    if let Some(auras) = auras {
        world
            .entity_mut(actor)
            .insert(crate::zones::AddonAuras(auras));
    }
    let Some(pose) = world
        .get::<crate::zones::SpawnPose>(actor)
        .map(|pose| pose.0)
    else {
        return false;
    };
    world
        .get_mut::<UnitPose>(actor)
        .is_some_and(|mut current| current.set_if_neq(pose))
}
