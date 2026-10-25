//! Creature::DespawnOrUnsummon without death/loot/quest-credit side effects.
use bevy::prelude::*;

#[derive(Component, Clone, Copy)]
struct TimedDespawn {
    remaining_ms: u32,
    force_respawn_secs: u32,
    fade_before_despawn: bool,
}

pub(in crate::scripted_ai) fn schedule_despawn(
    world: &mut World,
    entity: Entity,
    delay_ms: u32,
    force_respawn_secs: u32,
    fade_before_despawn: bool,
) {
    world.entity_mut(entity).insert(TimedDespawn {
        remaining_ms: delay_ms.max(1),
        force_respawn_secs,
        fade_before_despawn,
    });
}

pub(crate) fn tick_lifetimes_system(world: &mut World) {
    let diff_ms = world.resource::<Time>().delta().as_millis() as u32;
    tick_lifetimes(world, diff_ms);
}

pub(super) fn tick_lifetimes(world: &mut World, diff_ms: u32) {
    super::summons::tick_summons(world, diff_ms);
    let mut expired = Vec::new();
    for (entity, mut timer) in world.query::<(Entity, &mut TimedDespawn)>().iter_mut(world) {
        if timer.remaining_ms > diff_ms {
            timer.remaining_ms -= diff_ms;
        } else {
            expired.push((entity, *timer));
        }
    }
    for (entity, timer) in expired {
        if timer.fade_before_despawn {
            crate::interest::set_hidden(world, entity, true);
            world
                .entity_mut(entity)
                .remove::<lightyear::prelude::Replicate>();
            schedule_despawn(world, entity, 5000, timer.force_respawn_secs, false);
        } else {
            world.entity_mut(entity).remove::<TimedDespawn>();
            despawn_creature(world, entity, timer.force_respawn_secs);
        }
    }
}

pub(in crate::scripted_ai) fn despawn_creature(
    world: &mut World,
    entity: Entity,
    force_respawn_secs: u32,
) {
    crate::combat::combat_stop(world, entity);
    crate::spell_cast::cancel_cast(world, entity);
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
