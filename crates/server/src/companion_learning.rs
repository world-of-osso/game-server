//! Companion teaching consumes the physical item and grants the pet in one FIFO commit.
use super::*;
use crate::persistence_writer::CommitReceipt;
use redb::ReadableTable;
use shared::protocol::{InventoryChannel, InventoryError, InventoryErrorReason, ItemLocation};
use std::collections::VecDeque;

const BALANCED_BREED: u8 = 3;

#[cfg(test)]
#[path = "companion_learning_fixture.rs"]
mod fixture;
#[cfg(test)]
pub(crate) use fixture::learn_companion_fixture;

#[derive(Clone)]
pub(super) struct LearningRequest {
    pub connection: Entity,
    pub player: Entity,
    pub account: u64,
    pub character: u64,
    pub location: ItemLocation,
    pub item: u32,
    pub guid: u64,
    pub species: u32,
    pub level: u8,
    pub retail_item: Option<u32>,
}

struct PendingLearning {
    request: LearningRequest,
    receipt: CommitReceipt<Result<(), InventoryError>>,
}

#[derive(Resource, Default)]
struct PendingCompanionLearning(VecDeque<PendingLearning>);

pub(super) fn install(app: &mut App) {
    app.init_resource::<PendingCompanionLearning>()
        .add_systems(PostUpdate, publish_learning);
}

#[cfg(test)]
pub(crate) fn persistence_pending(world: &World) -> bool {
    !world.resource::<PendingCompanionLearning>().0.is_empty()
}

pub(super) fn enqueue(world: &mut World, request: LearningRequest) {
    let templates = world
        .resource::<crate::item_templates::ItemTemplates>()
        .clone();
    let owned = request.clone();
    let receipt = world
        .resource::<GameDatabase>()
        .writer
        .enqueue("companion item learning", move |db| {
            Ok(commit_learning(db, &templates, &owned))
        });
    world
        .resource_mut::<PendingCompanionLearning>()
        .0
        .push_back(PendingLearning { request, receipt });
}

fn commit_learning(
    db: &Database,
    templates: &crate::item_templates::ItemTemplates,
    request: &LearningRequest,
) -> Result<(), InventoryError> {
    let mut model = crate::inventory::load_model(db, templates, request.character)?;
    let item = model.existing(request.location)?;
    if item.item_guid != request.guid || item.item_id != request.item {
        return Err(learning_error("Companion item changed before learning"));
    }
    model.destroy(request.location, 1)?;
    persist_learning(db, request, &model.into_record()).map_err(learning_error)
}

fn persist_learning(
    db: &Database,
    request: &LearningRequest,
    inventory: &crate::persistence::inventory_store::InventoryRecord,
) -> Result<(), String> {
    let txn = db
        .begin_write()
        .map_err(|error| format!("Begin companion learning: {error}"))?;
    let mut journal = read_journal_for_write(&txn, request.account)?;
    let guid = allocate_pet_guid(&txn)?;
    journal
        .add_with_guid(
            request.species,
            request.level,
            shared::pet_battle::PetQuality::Common,
            BALANCED_BREED,
            guid,
        )
        .map_err(|reason| format!("cannot learn pet: {reason:?}"))?;
    let bytes = serde_json::to_vec(&journal)
        .map_err(|error| format!("Encode companion journal: {error}"))?;
    txn.open_table(ACCOUNT_PET_JOURNALS)
        .map_err(|error| error.to_string())?
        .insert(request.account, bytes.as_slice())
        .map_err(|error| error.to_string())?;
    crate::persistence::inventory_store::write_inventory_record(&txn, request.character, inventory);
    crate::persistence_commit::commit(txn)
        .map_err(|error| format!("Commit companion learning: {error}"))
}

fn read_journal_for_write(
    txn: &redb::WriteTransaction,
    account: u64,
) -> Result<PetJournal, String> {
    let table = txn
        .open_table(ACCOUNT_PET_JOURNALS)
        .map_err(|error| error.to_string())?;
    let row = table.get(account).map_err(|error| error.to_string())?;
    row.map(|row| {
        serde_json::from_slice(row.value())
            .map_err(|error| format!("Decode companion journal: {error}"))
    })
    .transpose()
    .map(Option::unwrap_or_default)
}

fn allocate_pet_guid(txn: &redb::WriteTransaction) -> Result<u64, String> {
    let mut ids = txn
        .open_table(crate::persistence::NEXT_IDS)
        .map_err(|error| error.to_string())?;
    let current = ids
        .get("battle_pet_instance")
        .map_err(|error| error.to_string())?
        .map(|row| row.value())
        .unwrap_or(0);
    let next = current.checked_add(1).ok_or("Battle pet GUID exhausted")?;
    ids.insert("battle_pet_instance", next)
        .map_err(|error| error.to_string())?;
    Ok(next)
}

fn learning_error(message: impl Into<String>) -> InventoryError {
    InventoryError {
        reason: InventoryErrorReason::CantDoRightNow,
        detail: Some(message.into()),
    }
}

fn owns_session(world: &World, request: &LearningRequest) -> bool {
    let authenticated = world
        .get::<Authenticated>(request.connection)
        .is_some_and(|auth| auth.account_id == request.account);
    let entered = world
        .get::<InWorld>(request.connection)
        .is_some_and(|session| session.character_id == request.character);
    let linked = world
        .get::<PlayerLink>(request.connection)
        .is_some_and(|link| link.0 == request.player);
    authenticated && entered && linked
}

fn publish_learning(world: &mut World) {
    let completed = {
        let mut pending = world.resource_mut::<PendingCompanionLearning>();
        let mut completed = Vec::new();
        while let Some(result) = pending.0.front().and_then(|job| job.receipt.result()) {
            let job = pending.0.pop_front().expect("completed companion receipt");
            completed.push((job.request, result));
        }
        completed
    };
    for (request, result) in completed {
        let result = result.unwrap_or_else(|message| Err(learning_error(message)));
        publish_result(world, request, result);
    }
}

fn publish_result(world: &mut World, request: LearningRequest, result: Result<(), InventoryError>) {
    let current_session = owns_session(world, &request);
    if let Err(error) = result {
        if current_session {
            if let Some(mut sender) =
                world.get_mut::<MessageSender<InventoryError>>(request.connection)
            {
                sender.send::<InventoryChannel>(error);
            }
        }
        return;
    }
    if current_session {
        world
            .resource_mut::<crate::gold::GoldUpdateQueue>()
            .record_items(request.character);
        if let Some(item) = request.retail_item {
            crate::achievement::record_item_use(world, request.player, item);
        }
    }
    publish_account_journal(world, &request, current_session);
}

fn publish_account_journal(world: &mut World, request: &LearningRequest, current_session: bool) {
    let account = request.account;
    let journal = load_journal(&world.resource::<GameDatabase>().db, account);
    let mut connections = world.query::<(Entity, &Authenticated, &InWorld, &PlayerLink)>();
    let recipients: Vec<_> = connections
        .iter(world)
        .filter(|(_, auth, _, _)| auth.account_id == account)
        .map(|(connection, _, _, link)| (connection, link.0))
        .collect();
    for (connection, player) in recipients {
        let Some(mut state) = world.get_mut::<AccountPetJournal>(player) else {
            continue;
        };
        if state.account_id != account {
            continue;
        }
        state.journal = journal.clone();
        refresh_catalog_snapshot(world, player);
        if current_session && player == request.player {
            crate::achievement::record_learned_pet(world, player, request.species);
        }
        send_pet_update(world, connection, player, None);
    }
}
