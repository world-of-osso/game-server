use std::collections::BTreeMap;

use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::GuildMembership;
use shared::protocol::{
    GuildChannel, GuildMemberSnapshot, GuildSnapshot, GuildStateUpdate, QueryGuild, SetGuildInfo,
    SetGuildMotd, SetGuildOfficerNote,
};

use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::GameDatabase;
use crate::persistence_character::{
    GuildMemberRecord, find_character_id_by_name, list_guild_members, load_guild_member_note,
    load_guild_profile, save_guild_member_note, save_guild_profile,
};
use crate::persistence_types::{GuildMemberNoteData, GuildProfileData};

type NewConnections<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static PlayerLink, &'static InWorld),
    (With<Authenticated>, With<InWorld>, Added<InWorld>),
>;
type ActiveConnections<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static PlayerLink, &'static InWorld),
    (With<Authenticated>, With<InWorld>),
>;
type GuildMemberships<'w, 's> = Query<'w, 's, &'static GuildMembership>;
#[path = "guild_online_zone.rs"]
mod online_zone;
use online_zone::OnlinePlayers;
type GuildSenders<'w, 's> = Query<'w, 's, &'static mut MessageSender<GuildStateUpdate>>;
type GuildQueryReceivers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PlayerLink,
        &'static mut MessageReceiver<QueryGuild>,
        &'static InWorld,
    ),
    (With<Authenticated>, With<InWorld>),
>;
type GuildProfileReceivers<'w, 's, Message> = Query<
    'w,
    's,
    (
        Entity,
        &'static PlayerLink,
        &'static mut MessageReceiver<Message>,
        &'static InWorld,
    ),
    (With<Authenticated>, With<InWorld>),
>;
type GuildMotdReceivers<'w, 's> = GuildProfileReceivers<'w, 's, SetGuildMotd>;
type GuildInfoReceivers<'w, 's> = GuildProfileReceivers<'w, 's, SetGuildInfo>;
type GuildOfficerNoteReceivers<'w, 's> = GuildProfileReceivers<'w, 's, SetGuildOfficerNote>;

#[derive(Clone)]
struct GuildUpdateMeta {
    message: Option<String>,
    error: Option<String>,
}

trait GuildProfileTextRequest {
    fn required_right() -> u32;
    fn trimmed_text(&self) -> String;
}

impl GuildProfileTextRequest for SetGuildMotd {
    fn required_right() -> u32 {
        shared::protocol::GUILD_RIGHT_EDIT_MOTD
    }
    fn trimmed_text(&self) -> String {
        self.text.trim().to_string()
    }
}

impl GuildProfileTextRequest for SetGuildInfo {
    fn required_right() -> u32 {
        shared::protocol::GUILD_RIGHT_EDIT_INFO
    }
    fn trimmed_text(&self) -> String {
        self.text.trim().to_string()
    }
}

#[path = "guild_charters.rs"]
pub(crate) mod charters;
#[path = "guild_invites.rs"]
mod guild_invites;
#[path = "guild_ranks.rs"]
mod guild_ranks;
pub(crate) use guild_invites::load_pending_invitation;
#[cfg(test)]
use guild_ranks::apply_rank_request;
pub(crate) use guild_ranks::{character_has_right, snapshot as guild_rank_snapshot};

pub struct GuildPlugin;

impl Plugin for GuildPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, online_zone::load_startup)
            .init_resource::<crate::gold::GoldUpdateQueue>()
            .init_resource::<crate::guild_news_network::Subscribers>()
            .add_systems(
                PostUpdate,
                crate::guild_news_network::publish.before(crate::inventory::InventoryDeltas),
            )
            .init_resource::<crate::guild_bank::ChangedGuilds>()
            .add_systems(
                Update,
                (
                    guild_invites::send_pending_invites,
                    guild_invites::decline_invites,
                    charters::process_requests,
                    send_initial_guild_state,
                    process_query_requests,
                    process_profile_text_updates,
                    process_set_officer_note_requests,
                    guild_ranks::process_rank_requests,
                    crate::guild_news_network::receive,
                )
                    .chain(),
            );
    }
}

fn send_initial_guild_state(
    db: Res<GameDatabase>,
    connections: NewConnections,
    guilds: GuildMemberships,
    mut online_players: OnlinePlayers,
    mut senders: GuildSenders,
) {
    for (conn_entity, link, world) in &connections {
        let Ok(guild) = guilds.get(link.0) else {
            continue;
        };
        send_guild_update(
            conn_entity,
            Some(visible_guild_snapshot(
                &db.db,
                guild,
                &mut online_players,
                world.character_id,
            )),
            Some("guild loaded".into()),
            None,
            &mut senders,
        );
    }
}

fn process_query_requests(
    db: Res<GameDatabase>,
    mut receivers: GuildQueryReceivers,
    guilds: GuildMemberships,
    mut online_players: OnlinePlayers,
    mut senders: GuildSenders,
) {
    for (conn_entity, link, mut receiver, world) in &mut receivers {
        let mut saw_request = false;
        for _ in receiver.receive() {
            saw_request = true;
        }
        if !saw_request {
            continue;
        }
        let Ok(guild) = guilds.get(link.0) else {
            send_guild_update(
                conn_entity,
                None,
                None,
                Some("not in a guild".into()),
                &mut senders,
            );
            continue;
        };
        send_guild_update(
            conn_entity,
            Some(visible_guild_snapshot(
                &db.db,
                guild,
                &mut online_players,
                world.character_id,
            )),
            Some("guild refreshed".into()),
            None,
            &mut senders,
        );
    }
}

fn process_profile_text_updates(
    db: Res<GameDatabase>,
    mut motd_receivers: GuildMotdReceivers,
    mut info_receivers: GuildInfoReceivers,
    connections: ActiveConnections,
    guilds: GuildMemberships,
    mut online_players: OnlinePlayers,
    mut senders: GuildSenders,
) {
    process_profile_text_requests(
        &db.db,
        &mut motd_receivers,
        &connections,
        &guilds,
        &mut online_players,
        &mut senders,
        |profile, text| {
            profile.motd = text;
            "guild motd updated"
        },
    );
    process_profile_text_requests(
        &db.db,
        &mut info_receivers,
        &connections,
        &guilds,
        &mut online_players,
        &mut senders,
        |profile, text| {
            profile.info_text = text;
            "guild info updated"
        },
    );
}

fn process_profile_text_requests<Request, Apply>(
    db: &redb::Database,
    receivers: &mut GuildProfileReceivers<Request>,
    connections: &ActiveConnections,
    guilds: &GuildMemberships,
    online_players: &mut OnlinePlayers,
    senders: &mut GuildSenders,
    apply: Apply,
) where
    Request: GuildProfileTextRequest + Send + Sync + 'static,
    Apply: Fn(&mut GuildProfileData, String) -> &'static str,
{
    for (conn_entity, link, mut receiver, world) in receivers.iter_mut() {
        let requests: Vec<_> = receiver
            .receive()
            .map(|request: Request| request.trimmed_text())
            .collect();
        if requests.is_empty() {
            continue;
        }
        let Ok(guild) = guilds.get(link.0) else {
            send_not_in_guild_error(conn_entity, senders);
            continue;
        };
        for text in requests {
            if !character_has_right(
                db,
                world.character_id,
                guild.guild_id,
                Request::required_right(),
            ) {
                send_guild_update(
                    conn_entity,
                    None,
                    None,
                    Some(
                        shared::protocol::GuildRankError::Permissions
                            .message()
                            .into(),
                    ),
                    senders,
                );
                continue;
            }
            let mut profile = load_guild_profile(db, guild.guild_id);
            let success_message = apply(&mut profile, text);
            save_guild_profile(db, guild.guild_id, &profile);
            broadcast_guild_update(
                db,
                connections,
                guilds,
                online_players,
                guild,
                GuildUpdateMeta {
                    message: Some(success_message.into()),
                    error: None,
                },
                senders,
            );
        }
    }
}

fn process_set_officer_note_requests(
    db: Res<GameDatabase>,
    mut receivers: GuildOfficerNoteReceivers,
    connections: ActiveConnections,
    guilds: GuildMemberships,
    mut online_players: OnlinePlayers,
    mut senders: GuildSenders,
) {
    for (conn_entity, link, mut receiver, world) in &mut receivers {
        let requests: Vec<_> = receiver.receive().collect();
        if requests.is_empty() {
            continue;
        }
        let Ok(guild) = guilds.get(link.0) else {
            send_not_in_guild_error(conn_entity, &mut senders);
            continue;
        };
        for request in requests {
            if !character_has_right(
                &db.db,
                world.character_id,
                guild.guild_id,
                shared::protocol::GUILD_RIGHT_EDIT_OFFICER_NOTE,
            ) {
                send_guild_update(
                    conn_entity,
                    None,
                    None,
                    Some(
                        shared::protocol::GuildRankError::Permissions
                            .message()
                            .into(),
                    ),
                    &mut senders,
                );
                continue;
            }
            match update_officer_note(&db.db, guild, request) {
                Ok(message) => broadcast_officer_note_update(
                    &db.db,
                    &connections,
                    &guilds,
                    &mut online_players,
                    guild,
                    message,
                    &mut senders,
                ),
                Err(error) => send_officer_note_error(
                    &db.db,
                    &mut online_players,
                    guild,
                    conn_entity,
                    world.character_id,
                    error,
                    &mut senders,
                ),
            }
        }
    }
}

fn send_not_in_guild_error(conn_entity: Entity, senders: &mut GuildSenders) {
    send_guild_update(
        conn_entity,
        None,
        None,
        Some("not in a guild".into()),
        senders,
    );
}

fn broadcast_officer_note_update(
    db: &redb::Database,
    connections: &ActiveConnections,
    guilds: &GuildMemberships,
    online_players: &mut OnlinePlayers,
    guild: &GuildMembership,
    message: String,
    senders: &mut GuildSenders,
) {
    broadcast_guild_update(
        db,
        connections,
        guilds,
        online_players,
        guild,
        GuildUpdateMeta {
            message: Some(message),
            error: None,
        },
        senders,
    );
}

fn send_officer_note_error(
    db: &redb::Database,
    online_players: &mut OnlinePlayers,
    guild: &GuildMembership,
    conn_entity: Entity,
    viewer: u64,
    error: String,
    senders: &mut GuildSenders,
) {
    let snapshot = visible_guild_snapshot(db, guild, online_players, viewer);
    send_guild_update(conn_entity, Some(snapshot), None, Some(error), senders);
}

fn update_officer_note(
    db: &redb::Database,
    guild: &GuildMembership,
    request: SetGuildOfficerNote,
) -> Result<String, String> {
    let Some(character_id) = find_character_id_by_name(db, &request.character_name) else {
        return Err(format!(
            "guild member not found: {}",
            request.character_name
        ));
    };
    let members = list_guild_members(db, guild.guild_id);
    if !members
        .iter()
        .any(|member| member.character_id == character_id)
    {
        return Err(format!(
            "guild member not found in {}: {}",
            guild.guild_name, request.character_name
        ));
    }
    if !shared::protocol::valid_guild_member_note(&request.note) {
        return Err(shared::protocol::GuildRankError::NoteTooLong
            .message()
            .into());
    }
    save_guild_member_note(
        db,
        character_id,
        &GuildMemberNoteData {
            officer_note: request.note,
        },
    );
    Ok(format!(
        "guild officer note updated: {}",
        request.character_name
    ))
}

fn broadcast_guild_update(
    db: &redb::Database,
    connections: &ActiveConnections,
    guilds: &GuildMemberships,
    online_players: &mut OnlinePlayers,
    guild: &GuildMembership,
    meta: GuildUpdateMeta,
    senders: &mut GuildSenders,
) {
    for (conn_entity, link, world) in connections.iter() {
        let Ok(target_guild) = guilds.get(link.0) else {
            continue;
        };
        if target_guild.guild_id != guild.guild_id {
            continue;
        }
        send_guild_update(
            conn_entity,
            Some(visible_guild_snapshot(
                db,
                guild,
                online_players,
                world.character_id,
            )),
            meta.message.clone(),
            meta.error.clone(),
            senders,
        );
    }
}

/// Refresh every online member's replicated design and full authoritative guild snapshot.
pub(crate) fn broadcast_registered_tabard(
    world: &mut World,
    guild_id: u32,
    design: shared::guild_creation::GuildTabardDesign,
) {
    let mut query = world.query::<(Entity, &GuildMembership)>();
    let members: Vec<_> = query
        .iter(world)
        .filter(|(_, guild)| guild.guild_id == guild_id)
        .map(|(entity, guild)| (entity, guild.clone()))
        .collect();
    for (entity, _) in &members {
        world.entity_mut(*entity).insert(design);
    }
    let Some((_, guild)) = members.first() else {
        return;
    };
    let mut state: bevy::ecs::system::SystemState<(
        Res<GameDatabase>,
        ActiveConnections,
        GuildMemberships,
        OnlinePlayers,
        GuildSenders,
    )> = bevy::ecs::system::SystemState::new(world);
    let (db, connections, guilds, mut players, mut senders) = state
        .get_mut(world)
        .expect("load guild tabard broadcast system parameters");
    broadcast_guild_update(
        &db.db,
        &connections,
        &guilds,
        &mut players,
        guild,
        GuildUpdateMeta {
            message: Some("Guild Emblem saved.".into()),
            error: None,
        },
        &mut senders,
    );
}

fn visible_guild_snapshot(
    db: &redb::Database,
    guild: &GuildMembership,
    online: &mut OnlinePlayers,
    viewer: u64,
) -> GuildSnapshot {
    let mut snapshot = build_guild_snapshot(db, guild, online, viewer);
    if !character_has_right(
        db,
        viewer,
        guild.guild_id,
        shared::protocol::GUILD_RIGHT_VIEW_OFFICER_NOTE,
    ) {
        for member in &mut snapshot.members {
            member.officer_note.clear();
        }
    }
    snapshot
}

fn build_guild_snapshot(
    db: &redb::Database,
    guild: &GuildMembership,
    online_players: &mut OnlinePlayers,
    viewer: u64,
) -> GuildSnapshot {
    let profile = load_guild_profile(db, guild.guild_id);
    let online = online_players.member_map(guild.guild_id);
    let ranks = crate::persistence::banks::load_guild(db, guild.guild_id);
    let members = list_guild_members(db, guild.guild_id)
        .into_iter()
        .map(|member| {
            let is_self = member.character_id == viewer;
            let mut snapshot = build_member_snapshot(db, member, &online, &ranks);
            snapshot.is_self = is_self;
            snapshot
        })
        .collect();
    GuildSnapshot {
        tabard: crate::persistence::guild_tabards::load_design(db, guild.guild_id),
        guild_id: guild.guild_id,
        guild_name: guild.guild_name.clone(),
        created_at: crate::persistence::charters::created_at(db, guild.guild_id),
        motd: profile.motd,
        info_text: profile.info_text,
        members,
    }
}

fn build_member_snapshot(
    db: &redb::Database,
    member: GuildMemberRecord,
    online: &BTreeMap<String, OnlineMemberState>,
    guild: &crate::persistence::banks::GuildData,
) -> GuildMemberSnapshot {
    let online_state = online.get(&member.name);
    let rank = crate::persistence::banks::load_guild_member_rank(db, guild, member.character_id);
    let officer_note = load_guild_member_note(db, member.character_id).officer_note;
    GuildMemberSnapshot {
        character_name: member.name.clone(),
        level: online_state
            .map(|state| state.level)
            .unwrap_or(member.level),
        class_name: online_state
            .map(|state| state.class_name.clone())
            .unwrap_or_else(|| class_name_from_id(member.class_id)),
        rank_name: guild.ranks[usize::from(rank)].name.clone(),
        is_online: online_state.is_some(),
        officer_note,
        public_note: crate::persistence_character::load_guild_public_note(db, member.character_id),
        is_self: false,
        online_zone: online_state.and_then(|state| state.online_zone.clone()),
        last_online: if online_state.is_some() {
            "Online".into()
        } else {
            "Offline".into()
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OnlineMemberState {
    level: u16,
    class_name: String,
    online_zone: Option<String>,
}

fn send_guild_update(
    conn_entity: Entity,
    guild: Option<GuildSnapshot>,
    message: Option<String>,
    error: Option<String>,
    senders: &mut GuildSenders,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<GuildChannel>(GuildStateUpdate {
        guild,
        message,
        error,
    });
}

fn class_name_from_id(class_id: u8) -> String {
    match class_id {
        1 => "Warrior",
        2 => "Paladin",
        3 => "Hunter",
        4 => "Rogue",
        5 => "Priest",
        6 => "Death Knight",
        7 => "Shaman",
        8 => "Mage",
        9 => "Warlock",
        10 => "Monk",
        11 => "Druid",
        12 => "Demon Hunter",
        13 => "Evoker",
        _ => "Unknown",
    }
    .into()
}

#[cfg(test)]
#[path = "guild_online_zone_tests.rs"]
mod guild_online_zone_tests;

#[cfg(test)]
#[path = "guild_ranks_tests.rs"]
mod guild_ranks_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence_types::GuildProfileData;

    #[test]
    fn update_officer_note_rejects_target_outside_guild() {
        let db = crate::persistence::temp_database();
        let (alice_id, _) =
            crate::persistence_character::create_character(&db, 1, "Alice", 1, 5).unwrap();
        let (bob_id, _) =
            crate::persistence_character::create_character(&db, 2, "Bob", 1, 8).unwrap();
        crate::persistence_character::set_character_guild_by_id(&db, alice_id, 7, "Raid Team")
            .unwrap();
        crate::persistence_character::set_character_guild_by_id(&db, bob_id, 9, "Other Team")
            .unwrap();

        let error = update_officer_note(
            &db,
            &GuildMembership {
                guild_id: 7,
                guild_name: "Raid Team".into(),
            },
            SetGuildOfficerNote {
                character_name: "Bob".into(),
                note: "Bench".into(),
            },
        )
        .expect_err("target should not be in guild");

        assert_eq!(error, "guild member not found in Raid Team: Bob");
    }

    #[test]
    fn build_guild_snapshot_uses_profile_and_online_state() {
        let db = crate::persistence::temp_database();
        let (alice_id, _) =
            crate::persistence_character::create_character(&db, 1, "Alice", 1, 5).unwrap();
        crate::persistence_character::set_character_guild_by_id(&db, alice_id, 7, "Raid Team")
            .unwrap();
        save_guild_profile(
            &db,
            7,
            &GuildProfileData {
                motd: "Bring flasks".into(),
                info_text: "Wed/Sun raids".into(),
            },
        );
        save_guild_member_note(
            &db,
            alice_id,
            &GuildMemberNoteData {
                officer_note: "Reliable healer".into(),
            },
        );

        let profile = load_guild_profile(&db, 7);
        let member = list_guild_members(&db, 7)
            .pop()
            .expect("member should exist");
        let snapshot = build_member_snapshot(
            &db,
            member,
            &BTreeMap::from([(
                "Alice".into(),
                OnlineMemberState {
                    level: 60,
                    class_name: "Priest".into(),
                    online_zone: None,
                },
            )]),
            &crate::persistence::banks::GuildData::default(),
        );

        assert_eq!(profile.motd, "Bring flasks");
        assert_eq!(profile.info_text, "Wed/Sun raids");
        assert!(snapshot.is_online);
        assert_eq!(snapshot.officer_note, "Reliable healer");
        assert_eq!(snapshot.class_name, "Priest");
        // No stored rank: the lowest default rank.
        assert_eq!(snapshot.rank_name, "Initiate");
    }
}
