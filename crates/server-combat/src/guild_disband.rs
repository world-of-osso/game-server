//! TC Guild::Disband: destroy the current guild and its owned durable state together.
use crate::persistence::{self, banks};
use crate::persistence_types::{GuildMembershipData, PendingGuildInviteData};
use redb::{Database, ReadableTable, WriteTransaction};

pub(super) fn delete_guild(db: &Database, guild_id: u32) {
    let transaction = db.begin_write().expect("begin guild disband");
    let members = read_members(&transaction, guild_id);
    let name = &members
        .first()
        .expect("disband has an authenticated master")
        .1;
    remove_members(&transaction, &members);
    remove_pending_invites(&transaction, guild_id);
    remove_bank_usage(&transaction, guild_id);
    remove_guild_metadata(&transaction, guild_id);
    persistence::charters::delete_guild_name_in(&transaction, name);
    transaction.commit().expect("commit complete guild disband");
}

fn read_members(transaction: &WriteTransaction, guild_id: u32) -> Vec<(u64, String)> {
    let table = transaction
        .open_table(persistence::CHARACTER_GUILDS)
        .expect("open disband memberships");
    table
        .iter()
        .expect("iterate disband memberships")
        .filter_map(|row| {
            let (id, value) = row.expect("read disband membership");
            let member: GuildMembershipData =
                bitcode::decode(value.value()).expect("decode disband membership");
            (member.guild_id == guild_id).then(|| (id.value(), member.guild_name))
        })
        .collect()
}

fn remove_members(transaction: &WriteTransaction, members: &[(u64, String)]) {
    for definition in [
        persistence::CHARACTER_GUILDS,
        persistence::GUILD_MEMBER_NOTES,
    ] {
        let mut table = transaction
            .open_table(definition)
            .expect("open disband member data");
        for (id, _) in members {
            table.remove(*id).expect("delete disband member data");
        }
    }
    let mut ranks = transaction
        .open_table(banks::GUILD_MEMBER_RANKS)
        .expect("open disband member ranks");
    let mut public_notes = transaction
        .open_table(persistence::GUILD_PUBLIC_NOTES)
        .expect("open disband public notes");
    for (id, _) in members {
        ranks.remove(*id).expect("delete disband member rank");
        public_notes
            .remove(*id)
            .expect("delete disband public note");
    }
}

fn remove_pending_invites(transaction: &WriteTransaction, guild_id: u32) {
    let mut table = transaction
        .open_table(persistence::PENDING_GUILD_INVITES)
        .expect("open disband invitations");
    let targets: Vec<_> = table
        .iter()
        .expect("iterate disband invitations")
        .filter_map(|row| {
            let (id, value) = row.expect("read disband invitation");
            let invite: PendingGuildInviteData =
                bitcode::decode(value.value()).expect("decode disband invitation");
            (invite.guild_id == guild_id).then_some(id.value())
        })
        .collect();
    for id in targets {
        table.remove(id).expect("delete disband invitation");
    }
}

fn remove_bank_usage(transaction: &WriteTransaction, guild_id: u32) {
    let mut table = transaction
        .open_table(banks::GUILD_BANK_USAGE)
        .expect("open disband bank usage");
    let characters: Vec<_> = table
        .iter()
        .expect("iterate disband bank usage")
        .filter_map(|row| {
            let (id, value) = row.expect("read disband bank usage");
            let usage: banks::GuildBankUsage =
                bitcode::decode(value.value()).expect("decode disband bank usage");
            (usage.guild_id == guild_id).then_some(id.value())
        })
        .collect();
    for id in characters {
        table.remove(id).expect("delete disband bank usage");
    }
}

fn remove_guild_metadata(transaction: &WriteTransaction, guild_id: u32) {
    for definition in [
        banks::GUILDS,
        banks::GUILD_BANK_LOGS,
        persistence::GUILD_PROFILES,
        persistence::guild_tabards::DESIGNS,
    ] {
        transaction
            .open_table(definition)
            .expect("open disband guild data")
            .remove(guild_id)
            .expect("delete disband guild data");
    }
    for definition in [crate::guild_event_log::EVENTS, crate::guild_news::NEWS] {
        transaction
            .open_table(definition)
            .expect("open disband guild history")
            .remove(guild_id)
            .expect("delete disband guild history");
    }
    transaction
        .open_table(persistence::charters::CREATED_AT)
        .expect("open disband creation dates")
        .remove(guild_id)
        .expect("delete disband creation date");
}
