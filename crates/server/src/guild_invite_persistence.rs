//! Fresh invitation authority on the shared FIFO; notifications follow durable receipts.
use super::*;
use crate::auth::{Authenticated, InWorld};
use crate::persistence_types::PendingGuildInviteData;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Clone, Copy)]
struct Owner {
    connection: Entity,
    player: Entity,
    character: u64,
    account: u64,
}
#[derive(Clone)]
struct Invitation {
    target: Owner,
    pending: PendingGuildInviteData,
}
struct PendingInvite {
    owner: Owner,
    receipt: CommitReceipt<Result<Invitation, String>>,
}
#[derive(Resource, Default)]
struct PendingGuildInvites(VecDeque<PendingInvite>);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<PendingGuildInvites>()
        .add_systems(PostUpdate, publish_invites);
}

fn receive_requests(world: &mut World) -> Vec<(Owner, GuildInviteRequest)> {
    let mut query = world.query::<(
        Entity,
        &Authenticated,
        &InWorld,
        &PlayerLink,
        &mut MessageReceiver<GuildInviteRequest>,
    )>();
    query
        .iter_mut(world)
        .flat_map(|(connection, auth, entered, link, mut receiver)| {
            let owner = Owner {
                connection,
                player: link.0,
                character: entered.character_id,
                account: auth.account_id,
            };
            receiver
                .receive()
                .map(move |request| (owner, request))
                .collect::<Vec<_>>()
        })
        .collect()
}
fn capture_online_players(world: &mut World) -> Vec<Owner> {
    let mut query = world.query::<(Entity, &Authenticated, &InWorld, &PlayerLink)>();
    query
        .iter(world)
        .map(|(connection, auth, entered, link)| Owner {
            connection,
            player: link.0,
            character: entered.character_id,
            account: auth.account_id,
        })
        .collect()
}
pub(super) fn enqueue_requests(world: &mut World) {
    let requests = receive_requests(world);
    if requests.is_empty() {
        return;
    }
    let online = Arc::new(capture_online_players(world));
    for (owner, request) in requests {
        let online = Arc::clone(&online);
        let receipt = world
            .resource::<GameDatabase>()
            .writer
            .enqueue("guild invitation", move |db| {
                Ok(persist_invite(db, owner.character, request, &online))
            });
        world
            .resource_mut::<PendingGuildInvites>()
            .0
            .push_back(PendingInvite { owner, receipt });
    }
}

fn persist_invite(
    db: &redb::Database,
    character: u64,
    request: GuildInviteRequest,
    online: &[Owner],
) -> Result<Invitation, String> {
    let inviter =
        persistence::load_character(db, character).ok_or("Inviter player entity missing")?;
    let guild = persistence::load_character_guild(db, character)
        .ok_or(shared::protocol::GuildRankError::NotInGuild.message())?;
    if !crate::guild::character_has_right(
        db,
        character,
        guild.guild_id,
        shared::protocol::GUILD_RIGHT_INVITE,
    ) {
        return Err(shared::protocol::GuildRankError::Permissions
            .message()
            .into());
    }
    let target = load_eligible_target(db, character, &request.target_character_name, online)?;
    let pending = PendingGuildInviteData {
        guild_id: guild.guild_id,
        guild_name: guild.guild_name,
        inviter_name: inviter.name,
    };
    persistence::set_pending_guild_invite(
        db,
        target.character,
        pending.guild_id,
        &pending.guild_name,
        &pending.inviter_name,
    );
    Ok(Invitation { target, pending })
}

fn load_eligible_target(
    db: &redb::Database,
    inviter: u64,
    name: &str,
    online: &[Owner],
) -> Result<Owner, String> {
    let character = persistence::find_character_id_by_name(db, name)
        .ok_or_else(|| format!("\"{name}\" not found."))?;
    if character == inviter {
        return Err("You can't invite yourself to a guild.".into());
    }
    if persistence::load_character_guild(db, character).is_some() {
        return Err(format!("{name} is already in a guild."));
    }
    if persistence::load_pending_guild_invite(db, character).is_some() {
        return Err(format!("{name} has already been invited to a guild."));
    }
    online
        .iter()
        .find(|owner| owner.character == character)
        .copied()
        .ok_or_else(|| format!("\"{name}\" not found."))
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
fn take_completed_invites(
    world: &mut World,
) -> Vec<(Owner, Result<Result<Invitation, String>, String>)> {
    let mut pending = world.resource_mut::<PendingGuildInvites>();
    let mut completed = Vec::new();
    while let Some(result) = pending.0.front().and_then(|job| job.receipt.result()) {
        let job = pending
            .0
            .pop_front()
            .expect("completed guild invite receipt");
        completed.push((job.owner, result));
    }
    completed
}
fn publish_invites(world: &mut World) {
    for (owner, result) in take_completed_invites(world) {
        let result = result.and_then(|result| result);
        if let Ok(invitation) = &result {
            notify_target(world, invitation);
        }
        if !owns_session(world, owner) {
            continue;
        }
        if let Some(mut sender) =
            world.get_mut::<MessageSender<GuildInviteResponse>>(owner.connection)
        {
            sender.send::<AuthChannel>(GuildInviteResponse {
                success: result.is_ok(),
                error: result.err(),
            });
        }
    }
}
fn notify_target(world: &mut World, invitation: &Invitation) {
    let current = persistence::load_pending_guild_invite(
        &world.resource::<GameDatabase>().db,
        invitation.target.character,
    );
    if current.as_ref() != Some(&invitation.pending) {
        return;
    }
    let original_current = owns_session(world, invitation.target);
    let recipients = capture_online_players(world);
    for target in recipients {
        let same_character = target.character == invitation.target.character
            && target.account == invitation.target.account;
        let rebound_original =
            target.connection == invitation.target.connection && !original_current;
        if !same_character || rebound_original {
            continue;
        }
        if let Some(mut sender) =
            world.get_mut::<MessageSender<shared::protocol::GuildInvitation>>(target.connection)
        {
            sender.send::<AuthChannel>(shared::protocol::GuildInvitation {
                guild_id: invitation.pending.guild_id,
                guild_name: invitation.pending.guild_name.clone(),
                inviter_name: invitation.pending.inviter_name.clone(),
            });
        }
    }
}
