//! Authoritative cast presentation only. Expiry does not resolve spell effects.
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::casting::{CastState, CastTickResult, GlobalCooldown, SpellCooldowns};
use shared::components::{Health, Mana, MovementSpeed, Player, Position, Resources};
use shared::protocol::{SpellCastIntent, StopSpellCast};
use shared::spell_data::{CastContext, ResourceType, SpellData, SpellTarget, validate_cast};

use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::spell_db::SpellDatabase;

pub(crate) fn receive_cast_requests(world: &mut World) {
    let mut receivers = world.query::<(
        &mut MessageReceiver<SpellCastIntent>,
        Option<&PlayerLink>,
        Has<Authenticated>,
        Has<InWorld>,
    )>();
    let requests: Vec<_> = receivers
        .iter_mut(world)
        .flat_map(|(mut receiver, link, auth, in_world)| {
            let owner = link.filter(|_| auth && in_world).map(|link| link.0);
            receiver
                .receive()
                .map(move |request| (owner, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (owner, request) in requests {
        let result = owner
            .ok_or("connection has no authenticated in-world owner")
            .and_then(|owner| start_cast(world, owner, &request));
        if let Err(reason) = result {
            warn!("Cast presentation rejected: {reason}");
        }
    }
}

fn start_cast(
    world: &mut World,
    owner: Entity,
    request: &SpellCastIntent,
) -> Result<(), &'static str> {
    let spell = world
        .get_resource::<SpellDatabase>()
        .ok_or("spell database unavailable")?
        .resolve(request.spell_id, &request.spell)
        .ok_or("unknown or ambiguous spell")?
        .clone();
    if !spell.cast_time.is_finite() || spell.cast_time <= 0.0 {
        return Err("instant spells have no cast presentation; effects are not implemented");
    }
    if world.get::<CastState>(owner).is_some() {
        return Err("already casting");
    }
    if world.get::<Player>(owner).is_none() {
        return Err("owner is not a player");
    }
    let target = if spell.target == SpellTarget::Self_ {
        Some(owner)
    } else {
        request
            .target_entity
            .and_then(|bits| Entity::try_from_bits(bits).ok())
    };
    let context = cast_context(world, owner, target, &spell)?;
    validate_cast(&spell, &context).map_err(|reason| {
        debug!("Cast validation failed: {reason:?}");
        "cast validation failed"
    })?;
    let mut cast = CastState::normal(
        spell.id,
        target.map_or(0, Entity::to_bits),
        spell.cast_time,
        spell.interruptible,
    );
    cast.spell_name = spell.name;
    cast.cast_while_moving = spell.cast_while_moving;
    world.entity_mut(owner).insert(cast);
    mark_dirty(world, owner);
    Ok(())
}

fn cast_context(
    world: &World,
    owner: Entity,
    target: Option<Entity>,
    spell: &SpellData,
) -> Result<CastContext, &'static str> {
    let position = world
        .get::<Position>(owner)
        .ok_or("caster position missing")?;
    let target_position = target.and_then(|target| world.get::<Position>(target));
    let distance = target_position.map_or(0.0, |target| {
        Vec3::new(position.x, position.y, position.z)
            .distance(Vec3::new(target.x, target.y, target.z))
    });
    if !distance.is_finite() {
        return Err("invalid position");
    }
    let resource_available = spell.cost.as_ref().and_then(|cost| match cost.resource {
        ResourceType::Mana => world.get::<Mana>(owner).map(|mana| mana.current),
        resource => world
            .get::<Resources>(owner)
            .and_then(|pools| pools.get(resource))
            .map(|pool| pool.current),
    });
    Ok(CastContext {
        distance,
        // Current server model has no faction component: players are friendly, NPCs hostile.
        target_is_friendly: target.is_some_and(|entity| world.get::<Player>(entity).is_some()),
        has_target: target_position.is_some()
            && target.is_some_and(|entity| {
                world
                    .get::<Health>(entity)
                    .is_some_and(|health| health.current > 0.0)
            }),
        caster_alive: world
            .get::<Health>(owner)
            .is_some_and(|health| health.current > 0.0),
        resource_available,
        cooldown_remaining: world
            .get::<SpellCooldowns>(owner)
            .map_or(0.0, |cooldowns| cooldowns.remaining(spell.id)),
        gcd_remaining: world
            .get::<GlobalCooldown>(owner)
            .map_or(0.0, |gcd| gcd.remaining),
        caster_moving: world
            .get::<MovementSpeed>(owner)
            .is_some_and(|speed| speed.0 != 0.0),
    })
}

pub(crate) fn receive_stop_requests(world: &mut World) {
    let mut receivers = world.query::<(
        &mut MessageReceiver<StopSpellCast>,
        Option<&PlayerLink>,
        Has<Authenticated>,
        Has<InWorld>,
    )>();
    let owners: Vec<_> = receivers
        .iter_mut(world)
        .filter_map(|(mut receiver, link, auth, in_world)| {
            let requested = receiver.receive().count() > 0;
            if requested && (!auth || !in_world || link.is_none()) {
                warn!("Stop cast rejected: no authenticated in-world owner");
            }
            link.filter(|_| requested && auth && in_world)
                .map(|link| link.0)
        })
        .collect();
    for owner in owners {
        if world.get::<CastState>(owner).is_some() {
            world.entity_mut(owner).remove::<CastState>();
            mark_dirty(world, owner);
        }
    }
}

pub(crate) fn advance_casts(
    mut commands: Commands,
    time: Res<Time>,
    mut casts: Query<(
        Entity,
        &mut CastState,
        Option<&Health>,
        Option<&MovementSpeed>,
    )>,
    dirty: Option<Res<ReplicationDirtyQueue>>,
) {
    for (entity, mut cast, health, speed) in &mut casts {
        let cancelled = health.is_none_or(|health| health.current <= 0.0)
            || (!cast.cast_while_moving && speed.is_some_and(|speed| speed.0 != 0.0));
        let finished = if cancelled {
            true
        } else {
            matches!(
                cast.tick(time.delta_secs()),
                CastTickResult::Completed | CastTickResult::ChannelComplete
            )
        };
        if finished {
            commands.entity(entity).remove::<CastState>();
        }
        if let Some(dirty) = &dirty {
            dirty.mark(entity);
        }
    }
}

fn mark_dirty(world: &World, entity: Entity) {
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(entity);
    }
}
