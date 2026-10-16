//! Guild rank mutations and dependent queries run at their shared FIFO position.
use super::*;
use crate::networking::PlayerLink;
use crate::persistence_writer::CommitReceipt;
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Copy)]
pub(super) struct Owner {
    pub connection: Entity,
    pub player: Entity,
    pub account: u64,
    pub character: u64,
}

#[derive(Clone)]
enum Publication {
    Owner(GuildRanksState),
    EventLog(GuildEventLog),
    Changed {
        guild: u32,
        previous_members: BTreeSet<u64>,
    },
}

struct PendingWrite {
    owner: Owner,
    receipt: CommitReceipt<Publication>,
}

#[derive(Resource, Default)]
pub(crate) struct PendingGuildRankWrites(VecDeque<PendingWrite>);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<PendingGuildRankWrites>()
        .add_systems(PostUpdate, publish_writes);
}

pub(super) fn enqueue(
    db: &GameDatabase,
    pending: &mut PendingGuildRankWrites,
    owner: Owner,
    request: GuildRankRequest,
) {
    let receipt = db.writer.enqueue("guild rank request", move |db| {
        Ok(apply_request(db, owner.character, request))
    });
    pending.0.push_back(PendingWrite { owner, receipt });
}

fn apply_request(db: &redb::Database, character: u64, request: GuildRankRequest) -> Publication {
    if matches!(request, GuildRankRequest::QueryEventLog) {
        return Publication::EventLog(crate::guild_event_log::query(db, character));
    }
    let Some(membership) = persistence::load_character_guild(db, character) else {
        return Publication::Owner(snapshot(db, character, Some(GuildRankError::NotInGuild)));
    };
    if matches!(request, GuildRankRequest::Query) {
        return Publication::Owner(snapshot(db, character, None));
    }
    let previous_members = list_guild_members(db, membership.guild_id)
        .into_iter()
        .map(|member| member.character_id)
        .collect();
    match apply_rank_request(db, membership.guild_id, character, request) {
        Ok(()) => Publication::Changed {
            guild: membership.guild_id,
            previous_members,
        },
        Err(error) => Publication::Owner(snapshot(db, character, Some(error))),
    }
}

fn owns_session(world: &World, owner: Owner) -> bool {
    let authenticated = world
        .get::<Authenticated>(owner.connection)
        .is_some_and(|auth| auth.account_id == owner.account);
    let entered = world
        .get::<InWorld>(owner.connection)
        .is_some_and(|session| session.character_id == owner.character);
    let linked = world
        .get::<PlayerLink>(owner.connection)
        .is_some_and(|link| link.0 == owner.player);
    authenticated && entered && linked
}

fn publish_writes(world: &mut World) {
    let completed = {
        let mut pending = world.resource_mut::<PendingGuildRankWrites>();
        let mut completed = Vec::new();
        while let Some(result) = pending.0.front().and_then(|job| job.receipt.result()) {
            let job = pending.0.pop_front().expect("completed guild rank receipt");
            completed.push((job.owner, result));
        }
        completed
    };
    for (owner, result) in completed {
        match result {
            Ok(Publication::Owner(state)) => {
                if owns_session(world, owner) {
                    send_state(world, owner.connection, state);
                }
            }
            Ok(Publication::EventLog(log)) => {
                if owns_session(world, owner)
                    && let Some(mut sender) =
                        world.get_mut::<MessageSender<GuildEventLog>>(owner.connection)
                {
                    sender.send::<GuildChannel>(log);
                }
            }
            Ok(Publication::Changed {
                guild,
                previous_members,
            }) => {
                world.resource_mut::<ChangedGuilds>().0.push(guild);
                publish_current_members(world, owner, &previous_members);
            }
            Err(error) => send_storage_error(world, owner, error),
        }
    }
}

fn publish_current_members(world: &mut World, owner: Owner, previous_members: &BTreeSet<u64>) {
    let source_current = owns_session(world, owner);
    let mut connections =
        world.query_filtered::<(Entity, &InWorld, &PlayerLink), With<Authenticated>>();
    let recipients: Vec<_> = connections
        .iter(world)
        .filter(|(_, entered, _)| previous_members.contains(&entered.character_id))
        .filter(|(connection, _, _)| *connection != owner.connection || source_current)
        .map(|(connection, entered, link)| (connection, entered.character_id, link.0))
        .collect();
    for (connection, character, player) in recipients {
        let db = &world.resource::<GameDatabase>().db;
        let departed = persistence::load_character_guild(db, character).is_none();
        let state = snapshot(db, character, None);
        if departed {
            world
                .entity_mut(player)
                .remove::<shared::components::GuildMembership>();
        }
        send_state(world, connection, state);
    }
}

fn send_state(world: &mut World, connection: Entity, state: GuildRanksState) {
    if let Some(mut sender) = world.get_mut::<MessageSender<GuildRanksState>>(connection) {
        sender.send::<GuildChannel>(state);
    }
}

fn send_storage_error(world: &mut World, owner: Owner, error: String) {
    if !owns_session(world, owner) {
        return;
    }
    // Existing string-valued guild errors preserve the actual storage failure; do not
    // mislabel it as a permission refusal or acknowledge an uncommitted rank mutation.
    if let Some(mut sender) = world.get_mut::<MessageSender<GuildStateUpdate>>(owner.connection) {
        sender.send::<GuildChannel>(GuildStateUpdate {
            guild: None,
            message: None,
            error: Some(error),
        });
    }
}
