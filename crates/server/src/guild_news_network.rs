//! Authenticated GuildChannel news; histories read on requests/real producer changes only.
use crate::auth::{Authenticated, InWorld};
use crate::persistence::GameDatabase;
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::protocol::{GuildChannel, GuildNewsRequest, GuildNewsState};
use std::collections::{BTreeSet, HashMap};

#[derive(Resource, Default)]
pub(crate) struct Subscribers {
    entries: HashMap<Entity, (u64, u32, GuildNewsState)>,
    dirty: BTreeSet<u32>,
}

type Requests<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static InWorld,
        &'static mut MessageReceiver<GuildNewsRequest>,
    ),
    With<Authenticated>,
>;

pub(crate) fn receive(
    db: Res<GameDatabase>,
    mut requests: Requests,
    mut senders: Query<&mut MessageSender<GuildNewsState>>,
    mut subscribers: ResMut<Subscribers>,
) {
    for (connection, owner, mut receiver) in &mut requests {
        for request in receiver.receive() {
            let changed = matches!(request, GuildNewsRequest::SetSticky { .. });
            let state = crate::guild_news::apply_request(&db.db, owner.character_id, request);
            if let Some(member) =
                crate::persistence::load_character_guild(&db.db, owner.character_id)
            {
                subscribers.entries.insert(
                    connection,
                    (owner.character_id, member.guild_id, state.clone()),
                );
                if changed && state.error.is_none() {
                    subscribers.dirty.insert(member.guild_id);
                }
            } else {
                subscribers.entries.remove(&connection);
            }
            if let Ok(mut sender) = senders.get_mut(connection) {
                sender.send::<GuildChannel>(state);
            }
        }
    }
}

pub(crate) fn publish(
    db: Res<GameDatabase>,
    mut updates: ResMut<crate::gold::GoldUpdateQueue>,
    mut subscribers: ResMut<Subscribers>,
    mut senders: Query<&mut MessageSender<GuildNewsState>, (With<Authenticated>, With<InWorld>)>,
) {
    let characters: BTreeSet<_> = updates
        .take_news_characters()
        .into_iter()
        .chain(updates.changed_inventories().iter().copied())
        .collect();
    for character in characters {
        if let Some(member) = crate::persistence::load_character_guild(&db.db, character) {
            subscribers.dirty.insert(member.guild_id);
        }
    }
    let dirty = std::mem::take(&mut subscribers.dirty);
    subscribers
        .entries
        .retain(|connection, (character, guild, prior)| {
            let Ok(mut sender) = senders.get_mut(*connection) else {
                return false;
            };
            if !dirty.contains(guild) {
                return true;
            }
            let state = crate::guild_news::query(&db.db, *character);
            let member = crate::persistence::load_character_guild(&db.db, *character);
            if member.is_none_or(|member| member.guild_id != *guild) {
                sender.send::<GuildChannel>(GuildNewsState {
                    entries: vec![],
                    error: Some(shared::protocol::GuildNewsError::NotInGuild),
                    ..state
                });
                return false;
            }
            if state.entries != prior.entries {
                sender.send::<GuildChannel>(state.clone());
                *prior = state;
            }
            true
        });
}
