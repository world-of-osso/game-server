//! Creature::DespawnOrUnsummon without death/loot/quest-credit side effects.
use bevy::prelude::*;

pub(super) fn despawn_creature(world: &mut World, entity: Entity, force_respawn_secs: u32) {
    crate::combat::combat_stop(world, entity);
    if world.get::<crate::zones::TempSummon>(entity).is_some() {
        crate::zones::unsummon(world, entity);
        return;
    }
    let Some(delay) = world.get::<crate::combat::RespawnDelay>(entity) else {
        world.entity_mut(entity).despawn();
        return;
    };
    let corpse_delay = world
        .get::<crate::combat::CorpseDelay>(entity)
        .map_or(0.0, |delay| delay.0);
    let respawn_timer = if force_respawn_secs > 0 {
        force_respawn_secs as f32
    } else {
        (delay.0 - corpse_delay).max(0.0)
    };
    crate::interest::set_hidden(world, entity, true);
    world
        .entity_mut(entity)
        .remove::<(
            super::super::movement::PointMovement,
            crate::combat::Corpse,
            lightyear::prelude::Replicate,
        )>()
        .insert(crate::combat::DeathState { respawn_timer });
}
