//! Durable inventory mutations publish owner deltas only after their FIFO receipt.
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;
struct PendingCommit {
    connection: Entity,
    character: u64,
    bank_move: bool,
    receipt: CommitReceipt<OpResult>,
}
#[derive(Resource, Default)]
pub(crate) struct PendingInventoryCommits(VecDeque<PendingCommit>);
impl PendingInventoryCommits {
    pub(super) fn push(
        &mut self,
        connection: Entity,
        character: u64,
        bank_move: bool,
        receipt: CommitReceipt<OpResult>,
    ) {
        self.0.push_back(PendingCommit {
            connection,
            character,
            bank_move,
            receipt,
        });
    }
    #[cfg(test)]
    pub(crate) fn is_idle(&self) -> bool {
        self.0.is_empty()
    }
}
pub(super) fn publish_inventory_commits(
    mut pending: ResMut<PendingInventoryCommits>,
    sessions: Query<&InWorld, With<Authenticated>>,
    mut updates: ResMut<GoldUpdateQueue>,
    mut banks: ResMut<crate::bank::ChangedCharacterBanks>,
    mut senders: ErrorSenders,
) {
    while let Some(result) = pending.0.front().and_then(|commit| commit.receipt.result()) {
        let commit = pending.0.pop_front().expect("completed inventory receipt");
        let result = result
            .unwrap_or_else(|message| Err(error(InventoryErrorReason::InternalBagError, message)));
        let same_character = sessions
            .get(commit.connection)
            .is_ok_and(|session| session.character_id == commit.character);
        match result {
            Ok(()) => {
                updates.record_items(commit.character);
                if same_character && commit.bank_move {
                    banks.0.insert(commit.connection);
                }
            }
            Err(error) if same_character => send_error(commit.connection, &mut senders, error),
            Err(_) => {}
        }
    }
}
