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
pub(crate) struct PendingInventoryCommits {
    mutations: VecDeque<PendingCommit>,
    initial: VecDeque<PendingInitial>,
}

#[derive(Clone)]
pub(super) struct InitialInventory {
    pub snapshot: InventorySnapshot,
    pub equipment: EquipmentSnapshot,
    pub view: InventoryView,
    pub appearances: Vec<(shared::item_data::ItemDefinitionSource, u32)>,
    pub equipped: EquippedItems,
}
struct PendingInitial {
    connection: Entity,
    player: Entity,
    character: u64,
    receipt: CommitReceipt<Result<InitialInventory, InventoryError>>,
}
impl PendingInventoryCommits {
    pub(super) fn push_initial(
        &mut self,
        connection: Entity,
        player: Entity,
        character: u64,
        receipt: CommitReceipt<Result<InitialInventory, InventoryError>>,
    ) {
        self.initial.push_back(PendingInitial {
            connection,
            player,
            character,
            receipt,
        });
    }
    pub(super) fn push(
        &mut self,
        connection: Entity,
        character: u64,
        bank_move: bool,
        receipt: CommitReceipt<OpResult>,
    ) {
        self.mutations.push_back(PendingCommit {
            connection,
            character,
            bank_move,
            receipt,
        });
    }
    #[cfg(test)]
    pub(crate) fn is_idle(&self) -> bool {
        self.mutations.is_empty() && self.initial.is_empty()
    }
}
pub(super) fn publish_inventory_commits(
    mut pending: ResMut<PendingInventoryCommits>,
    sessions: Query<(&InWorld, &PlayerLink), With<Authenticated>>,
    mut commands: Commands,
    mut snapshots: Query<&mut MessageSender<InventorySnapshot>>,
    mut equipment: Query<&mut MessageSender<EquipmentSnapshot>>,
    mut updates: ResMut<GoldUpdateQueue>,
    mut banks: ResMut<crate::bank::ChangedCharacterBanks>,
    mut senders: ErrorSenders,
) {
    while let Some(result) = pending
        .initial
        .front()
        .and_then(|commit| commit.receipt.result())
    {
        let commit = pending
            .initial
            .pop_front()
            .expect("completed initial inventory");
        let same_player = sessions
            .get(commit.connection)
            .is_ok_and(|(session, link)| {
                session.character_id == commit.character && link.0 == commit.player
            });
        if !same_player {
            continue;
        }
        let result = result
            .unwrap_or_else(|message| Err(error(InventoryErrorReason::InternalBagError, message)));
        match result {
            Ok(initial) => {
                if let Ok(mut sender) = snapshots.get_mut(commit.connection) {
                    sender.send::<InventoryChannel>(initial.snapshot);
                }
                if let Ok(mut sender) = equipment.get_mut(commit.connection) {
                    sender.send::<InventoryChannel>(initial.equipment);
                }
                commands.entity(commit.connection).insert((
                    SentInventoryView(initial.view),
                    AppearanceSourceItems(initial.appearances),
                ));
                commands.entity(commit.player).insert(initial.equipped);
            }
            Err(error) => send_error(commit.connection, &mut senders, error),
        }
    }
    while let Some(result) = pending
        .mutations
        .front()
        .and_then(|commit| commit.receipt.result())
    {
        let commit = pending
            .mutations
            .pop_front()
            .expect("completed inventory receipt");
        let result = result
            .unwrap_or_else(|message| Err(error(InventoryErrorReason::InternalBagError, message)));
        let same_character = sessions
            .get(commit.connection)
            .is_ok_and(|(session, _)| session.character_id == commit.character);
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
