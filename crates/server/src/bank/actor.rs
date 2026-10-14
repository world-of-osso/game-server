use super::transactions::Publication;
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

struct PendingCommit {
    patron: Patron,
    npc: u64,
    receipt: CommitReceipt<Publication>,
}
#[derive(Resource, Default)]
pub(crate) struct PendingBankCommits(VecDeque<PendingCommit>);
impl PendingBankCommits {
    pub(super) fn push(&mut self, patron: Patron, npc: u64, receipt: CommitReceipt<Publication>) {
        self.0.push_back(PendingCommit {
            patron,
            npc,
            receipt,
        });
    }
    #[cfg(test)]
    pub(crate) fn is_idle(&self) -> bool {
        self.0.is_empty()
    }
}

pub(super) fn publish_bank_commits(
    mut pending: ResMut<PendingBankCommits>,
    sessions: Query<(&InWorld, &Authenticated)>,
    mut vault: Vault,
) {
    while let Some(result) = pending.0.front().and_then(|commit| commit.receipt.result()) {
        let commit = pending.0.pop_front().expect("completed bank receipt");
        let patron = commit.patron;
        let publication = match result {
            Ok(publication) => publication,
            Err(error) => {
                error!("bank commit character={}: {error}", patron.character_id);
                vault.fail(patron.connection, commit.npc, BankError::ItemNotFound);
                continue;
            }
        };
        if let Some(gold) = publication.gold {
            vault.gold.record(patron.character_id, gold);
        }
        for (character, event) in publication.criteria {
            vault.commands.queue(move |world: &mut World| {
                crate::achievement::record_character_event(world, character, event);
            });
        }
        let same_owner = sessions.get(patron.connection).is_ok_and(|(world, auth)| {
            world.character_id == patron.character_id && auth.account_id == patron.account_id
        });
        if !same_owner {
            continue;
        }
        for contents in publication.contents {
            if let Ok(mut sender) = vault.contents.get_mut(patron.connection) {
                sender.send::<BankChannel>(contents);
            }
        }
        if let Some(error) = publication.refusal {
            vault.fail(patron.connection, commit.npc, error);
        }
    }
}
