//! Owned guild acceptance on the shared FIFO; live membership follows durable receipts.
use super::*;
use crate::auth::{Authenticated, InWorld};
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

#[derive(Clone, Copy)]
struct Owner {
    connection: Entity,
    player: Entity,
    account: u64,
    character: u64,
}

struct PendingAccept {
    owner: Owner,
    receipt: CommitReceipt<GuildAcceptInviteResponse>,
}

#[derive(Resource, Default)]
struct PendingGuildAccepts(VecDeque<PendingAccept>);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<PendingGuildAccepts>()
        .add_systems(PostUpdate, publish_accepts);
}

pub(super) fn enqueue_requests(world: &mut World) {
    let mut query = world.query::<(
        Entity,
        &Authenticated,
        &InWorld,
        &PlayerLink,
        &mut MessageReceiver<GuildAcceptInviteRequest>,
    )>();
    let owners: Vec<_> = query
        .iter_mut(world)
        .flat_map(|(connection, auth, entered, link, mut receiver)| {
            let owner = Owner {
                connection,
                player: link.0,
                account: auth.account_id,
                character: entered.character_id,
            };
            receiver.receive().map(move |_| owner).collect::<Vec<_>>()
        })
        .collect();
    for owner in owners {
        let receipt = world
            .resource::<GameDatabase>()
            .writer
            .enqueue("guild invite acceptance", move |db| {
                Ok(persist_accept(db, owner.character))
            });
        world
            .resource_mut::<PendingGuildAccepts>()
            .0
            .push_back(PendingAccept { owner, receipt });
    }
}

fn persist_accept(db: &redb::Database, character: u64) -> GuildAcceptInviteResponse {
    if persistence::load_character_guild(db, character).is_some() {
        return guild_accept_error("You are already in a guild.");
    }
    match persistence::accept_pending_guild_invite(db, character) {
        Ok(invite) => GuildAcceptInviteResponse {
            success: true,
            guild_id: Some(invite.guild_id),
            guild_name: Some(invite.guild_name),
            error: None,
        },
        Err(error) => guild_accept_error(error),
    }
}

fn owns_session(world: &World, owner: Owner) -> bool {
    let authenticated = world
        .get::<Authenticated>(owner.connection)
        .is_some_and(|auth| auth.account_id == owner.account);
    let entered = world
        .get::<InWorld>(owner.connection)
        .is_some_and(|entered| entered.character_id == owner.character);
    let linked = world
        .get::<PlayerLink>(owner.connection)
        .is_some_and(|link| link.0 == owner.player);
    authenticated && entered && linked
}

fn publish_accepts(world: &mut World) {
    let completed = {
        let mut pending = world.resource_mut::<PendingGuildAccepts>();
        let mut completed = Vec::new();
        while let Some(result) = pending.0.front().and_then(|job| job.receipt.result()) {
            let job = pending
                .0
                .pop_front()
                .expect("completed guild accept receipt");
            completed.push((job.owner, result));
        }
        completed
    };
    for (owner, result) in completed {
        let response = result.unwrap_or_else(guild_accept_error);
        let current = owns_session(world, owner);
        if let Some(guild) = response.guild_id {
            publish_join(world, owner, guild, current);
        }
        if current
            && let Some(mut sender) =
                world.get_mut::<MessageSender<GuildAcceptInviteResponse>>(owner.connection)
        {
            sender.send::<AuthChannel>(response);
        }
    }
}

fn publish_join(world: &mut World, owner: Owner, guild_id: u32, source_current: bool) {
    world
        .resource_mut::<crate::guild_bank::ChangedGuilds>()
        .0
        .push(guild_id);
    if source_current {
        publish_membership(world, owner, guild_id);
    }
    let mut query = world.query_filtered::<(Entity, &InWorld), With<Authenticated>>();
    let recipients: Vec<_> = query
        .iter(world)
        .filter(|(connection, _)| *connection != owner.connection || source_current)
        .map(|(connection, entered)| (connection, entered.character_id))
        .collect();
    for (connection, character) in recipients {
        let db = &world.resource::<GameDatabase>().db;
        if !persistence::load_character_guild(db, character)
            .is_some_and(|guild| guild.guild_id == guild_id)
        {
            continue;
        }
        let snapshot = crate::guild::guild_rank_snapshot(db, character, None);
        if let Some(mut sender) =
            world.get_mut::<MessageSender<shared::protocol::GuildRanksState>>(connection)
        {
            sender.send::<shared::protocol::GuildChannel>(snapshot);
        }
    }
}

fn publish_membership(world: &mut World, owner: Owner, guild_id: u32) {
    let Some(membership) =
        persistence::load_character_guild(&world.resource::<GameDatabase>().db, owner.character)
    else {
        return;
    };
    if membership.guild_id != guild_id
        || world.get::<GuildMembership>(owner.player) == Some(&membership)
    {
        return;
    }
    if let Ok(mut player) = world.get_entity_mut(owner.player) {
        player.insert(membership);
    }
}
