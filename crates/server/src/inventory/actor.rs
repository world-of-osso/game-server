//! Durable inventory work publishes owner views only after its FIFO receipt.
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

struct PendingCommit {
    connection: Entity,
    character: u64,
    bank_move: bool,
    receipt: CommitReceipt<OpResult>,
}
struct PendingView {
    connection: Entity,
    player: Entity,
    character: u64,
    receipt: CommitReceipt<Result<PreparedInventory, InventoryError>>,
}
#[derive(Resource, Default)]
pub(crate) struct PendingInventoryCommits {
    mutations: VecDeque<PendingCommit>,
    initial: VecDeque<PendingView>,
    deltas: VecDeque<PendingView>,
}
#[derive(Clone)]
pub(super) struct PreparedInventory {
    snapshot: InventorySnapshot,
    equipment: EquipmentSnapshot,
    view: InventoryView,
    appearances: Vec<(shared::item_data::ItemDefinitionSource, u32)>,
    equipped: EquippedItems,
    appearance: EquipmentAppearance,
}
impl PendingInventoryCommits {
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
    pub(super) fn push_initial(
        &mut self,
        connection: Entity,
        player: Entity,
        character: u64,
        receipt: CommitReceipt<Result<PreparedInventory, InventoryError>>,
    ) {
        self.initial.push_back(PendingView {
            connection,
            player,
            character,
            receipt,
        });
    }
    pub(super) fn push_delta(
        &mut self,
        connection: Entity,
        player: Entity,
        character: u64,
        receipt: CommitReceipt<Result<PreparedInventory, InventoryError>>,
    ) {
        self.deltas.push_back(PendingView {
            connection,
            player,
            character,
            receipt,
        });
    }
    #[cfg(test)]
    pub(crate) fn is_idle(&self) -> bool {
        self.mutations.is_empty() && self.initial.is_empty() && self.deltas.is_empty()
    }
}

pub(super) fn normalize_inventory(
    db: &Database,
    templates: &ItemTemplates,
    character: u64,
) -> Result<PreparedInventory, InventoryError> {
    let model = load_model(db, templates, character)?;
    let prepared = PreparedInventory {
        snapshot: model.inventory_snapshot(),
        equipment: model.equipment_snapshot(),
        view: model.view(),
        appearances: model.appearance_sources(),
        equipped: model.equipped_items(),
        appearance: model.appearance().clone(),
    };
    save_inventory_record(db, character, &model.into_record());
    Ok(prepared)
}

#[derive(SystemParam)]
pub(super) struct PublicationIo<'w, 's> {
    sessions: Query<'w, 's, (&'static InWorld, &'static PlayerLink), With<Authenticated>>,
    commands: Commands<'w, 's>,
    snapshots: Query<'w, 's, &'static mut MessageSender<InventorySnapshot>>,
    equipment: Query<'w, 's, &'static mut MessageSender<EquipmentSnapshot>>,
    deltas: Query<'w, 's, &'static mut MessageSender<InventoryDelta>>,
    errors: ErrorSenders<'w, 's>,
    views: Query<'w, 's, &'static mut SentInventoryView>,
    players: Query<
        'w,
        's,
        (
            &'static mut EquipmentAppearance,
            Option<&'static mut EquippedItems>,
        ),
    >,
    dirty: Option<Res<'w, lightyear::prelude::ReplicationDirtyQueue>>,
}

pub(super) fn publish_inventory_commits(
    mut pending: ResMut<PendingInventoryCommits>,
    mut io: PublicationIo,
    mut updates: ResMut<GoldUpdateQueue>,
    mut banks: ResMut<crate::bank::ChangedCharacterBanks>,
) {
    publish_views(&mut pending.initial, &mut io, true);
    publish_views(&mut pending.deltas, &mut io, false);
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
        let same_character = io
            .sessions
            .get(commit.connection)
            .is_ok_and(|(session, _)| session.character_id == commit.character);
        match result {
            Ok(()) => {
                updates.record_items(commit.character);
                if same_character && commit.bank_move {
                    banks.0.insert(commit.connection);
                }
            }
            Err(error) if same_character => send_error(commit.connection, &mut io.errors, error),
            Err(_) => {}
        }
    }
}

fn publish_views(pending: &mut VecDeque<PendingView>, io: &mut PublicationIo, initial: bool) {
    while let Some(result) = pending.front().and_then(|commit| commit.receipt.result()) {
        let commit = pending.pop_front().expect("completed inventory view");
        let same_player = io
            .sessions
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
            Ok(prepared) if initial => {
                publish_initial(io, commit.connection, commit.player, prepared)
            }
            Ok(prepared) => publish_delta(io, commit.connection, commit.player, prepared),
            Err(error) => send_error(commit.connection, &mut io.errors, error),
        }
    }
}

fn publish_initial(
    io: &mut PublicationIo,
    connection: Entity,
    player: Entity,
    prepared: PreparedInventory,
) {
    if let Ok(mut sender) = io.snapshots.get_mut(connection) {
        sender.send::<InventoryChannel>(prepared.snapshot);
    }
    if let Ok(mut sender) = io.equipment.get_mut(connection) {
        sender.send::<InventoryChannel>(prepared.equipment);
    }
    io.commands.entity(connection).insert((
        SentInventoryView(prepared.view),
        AppearanceSourceItems(prepared.appearances),
    ));
    io.commands.entity(player).insert(prepared.equipped);
}

fn publish_delta(
    io: &mut PublicationIo,
    connection: Entity,
    player: Entity,
    prepared: PreparedInventory,
) {
    let Ok(mut sent) = io.views.get_mut(connection) else {
        error!("committed inventory delta has no owner view: {connection:?}");
        return;
    };
    let changes = diff_views(&sent.0, &prepared.view);
    if !changes.is_empty() {
        if changes.iter().any(changes_bag_slot) {
            if let Ok(mut sender) = io.snapshots.get_mut(connection) {
                sender.send::<InventoryChannel>(prepared.snapshot);
            }
        }
        if let Ok(mut sender) = io.deltas.get_mut(connection) {
            sender.send::<InventoryChannel>(InventoryDelta { changes });
        }
        sent.0 = prepared.view;
    }
    io.commands
        .entity(connection)
        .insert(AppearanceSourceItems(prepared.appearances));
    publish_equipped(io, player, prepared.equipped, prepared.appearance);
}

fn publish_equipped(
    io: &mut PublicationIo,
    player: Entity,
    equipped: EquippedItems,
    appearance: EquipmentAppearance,
) {
    let Ok((mut current, equipped_state)) = io.players.get_mut(player) else {
        return;
    };
    if let Some(mut live) = equipped_state {
        live.set_if_neq(equipped);
    }
    if *current != appearance {
        *current = appearance;
        if let Some(dirty) = io.dirty.as_deref() {
            dirty.mark(player);
        }
    }
}
