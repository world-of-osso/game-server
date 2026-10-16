//! Fresh guild profile/note writes; current recipient visibility is resolved after receipt.
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub(super) struct Owner {
    pub connection: Entity,
    pub player: Entity,
    pub account: u64,
    pub character: u64,
}

pub(super) enum Change {
    Motd(String),
    Info(String),
    OfficerNote(SetGuildOfficerNote),
}

impl Change {
    fn required_right(&self) -> u32 {
        match self {
            Self::Motd(_) => shared::protocol::GUILD_RIGHT_EDIT_MOTD,
            Self::Info(_) => shared::protocol::GUILD_RIGHT_EDIT_INFO,
            Self::OfficerNote(_) => shared::protocol::GUILD_RIGHT_EDIT_OFFICER_NOTE,
        }
    }
}

#[derive(Clone)]
enum Publication {
    Changed {
        guild: u32,
        message: String,
    },
    Refused {
        guild: Option<GuildMembership>,
        error: String,
    },
}

struct PendingWrite {
    owner: Owner,
    receipt: CommitReceipt<Publication>,
}

#[derive(Resource, Default)]
struct PendingGuildProfileWrites(VecDeque<PendingWrite>);

pub(super) fn install(app: &mut App) {
    app.init_resource::<PendingGuildProfileWrites>()
        .add_systems(PostUpdate, publish_writes);
}

pub(super) fn enqueue(world: &mut World, owner: Owner, change: Change) {
    let receipt = world
        .resource::<GameDatabase>()
        .writer
        .enqueue("guild profile request", move |db| {
            Ok(persist_request(db, owner.character, change))
        });
    world
        .resource_mut::<PendingGuildProfileWrites>()
        .0
        .push_back(PendingWrite { owner, receipt });
}

fn persist_request(db: &redb::Database, character: u64, change: Change) -> Publication {
    let Some(guild) = crate::persistence::load_character_guild(db, character) else {
        return Publication::Refused {
            guild: None,
            error: "not in a guild".into(),
        };
    };
    if !character_has_right(db, character, guild.guild_id, change.required_right()) {
        return Publication::Refused {
            guild: None,
            error: shared::protocol::GuildRankError::Permissions
                .message()
                .into(),
        };
    }
    persist_authorized_change(db, guild, change)
}

fn persist_authorized_change(
    db: &redb::Database,
    guild: GuildMembership,
    change: Change,
) -> Publication {
    let message = match change {
        Change::Motd(text) => persist_text(
            db,
            guild.guild_id,
            |profile| profile.motd = text,
            "guild motd updated",
        ),
        Change::Info(text) => persist_text(
            db,
            guild.guild_id,
            |profile| profile.info_text = text,
            "guild info updated",
        ),
        Change::OfficerNote(request) => match update_officer_note(db, &guild, request) {
            Ok(message) => message,
            Err(error) => {
                return Publication::Refused {
                    guild: Some(guild),
                    error,
                };
            }
        },
    };
    Publication::Changed {
        guild: guild.guild_id,
        message,
    }
}

fn persist_text(
    db: &redb::Database,
    guild: u32,
    apply: impl FnOnce(&mut GuildProfileData),
    message: &'static str,
) -> String {
    let mut profile = load_guild_profile(db, guild);
    apply(&mut profile);
    save_guild_profile(db, guild, &profile);
    message.into()
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

fn publish_writes(world: &mut World) {
    let completed = {
        let mut pending = world.resource_mut::<PendingGuildProfileWrites>();
        let mut completed = Vec::new();
        while let Some(result) = pending.0.front().and_then(|job| job.receipt.result()) {
            let job = pending
                .0
                .pop_front()
                .expect("completed guild profile receipt");
            completed.push((job.owner, result));
        }
        completed
    };
    for (owner, result) in completed {
        match result {
            Ok(Publication::Changed { guild, message }) => {
                publish_changed(world, owner, guild, message)
            }
            Ok(Publication::Refused { guild, error }) => publish_error(world, owner, guild, error),
            Err(error) => publish_error(world, owner, None, error),
        }
    }
}

fn publish_error(world: &mut World, owner: Owner, guild: Option<GuildMembership>, error: String) {
    if !owns_session(world, owner) {
        return;
    }
    let snapshot = guild.map(|guild| {
        let db = Arc::clone(&world.resource::<GameDatabase>().db);
        let mut state = bevy::ecs::system::SystemState::<OnlinePlayers>::new(world);
        let online = state.get(world).expect("read guild online players");
        visible_guild_snapshot(&db, &guild, &online, owner.character)
    });
    send_update(
        world,
        owner.connection,
        GuildStateUpdate {
            guild: snapshot,
            message: None,
            error: Some(error),
        },
    );
}

fn publish_changed(world: &mut World, owner: Owner, guild_id: u32, message: String) {
    let recipients = find_current_recipients(world, owner);
    let updates = load_visible_updates(world, guild_id, &message, recipients);
    for (connection, update) in updates {
        send_update(world, connection, update);
    }
}

fn find_current_recipients(world: &mut World, owner: Owner) -> Vec<(Entity, u64)> {
    let source_current = owns_session(world, owner);
    let mut connections =
        world.query_filtered::<(Entity, &InWorld, &PlayerLink), With<Authenticated>>();
    connections
        .iter(world)
        .filter(|(connection, _, _)| *connection != owner.connection || source_current)
        .map(|(connection, entered, _)| (connection, entered.character_id))
        .collect()
}

fn load_visible_updates(
    world: &mut World,
    guild_id: u32,
    message: &str,
    recipients: Vec<(Entity, u64)>,
) -> Vec<(Entity, GuildStateUpdate)> {
    let db = Arc::clone(&world.resource::<GameDatabase>().db);
    let mut state = bevy::ecs::system::SystemState::<OnlinePlayers>::new(world);
    let online = state.get(world).expect("read guild online players");
    recipients
        .into_iter()
        .filter_map(|(connection, character)| {
            let guild = crate::persistence::load_character_guild(&db, character)?;
            if guild.guild_id != guild_id {
                return None;
            }
            let snapshot = visible_guild_snapshot(&db, &guild, &online, character);
            Some((
                connection,
                GuildStateUpdate {
                    guild: Some(snapshot),
                    message: Some(message.into()),
                    error: None,
                },
            ))
        })
        .collect()
}

fn send_update(world: &mut World, connection: Entity, update: GuildStateUpdate) {
    if let Some(mut sender) = world.get_mut::<MessageSender<GuildStateUpdate>>(connection) {
        sender.send::<GuildChannel>(update);
    }
}
