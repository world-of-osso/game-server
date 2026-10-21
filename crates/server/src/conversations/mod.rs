//! Source-backed conversation runtime/payload. Protocol and TalkingHead presentation
//! follow the server slice; no replacement chat event is emitted here.
mod catalog;
mod ingress;
mod wire;
use bevy::prelude::*;
pub(crate) use catalog::{Catalog, load_catalog};
use shared::components::Position;

pub(crate) use shared::protocol::{
    ActorObject, ActorPayload, BroadcastTextPayload, ConversationPayload, LinePayload,
};

#[derive(Component)]
struct Conversation {
    payload: ConversationPayload,
    created_at: f64,
}

pub(crate) fn visible_for(world: &mut World, viewer: Entity) -> Vec<ConversationPayload> {
    let Some(map) = world.get::<crate::maps::InMap>(viewer).copied() else {
        return Vec::new();
    };
    let now = world.resource::<Time>().elapsed_secs_f64();
    world
        .query::<(&crate::maps::InMap, &Conversation)>()
        .iter(world)
        .filter(|(copy, conversation)| {
            **copy == map && conversation.payload.private_owner == viewer.to_bits()
        })
        .map(|(_, conversation)| {
            let mut payload = conversation.payload.clone();
            payload.elapsed_ms = ((now - conversation.created_at) * 1000.0).max(0.0) as u64;
            payload
        })
        .collect()
}

pub(crate) fn create_private(
    world: &mut World,
    entry: u32,
    creator: Entity,
    position: Position,
    owner: Entity,
) -> Result<Entity, String> {
    let map = *world
        .get::<crate::maps::InMap>(creator)
        .ok_or("conversation creator has no map")?;
    if world.get::<crate::maps::InMap>(owner) != Some(&map) {
        return Err("conversation private owner is outside creator's map copy".into());
    }
    let definition = world
        .get_resource::<Catalog>()
        .ok_or("conversation source catalog unavailable")?
        .0
        .get(&entry)
        .ok_or_else(|| format!("conversation{entry}: source template unavailable"))?
        .clone()?;
    let actors = resolve_actors(world, map, definition.actors)?;
    const ALLOW_WITHOUT_SPAWNED_ACTOR: u8 = 0x4;
    if definition.flags & ALLOW_WITHOUT_SPAWNED_ACTOR == 0
        && definition
            .lines
            .iter()
            .any(|line| !actors.iter().any(|actor| actor.index == line.actor_index))
    {
        return Err(format!("conversation{entry}: source line actor absent"));
    }
    let created_at = world.resource::<Time>().elapsed_secs_f64();
    let entity = world.spawn((position, map)).id();
    let duration_ms = (definition.maximum_end + 10_000).max(0) as u64;
    let payload = ConversationPayload {
        id: entity.to_bits(),
        entry,
        creator: creator.to_bits(),
        private_owner: owner.to_bits(),
        texture_kit_id: definition.texture_kit,
        flags: definition.flags,
        actors,
        lines: definition.lines,
        last_line_end_ms: definition.last_end,
        duration_ms,
        elapsed_ms: 0,
    };
    world.entity_mut(entity).insert(Conversation {
        payload,
        created_at,
    });
    Ok(entity)
}

fn resolve_actors(
    world: &mut World,
    map: crate::maps::InMap,
    actors: Vec<ActorPayload>,
) -> Result<Vec<ActorPayload>, String> {
    actors
        .into_iter()
        .map(|mut actor| {
            if let ActorObject::WorldObject(source_guid) = actor.object {
                // Existing content GUID projection, not a guessed actor/name match.
                const TDB_GUID_OFFSET: u64 = 20_000_000;
                let entity = world
                    .query::<(Entity, &crate::maps::InMap, &crate::zones::CreatureOrigin)>()
                    .iter(world)
                    .find(|(_, copy, origin)| {
                        **copy == map && u64::from(origin.guid) == source_guid + TDB_GUID_OFFSET
                    })
                    .map(|(entity, _, _)| entity)
                    .ok_or_else(|| {
                        format!(
                            "conversation actor{}: source spawn{source_guid} absent in {map:?}",
                            actor.id
                        )
                    })?;
                actor.object = ActorObject::WorldObject(entity.to_bits());
            }
            Ok(actor)
        })
        .collect()
}

fn expire(world: &mut World) {
    let now = world.resource::<Time>().elapsed_secs_f64();
    let expired: Vec<_> = world
        .query::<(Entity, &Conversation)>()
        .iter(world)
        .filter(|(_, conversation)| {
            (now - conversation.created_at) * 1000.0 >= conversation.payload.duration_ms as f64
        })
        .map(|(entity, _)| entity)
        .collect();
    for entity in expired {
        world.despawn(entity);
    }
}

pub(crate) struct ConversationPlugin;
impl Plugin for ConversationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (expire, ingress::detect, wire::publish).chain(),
        );
    }
}
