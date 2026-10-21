//! Reliable private-owner snapshots; only membership changes send, not elapsed ticks.
use crate::maps::InMap;
use crate::networking::PlayerLink;
use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use shared::protocol::{ConversationUpdate, EncounterChannel};

#[derive(Component, PartialEq, Eq)]
struct Published {
    player: Entity,
    map: InMap,
    conversations: Vec<u64>,
}

pub(super) fn publish(world: &mut World) {
    let connections: Vec<_> = world
        .query::<(Entity, &PlayerLink)>()
        .iter(world)
        .map(|(connection, link)| (connection, link.0))
        .collect();
    for (connection, player) in connections {
        let Some(map) = world.get::<InMap>(player).copied() else {
            continue;
        };
        let mut conversations = super::visible_for(world, player);
        conversations.sort_unstable_by_key(|conversation| conversation.id);
        let state = Published {
            player,
            map,
            conversations: conversations
                .iter()
                .map(|conversation| conversation.id)
                .collect(),
        };
        if world.get::<Published>(connection) == Some(&state) {
            continue;
        }
        let Some(mut sender) = world.get_mut::<MessageSender<ConversationUpdate>>(connection)
        else {
            continue;
        };
        sender.send::<EncounterChannel>(ConversationUpdate {
            map_id: map.map_id,
            instance_id: map.instance_id,
            conversations,
        });
        world.entity_mut(connection).insert(state);
    }
}
