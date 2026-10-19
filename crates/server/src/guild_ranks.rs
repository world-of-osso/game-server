//! Authoritative Retail guild rank controls. Rank identities and member positions are
//! persisted in one redb transaction; the connection supplies the actor, never the packet.
use crate::auth::{Authenticated, InWorld};
use crate::guild_bank::ChangedGuilds;
use crate::persistence::banks::{
    GuildData, GuildRankData, GuildTabRights, load_guild, load_guild_member_rank, save_guild_ranks,
};
use crate::persistence::{self, GameDatabase};
use crate::persistence_character::list_guild_members;
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::protocol::*;

pub(crate) fn character_has_right(
    db: &redb::Database,
    character: u64,
    guild_id: u32,
    right: u32,
) -> bool {
    let Some(membership) = persistence::load_character_guild(db, character) else {
        return false;
    };
    if membership.guild_id != guild_id {
        return false;
    }
    let guild = load_guild(db, guild_id);
    let rank = load_guild_member_rank(db, &guild, character);
    rank_has_right(&guild, rank, right)
}

fn rank_has_right(guild: &GuildData, rank: u8, right: u32) -> bool {
    rank == 0
        || guild
            .ranks
            .get(usize::from(rank))
            .is_some_and(|rank| rank.rights & right == right)
}

pub(super) fn apply_rank_request(
    db: &redb::Database,
    guild_id: u32,
    actor: u64,
    request: GuildRankRequest,
) -> Result<(), GuildRankError> {
    if persistence::load_character_guild(db, actor).is_none_or(|g| g.guild_id != guild_id) {
        return Err(GuildRankError::NotInGuild);
    }
    if matches!(
        request,
        GuildRankRequest::Query | GuildRankRequest::QueryEventLog
    ) {
        return Ok(());
    }
    if let GuildRankRequest::SetMemberNote {
        character_name,
        note,
        officer,
    } = &request
    {
        return set_member_note(db, guild_id, actor, character_name, note, *officer);
    }
    let mut guild = load_guild(db, guild_id);
    let actor_rank = load_guild_member_rank(db, &guild, actor);
    if matches!(
        request,
        GuildRankRequest::Leave | GuildRankRequest::RemoveMember { .. }
    ) {
        return remove_member(db, guild_id, actor, actor_rank, &guild, request);
    }
    let mut members: Vec<_> = list_guild_members(db, guild_id)
        .into_iter()
        .map(|m| {
            let rank = load_guild_member_rank(db, &guild, m.character_id);
            (m.character_id, m.name, rank)
        })
        .collect();
    let event = if let GuildRankRequest::SetMemberRank {
        character_name,
        rank,
    } = &request
    {
        assign_member_rank(
            &guild,
            &mut members,
            (actor, actor_rank),
            character_name,
            *rank,
        )?;
        // TC direct assignment broadcasts/persists once, without step-event log entries.
        None
    } else if let GuildRankRequest::Promote { character_name }
    | GuildRankRequest::Demote { character_name } = &request
    {
        let promote = matches!(request, GuildRankRequest::Promote { .. });
        let (member, rank) =
            change_member_rank(&guild, &mut members, actor_rank, character_name, promote)?;
        let actor = members
            .iter()
            .find(|(id, _, _)| *id == actor)
            .expect("authenticated actor is a stored guild member")
            .1
            .clone();
        Some(if promote {
            GuildEventKind::Promote {
                actor,
                member,
                rank,
            }
        } else {
            GuildEventKind::Demote {
                actor,
                member,
                rank,
            }
        })
    } else {
        if actor_rank != 0 {
            return Err(GuildRankError::Permissions);
        }
        edit_rank(&mut guild, &mut members, request)?;
        None
    };
    let positions: Vec<_> = members
        .into_iter()
        .map(|(id, _, rank)| (id, rank))
        .collect();
    save_guild_ranks(db, guild_id, &guild, &positions, event);
    Ok(())
}

/// Retail GuildRoster.lua permits a member to edit their own public note.
fn set_member_note(
    db: &redb::Database,
    guild_id: u32,
    actor: u64,
    name: &str,
    note: &str,
    officer: bool,
) -> Result<(), GuildRankError> {
    let target = list_guild_members(db, guild_id)
        .into_iter()
        .find(|member| member.name.eq_ignore_ascii_case(name))
        .ok_or(GuildRankError::MemberNotFound)?;
    let right = if officer {
        GUILD_RIGHT_EDIT_OFFICER_NOTE
    } else {
        GUILD_RIGHT_EDIT_PUBLIC_NOTE
    };
    if (officer || target.character_id != actor) && !character_has_right(db, actor, guild_id, right)
    {
        return Err(GuildRankError::Permissions);
    }
    if !valid_guild_member_note(note) {
        return Err(GuildRankError::NoteTooLong);
    }
    if officer {
        crate::persistence_character::save_guild_member_note(
            db,
            target.character_id,
            &crate::persistence_types::GuildMemberNoteData {
                officer_note: note.to_owned(),
            },
        );
    } else {
        crate::persistence_character::save_guild_public_note(db, target.character_id, note);
    }
    Ok(())
}

fn remove_member(
    db: &redb::Database,
    guild_id: u32,
    actor: u64,
    actor_rank: u8,
    guild: &GuildData,
    request: GuildRankRequest,
) -> Result<(), GuildRankError> {
    let target = match request {
        GuildRankRequest::Leave => {
            if actor_rank == 0 {
                return Err(GuildRankError::LeaderCannotLeave);
            }
            actor
        }
        GuildRankRequest::RemoveMember { character_name } => {
            if !character_has_right(db, actor, guild_id, GUILD_RIGHT_REMOVE) {
                return Err(GuildRankError::Permissions);
            }
            let target = list_guild_members(db, guild_id)
                .into_iter()
                .find(|member| member.name.eq_ignore_ascii_case(&character_name))
                .ok_or(GuildRankError::MemberNotFound)?;
            if load_guild_member_rank(db, guild, target.character_id) <= actor_rank {
                return Err(GuildRankError::RankTooHigh);
            }
            target.character_id
        }
        _ => unreachable!("only membership removal requests"),
    };
    persistence::remove_character_guild(db, target, actor);
    Ok(())
}

fn change_member_rank(
    guild: &GuildData,
    members: &mut [(u64, String, u8)],
    actor_rank: u8,
    name: &str,
    promote: bool,
) -> Result<(String, String), GuildRankError> {
    let right = if promote {
        GUILD_RIGHT_PROMOTE
    } else {
        GUILD_RIGHT_DEMOTE
    };
    if !rank_has_right(guild, actor_rank, right) {
        return Err(GuildRankError::Permissions);
    }
    let target = members
        .iter_mut()
        .find(|(_, target, _)| target.eq_ignore_ascii_case(name))
        .ok_or(GuildRankError::MemberNotFound)?;
    if target.2 <= actor_rank || (promote && target.2 <= actor_rank + 1) {
        return Err(GuildRankError::RankTooHigh);
    }
    if promote {
        target.2 -= 1;
    } else if target.2 < guild.lowest_rank() {
        target.2 += 1;
    } else {
        return Err(GuildRankError::RankTooLow);
    }
    Ok((
        target.1.clone(),
        guild.ranks[usize::from(target.2)].name.clone(),
    ))
}

fn assign_member_rank(
    guild: &GuildData,
    members: &mut [(u64, String, u8)],
    (actor, actor_rank): (u64, u8),
    name: &str,
    rank: u8,
) -> Result<(), GuildRankError> {
    let target = members
        .iter_mut()
        .find(|(_, name_in_guild, _)| name_in_guild.eq_ignore_ascii_case(name))
        .ok_or(GuildRankError::MemberNotFound)?;
    let right = if rank > target.2 {
        GUILD_RIGHT_DEMOTE
    } else {
        GUILD_RIGHT_PROMOTE
    };
    if !rank_has_right(guild, actor_rank, right) {
        return Err(GuildRankError::Permissions);
    }
    if target.0 == actor {
        return Err(GuildRankError::InvalidName);
    }
    if target.2 <= actor_rank || rank <= actor_rank {
        return Err(GuildRankError::RankTooHigh);
    }
    if rank > guild.lowest_rank() {
        return Err(GuildRankError::RankTooLow);
    }
    target.2 = rank;
    Ok(())
}

fn mutable_rank(guild: &mut GuildData, rank: u8) -> Result<&mut GuildRankData, GuildRankError> {
    if rank == 0 {
        return Err(GuildRankError::Permissions);
    }
    guild
        .ranks
        .get_mut(usize::from(rank))
        .ok_or(GuildRankError::RankTooLow)
}

fn rank_name(name: String) -> Result<String, GuildRankError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 15 || name.chars().any(char::is_control) {
        return Err(GuildRankError::InvalidName);
    }
    Ok(name.to_owned())
}

fn edit_rank(
    guild: &mut GuildData,
    members: &mut [(u64, String, u8)],
    request: GuildRankRequest,
) -> Result<(), GuildRankError> {
    match request {
        GuildRankRequest::Add { name } => {
            if guild.ranks.len() >= GUILD_MAX_RANKS {
                return Err(GuildRankError::TooManyRanks);
            }
            guild.ranks.push(GuildRankData {
                name: rank_name(name)?,
                rights: GUILD_RIGHT_CHAT_LISTEN | GUILD_RIGHT_CHAT_SPEAK,
                withdraw_gold: false,
                withdraw_repair: false,
                gold_per_day: 0,
                tabs: vec![GuildTabRights::default(); GUILD_BANK_MAX_TABS],
            });
        }
        GuildRankRequest::Remove { rank } => {
            mutable_rank(guild, rank)?;
            if guild.ranks.len() <= GUILD_MIN_RANKS {
                return Err(GuildRankError::TooFewRanks);
            }
            if members.iter().any(|(_, _, position)| *position == rank) {
                return Err(GuildRankError::RankInUse);
            }
            guild.ranks.remove(usize::from(rank));
            for (_, _, position) in members {
                if *position > rank {
                    *position -= 1;
                }
            }
        }
        GuildRankRequest::Rename { rank, name } => {
            mutable_rank(guild, rank)?.name = rank_name(name)?
        }
        GuildRankRequest::Move { rank, up } => move_rank(guild, members, rank, up)?,
        GuildRankRequest::SetPermissions {
            rank,
            rights,
            gold_per_day,
        } => {
            if rights & !GUILD_RIGHT_ALL != 0 {
                return Err(GuildRankError::Permissions);
            }
            let rank = mutable_rank(guild, rank)?;
            rank.rights = rights & !(GUILD_RIGHT_WITHDRAW_GOLD | GUILD_RIGHT_WITHDRAW_REPAIR);
            rank.withdraw_gold = rights & GUILD_RIGHT_WITHDRAW_GOLD != 0;
            rank.withdraw_repair = rights & GUILD_RIGHT_WITHDRAW_REPAIR != 0;
            rank.gold_per_day = gold_per_day;
        }
        GuildRankRequest::SetTab {
            rank,
            tab,
            view,
            deposit,
            withdrawals_per_day,
        } => {
            let tab = mutable_rank(guild, rank)?
                .tabs
                .get_mut(usize::from(tab))
                .ok_or(GuildRankError::WrongTab)?;
            *tab = GuildTabRights {
                view,
                deposit,
                withdrawals_per_day,
            };
        }
        GuildRankRequest::Leave
        | GuildRankRequest::RemoveMember { .. }
        | GuildRankRequest::TransferLeadership { .. } => {
            return Err(GuildRankError::Permissions);
        }
        GuildRankRequest::Query | GuildRankRequest::QueryEventLog => {}
        GuildRankRequest::Promote { .. }
        | GuildRankRequest::Demote { .. }
        | GuildRankRequest::SetMemberNote { .. }
        | GuildRankRequest::SetMemberRank { .. } => {
            unreachable!("handled before rank editing")
        }
    }
    Ok(())
}

fn move_rank(
    guild: &mut GuildData,
    members: &mut [(u64, String, u8)],
    rank: u8,
    up: bool,
) -> Result<(), GuildRankError> {
    mutable_rank(guild, rank)?;
    let destination = if up {
        rank.checked_sub(1)
            .filter(|r| *r > 0)
            .ok_or(GuildRankError::RankTooHigh)?
    } else {
        rank.checked_add(1)
            .filter(|r| usize::from(*r) < guild.ranks.len())
            .ok_or(GuildRankError::RankTooLow)?
    };
    guild
        .ranks
        .swap(usize::from(rank), usize::from(destination));
    for (_, _, position) in members {
        if *position == rank {
            *position = destination;
        } else if *position == destination {
            *position = rank;
        }
    }
    Ok(())
}

fn rank_settings(rank: &GuildRankData, index: usize) -> GuildRankSettings {
    let mut rights = rank.rights;
    if rank.withdraw_gold {
        rights |= GUILD_RIGHT_WITHDRAW_GOLD;
    }
    if rank.withdraw_repair {
        rights |= GUILD_RIGHT_WITHDRAW_REPAIR;
    }
    GuildRankSettings {
        name: rank.name.clone(),
        rights: if index == 0 { GUILD_RIGHT_ALL } else { rights },
        gold_per_day: rank.gold_per_day,
        tabs: rank
            .tabs
            .iter()
            .map(|tab| GuildRankTab {
                view: index == 0 || tab.view,
                deposit: index == 0 || tab.deposit,
                withdrawals_per_day: tab.withdrawals_per_day,
            })
            .collect(),
    }
}

pub(crate) fn snapshot(
    db: &redb::Database,
    character: u64,
    error: Option<GuildRankError>,
) -> GuildRanksState {
    let Some(membership) = persistence::load_character_guild(db, character) else {
        return GuildRanksState {
            ranks: Vec::new(),
            members: Vec::new(),
            own_rank: u8::MAX,
            tab_names: Vec::new(),
            error,
        };
    };
    let guild = load_guild(db, membership.guild_id);
    GuildRanksState {
        ranks: guild
            .ranks
            .iter()
            .enumerate()
            .map(|(i, r)| rank_settings(r, i))
            .collect(),
        members: list_guild_members(db, membership.guild_id)
            .into_iter()
            .map(|member| GuildRankMember {
                rank: load_guild_member_rank(db, &guild, member.character_id),
                character_name: member.name,
            })
            .collect(),
        own_rank: load_guild_member_rank(db, &guild, character),
        tab_names: guild.tabs.iter().map(|tab| tab.name.clone()).collect(),
        error,
    }
}

type Receivers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static InWorld,
        &'static mut MessageReceiver<GuildRankRequest>,
    ),
    With<Authenticated>,
>;
type Connections<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static InWorld,
        &'static crate::networking::PlayerLink,
    ),
    With<Authenticated>,
>;

pub(super) fn process_rank_requests(
    db: Res<GameDatabase>,
    mut receivers: Receivers,
    connections: Connections,
    mut senders: Query<&mut MessageSender<GuildRanksState>>,
    mut log_senders: Query<&mut MessageSender<GuildEventLog>>,
    mut changed: ResMut<ChangedGuilds>,
    mut commands: Commands,
) {
    for (connection, world, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            if matches!(request, GuildRankRequest::QueryEventLog) {
                if let Ok(mut sender) = log_senders.get_mut(connection) {
                    sender.send::<GuildChannel>(crate::guild_event_log::query(
                        &db.db,
                        world.character_id,
                    ));
                }
                continue;
            }
            let membership = persistence::load_character_guild(&db.db, world.character_id);
            let query = matches!(request, GuildRankRequest::Query);
            let previous_members = membership
                .as_ref()
                .map(|g| list_guild_members(&db.db, g.guild_id))
                .unwrap_or_default();
            let result = membership
                .as_ref()
                .ok_or(GuildRankError::NotInGuild)
                .and_then(|g| apply_rank_request(&db.db, g.guild_id, world.character_id, request));
            if query || result.is_err() {
                if let Ok(mut sender) = senders.get_mut(connection) {
                    sender.send::<GuildChannel>(snapshot(&db.db, world.character_id, result.err()));
                }
                continue;
            }
            let guild_id = membership.expect("successful guild request").guild_id;
            changed.0.push(guild_id);
            for (recipient, recipient_world, link) in &connections {
                if previous_members
                    .iter()
                    .any(|member| member.character_id == recipient_world.character_id)
                {
                    if persistence::load_character_guild(&db.db, recipient_world.character_id)
                        .is_none()
                    {
                        commands
                            .entity(link.0)
                            .remove::<shared::components::GuildMembership>();
                    }
                    if let Ok(mut sender) = senders.get_mut(recipient) {
                        sender.send::<GuildChannel>(snapshot(
                            &db.db,
                            recipient_world.character_id,
                            None,
                        ));
                    }
                }
            }
        }
    }
}
