//! Guild news requests and producer notifications read at their position on the durable FIFO.
use crate::auth::{Authenticated, InWorld};
use crate::persistence::GameDatabase;
use crate::persistence_writer::CommitReceipt;
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::protocol::{GuildChannel, GuildNewsError, GuildNewsRequest, GuildNewsState};
use std::collections::{BTreeSet, HashMap, VecDeque};

#[derive(Resource, Default)]
pub(crate) struct Subscribers {
    entries: HashMap<Entity, (u64, u32, GuildNewsState)>,
    dirty: BTreeSet<u32>,
    pending: VecDeque<PendingNews>,
}

enum Delivery {
    Requested { changes_history: bool },
    Publication { guild: u32 },
}

struct PendingNews {
    connection: Entity,
    character: u64,
    delivery: Delivery,
    receipt: CommitReceipt<(Option<u32>, GuildNewsState)>,
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
type Senders<'w, 's> = Query<
    'w,
    's,
    (&'static InWorld, &'static mut MessageSender<GuildNewsState>),
    With<Authenticated>,
>;

fn enqueue_news(
    db: &GameDatabase,
    subscribers: &mut Subscribers,
    connection: Entity,
    character: u64,
    request: GuildNewsRequest,
    delivery: Delivery,
) {
    let receipt = db.writer.enqueue_result("ordered guild news", move |db| {
        let state = crate::guild_news::apply_request(db, character, request);
        let guild =
            crate::persistence::load_character_guild(db, character).map(|member| member.guild_id);
        Ok((guild, state))
    });
    subscribers.pending.push_back(PendingNews {
        connection,
        character,
        delivery,
        receipt,
    });
}

pub(crate) fn receive(
    db: Res<GameDatabase>,
    mut requests: Requests,
    mut subscribers: ResMut<Subscribers>,
) {
    for (connection, owner, mut receiver) in &mut requests {
        for request in receiver.receive() {
            let changes_history = matches!(request, GuildNewsRequest::SetSticky { .. });
            enqueue_news(
                &db,
                &mut subscribers,
                connection,
                owner.character_id,
                request,
                Delivery::Requested { changes_history },
            );
        }
    }
}

fn update_subscriber(
    subscribers: &mut Subscribers,
    pending: &PendingNews,
    guild: Option<u32>,
    state: &GuildNewsState,
) -> bool {
    let requested = matches!(pending.delivery, Delivery::Requested { .. });
    let changed = subscribers
        .entries
        .get(&pending.connection)
        .is_none_or(|(_, _, prior)| prior.entries != state.entries);
    if let Some(guild) = guild {
        subscribers.entries.insert(
            pending.connection,
            (pending.character, guild, state.clone()),
        );
        if matches!(
            pending.delivery,
            Delivery::Requested {
                changes_history: true
            }
        ) && state.error.is_none()
        {
            subscribers.dirty.insert(guild);
        }
    } else {
        subscribers.entries.remove(&pending.connection);
    }
    requested || changed || guild.is_none()
}

fn publish_reply(
    subscribers: &mut Subscribers,
    senders: &mut Senders,
    pending: PendingNews,
    (mut guild, mut state): (Option<u32>, GuildNewsState),
) {
    let Ok((owner, mut sender)) = senders.get_mut(pending.connection) else {
        subscribers.entries.remove(&pending.connection);
        return;
    };
    if owner.character_id != pending.character {
        subscribers.entries.remove(&pending.connection);
        return;
    }
    if let Delivery::Publication { guild: expected } = pending.delivery
        && guild != Some(expected)
    {
        guild = None;
        state.entries.clear();
        state.error = Some(GuildNewsError::NotInGuild);
    }
    if update_subscriber(subscribers, &pending, guild, &state) {
        sender.send::<GuildChannel>(state);
    }
}

fn publish_completed(subscribers: &mut Subscribers, senders: &mut Senders) {
    while let Some(front) = subscribers.pending.front() {
        let Some(result) = front.receipt.result() else {
            break;
        };
        let pending = subscribers
            .pending
            .pop_front()
            .expect("completed FIFO guild news");
        match result {
            Ok(reply) => publish_reply(subscribers, senders, pending, reply),
            Err(error) => error!(character = pending.character, %error,
                "Guild news job failed; no success reply sent"),
        }
    }
}

pub(crate) fn publish(
    db: Res<GameDatabase>,
    mut updates: ResMut<crate::gold::GoldUpdateQueue>,
    mut subscribers: ResMut<Subscribers>,
    mut senders: Senders,
) {
    subscribers.entries.retain(|connection, (character, _, _)| {
        senders
            .get_mut(*connection)
            .is_ok_and(|(owner, _)| owner.character_id == *character)
    });
    publish_completed(&mut subscribers, &mut senders);
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
    let recipients: Vec<_> = subscribers
        .entries
        .iter()
        .filter(|(_, (_, guild, _))| dirty.contains(guild))
        .map(|(connection, (character, guild, _))| (*connection, *character, *guild))
        .collect();
    for (connection, character, guild) in recipients {
        enqueue_news(
            &db,
            &mut subscribers,
            connection,
            character,
            GuildNewsRequest::Query,
            Delivery::Publication { guild },
        );
    }
}
