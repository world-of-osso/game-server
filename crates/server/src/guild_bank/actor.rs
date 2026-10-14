use super::transactions::Publication;
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;
struct PendingCommit {
    connection: Entity,
    character: u64,
    object: u64,
    receipt: CommitReceipt<Publication>,
}
#[derive(Resource, Default)]
pub(crate) struct PendingGuildBankCommits(VecDeque<PendingCommit>);
impl PendingGuildBankCommits {
    pub(super) fn push(
        &mut self,
        connection: Entity,
        character: u64,
        object: u64,
        receipt: CommitReceipt<Publication>,
    ) {
        self.0.push_back(PendingCommit {
            connection,
            character,
            object,
            receipt,
        });
    }
    #[cfg(test)]
    pub(crate) fn is_idle(&self) -> bool {
        self.0.is_empty()
    }
}
pub(super) fn publish_guild_bank_commits(
    mut pending: ResMut<PendingGuildBankCommits>,
    sessions: Query<&InWorld>,
    mut vault: GuildVault,
) {
    while let Some(result) = pending.0.front().and_then(|commit| commit.receipt.result()) {
        let commit = pending.0.pop_front().expect("completed guild bank receipt");
        let publication = match result {
            Ok(publication) => publication,
            Err(error) => {
                error!("guild bank commit character={}: {error}", commit.character);
                vault.fail(
                    commit.connection,
                    commit.object,
                    GuildBankError::ItemNotFound,
                );
                continue;
            }
        };
        if let Some(gold) = publication.gold {
            vault.gold.record(commit.character, gold);
        }
        if let Some(guild) = publication.guild {
            vault.changed.0.push(guild);
        }
        let same_character = sessions
            .get(commit.connection)
            .is_ok_and(|session| session.character_id == commit.character);
        if !same_character {
            continue;
        }
        for log in publication.logs {
            if let Ok(mut sender) = vault.logs.get_mut(commit.connection) {
                sender.send::<GuildBankChannel>(log);
            }
        }
        if let Some(error) = publication.refusal {
            vault.fail(commit.connection, commit.object, error);
        }
    }
}
