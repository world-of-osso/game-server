//! Fresh FIFO housing writes; inventory and owner replies follow commit receipts.
use super::*;
use crate::persistence_writer::CommitReceipt;
use shared::protocol::{DecorDyePreview, InventoryChannel, InventoryError, InventoryErrorReason};
use std::collections::VecDeque;

#[derive(Clone, Copy)]
pub(super) enum ReplyOrigin {
    ItemUse,
    Request(u64),
}

enum InventoryPublication {
    Unchanged,
    Items,
    Gold,
}

enum HousingReceipt {
    Mutation(CommitReceipt),
    Dye(CommitReceipt<bool>),
}

impl HousingReceipt {
    // Unit receipts publish their configured effect; dye no-ops suppress inventory updates.
    fn result(&self) -> Option<Result<bool, String>> {
        match self {
            Self::Mutation(receipt) => receipt.result().map(|result| result.map(|()| true)),
            Self::Dye(receipt) => receipt.result(),
        }
    }
}

struct PendingHousingWrite {
    actor: Actor,
    origin: ReplyOrigin,
    inventory: InventoryPublication,
    // Layout observers stay bound to the request map after its editor leaves.
    observer_map: Option<InMap>,
    dye_preview: Option<DecorDyePreview>,
    receipt: HousingReceipt,
}

#[derive(Resource, Default)]
pub(super) struct PendingHousingWrites(VecDeque<PendingHousingWrite>);

pub(crate) struct HousingPersistencePlugin;

impl HousingPersistencePlugin {
    pub(crate) fn install(app: &mut App) {
        transfers::install(app);
        app.init_resource::<PendingHousingWrites>()
            .add_systems(PostUpdate, publish_writes);
    }
}

#[cfg(test)]
pub(crate) fn persistence_pending(world: &World) -> bool {
    !world.resource::<PendingHousingWrites>().0.is_empty()
}

pub(super) fn enqueue_request(world: &mut World, actor: Actor, request: &HousingRequest) -> bool {
    let origin = ReplyOrigin::Request(request.request_id);
    let result = match &request.command {
        HousingCommand::RedeemDecor { item_guid } => {
            enqueue_collection(world, actor, *item_guid, origin)
        }
        HousingCommand::PurchasePlot { plot_id } => {
            enqueue_purchase(world, actor, *plot_id, request.request_id)
        }
        HousingCommand::CommitDecorDyes {
            house_id,
            instance_id,
        } => enqueue_dye_commit(world, actor, *house_id, *instance_id, request.request_id),
        HousingCommand::SaveHouseAccess { .. }
        | HousingCommand::PlaceDecor { .. }
        | HousingCommand::TransformDecor { .. }
        | HousingCommand::MoveDecor { .. }
        | HousingCommand::RotateDecor { .. }
        | HousingCommand::RemoveDecor { .. } => enqueue_house_mutation(world, actor, request),
        _ => return false,
    };
    if let Err(error) = result {
        send_snapshot(world, actor, request.request_id, None, None, Some(error));
    }
    true
}

pub(super) fn enqueue_collection(
    world: &mut World,
    actor: Actor,
    guid: u64,
    origin: ReplyOrigin,
) -> Result<(), String> {
    catalog(world)?;
    let context = actor_context(world, actor)?;
    let receipt = crate::housing_acquisition::redeem(world, context, actor.player, guid)?;
    world
        .resource_mut::<PendingHousingWrites>()
        .0
        .push_back(PendingHousingWrite {
            actor,
            origin,
            inventory: InventoryPublication::Items,
            observer_map: None,
            dye_preview: None,
            receipt: HousingReceipt::Mutation(receipt),
        });
    Ok(())
}

pub(super) fn enqueue_purchase(
    world: &mut World,
    actor: Actor,
    plot_id: u32,
    request_id: u64,
) -> Result<(), String> {
    let context = actor_context(world, actor)?;
    let plot = catalog(world)?.plot(plot_id)?.clone();
    if context.neighborhood_id != Some(plot.neighborhood_id) {
        return Err("Enter this neighborhood before purchasing its plot.".into());
    }
    require_cornerstone_reach(world, actor.player, plot_id)?;
    let receipt = world
        .resource::<GameDatabase>()
        .writer
        .enqueue("housing plot purchase", move |db| {
            store::purchase(db, context, &plot).map(|_| ())
        });
    world
        .resource_mut::<PendingHousingWrites>()
        .0
        .push_back(PendingHousingWrite {
            actor,
            origin: ReplyOrigin::Request(request_id),
            inventory: InventoryPublication::Gold,
            observer_map: None,
            dye_preview: None,
            receipt: HousingReceipt::Mutation(receipt),
        });
    Ok(())
}

fn enqueue_house_mutation(
    world: &mut World,
    actor: Actor,
    request: &HousingRequest,
) -> Result<(), String> {
    let context = actor_context(world, actor)?;
    let rules = catalog(world)?.decor_scale_rules();
    let command = request.command.clone();
    let observer_map = match command {
        HousingCommand::SaveHouseAccess { .. } => None,
        _ => world.get::<InMap>(actor.player).copied(),
    };
    let receipt = world
        .resource::<GameDatabase>()
        .writer
        .enqueue("housing layout/access mutation", move |db| {
            persist_house_mutation(db, context, &command, &rules)
        });
    world
        .resource_mut::<PendingHousingWrites>()
        .0
        .push_back(PendingHousingWrite {
            actor,
            origin: ReplyOrigin::Request(request.request_id),
            inventory: InventoryPublication::Unchanged,
            observer_map,
            dye_preview: None,
            receipt: HousingReceipt::Mutation(receipt),
        });
    Ok(())
}

fn enqueue_dye_commit(
    world: &mut World,
    actor: Actor,
    house_id: u64,
    instance_id: u64,
    request_id: u64,
) -> Result<(), String> {
    let dyes = catalog(world)?.dyes.clone();
    let context = actor_context(world, actor)?;
    let preview = dye_requests::staged_preview(world, actor, house_id, instance_id)?;
    let selection = preview.clone();
    let observer_map = world.get::<InMap>(actor.player).copied();
    let receipt = world
        .resource::<GameDatabase>()
        .writer
        .enqueue("housing dye commit", move |db| {
            store::commit_dyes(db, context, &selection, &dyes)
        });
    world
        .resource_mut::<PendingHousingWrites>()
        .0
        .push_back(PendingHousingWrite {
            actor,
            origin: ReplyOrigin::Request(request_id),
            inventory: InventoryPublication::Items,
            observer_map,
            dye_preview: Some(preview),
            receipt: HousingReceipt::Dye(receipt),
        });
    Ok(())
}

fn persist_house_mutation(
    db: &redb::Database,
    actor: store::Actor,
    command: &HousingCommand,
    rules: &crate::housing_catalog::DecorScaleRules,
) -> Result<(), String> {
    if let HousingCommand::SaveHouseAccess { house_id, access } = command {
        return store::set_access(db, actor, *house_id, *access);
    }
    read_and_validate_layout_scale(db, actor, command, rules)?;
    store::edit(db, actor, command)
}

fn read_and_validate_layout_scale(
    db: &redb::Database,
    actor: store::Actor,
    command: &HousingCommand,
    rules: &crate::housing_catalog::DecorScaleRules,
) -> Result<(), String> {
    match command {
        HousingCommand::PlaceDecor {
            decor_id,
            transform,
            ..
        } => rules.validate(*decor_id, transform.scale),
        HousingCommand::TransformDecor {
            house_id,
            instance_id,
            transform,
        } => {
            let decor_id = read_placed_decor_id(db, actor, *house_id, *instance_id)?;
            rules.validate(decor_id, transform.scale)
        }
        HousingCommand::MoveDecor { .. }
        | HousingCommand::RotateDecor { .. }
        | HousingCommand::RemoveDecor { .. } => Ok(()),
        _ => unreachable!("only layout commands reach scale validation"),
    }
}

fn read_placed_decor_id(
    db: &redb::Database,
    actor: store::Actor,
    house_id: u64,
    instance_id: u64,
) -> Result<u32, String> {
    let layout = store::snapshot(db, actor, Some(house_id), None)?
        .layout
        .ok_or("House layout is unavailable.")?;
    layout
        .decor
        .iter()
        .find(|decor| decor.instance_id == instance_id)
        .map(|decor| decor.decor_id)
        .ok_or_else(|| "Placed decor not found in this house.".into())
}

fn drain_completed(world: &mut World) -> Vec<(PendingHousingWrite, Result<bool, String>)> {
    let mut pending = world.resource_mut::<PendingHousingWrites>();
    let mut completed = Vec::new();
    while let Some(result) = pending.0.front().and_then(|job| job.receipt.result()) {
        let job = pending
            .0
            .pop_front()
            .expect("completed housing write receipt");
        completed.push((job, result));
    }
    completed
}

pub(super) fn owns_housing_session(world: &World, actor: Actor) -> bool {
    let authenticated = world
        .get::<Authenticated>(actor.connection)
        .is_some_and(|auth| auth.account_id == actor.account);
    let in_world = world
        .get::<InWorld>(actor.connection)
        .is_some_and(|session| session.character_id == actor.character);
    let linked = world
        .get::<PlayerLink>(actor.connection)
        .is_some_and(|link| link.0 == actor.player);
    authenticated && in_world && linked
}

pub(super) fn publish_writes(world: &mut World) {
    for (job, result) in drain_completed(world) {
        let same_session = owns_housing_session(world, job.actor);
        if result.is_ok()
            && let Some(map) = job.observer_map
        {
            publish_layout_at(world, map, same_session.then_some(job.actor.connection));
        }
        if same_session {
            publish_write_result(world, job, result);
        }
    }
}

fn publish_inventory(world: &mut World, actor: Actor, publication: InventoryPublication) {
    match publication {
        InventoryPublication::Unchanged => {}
        InventoryPublication::Items => world
            .resource_mut::<crate::gold::GoldUpdateQueue>()
            .record_items(actor.character),
        InventoryPublication::Gold => {
            let gold = crate::persistence::load_inventory(
                &world.resource::<GameDatabase>().db,
                actor.character,
            )
            .gold;
            world
                .resource_mut::<crate::gold::GoldUpdateQueue>()
                .record(actor.character, gold);
        }
    }
}

fn publish_write_result(world: &mut World, job: PendingHousingWrite, result: Result<bool, String>) {
    if result.is_ok()
        && let Some(preview) = &job.dye_preview
    {
        dye_requests::clear_committed_preview(world, job.actor, preview);
    }
    if result
        .as_ref()
        .is_ok_and(|publish_inventory| *publish_inventory)
    {
        publish_inventory(world, job.actor, job.inventory);
    }
    match (job.origin, result) {
        (ReplyOrigin::Request(id), result) => {
            send_snapshot(world, job.actor, id, None, None, result.err())
        }
        (ReplyOrigin::ItemUse, Ok(_)) => send_snapshot(world, job.actor, 0, None, None, None),
        (ReplyOrigin::ItemUse, Err(detail)) => send_item_use_error(world, job.actor, detail),
    }
}

fn send_item_use_error(world: &mut World, actor: Actor, detail: String) {
    if let Some(mut sender) = world.get_mut::<MessageSender<InventoryError>>(actor.connection) {
        sender.send::<InventoryChannel>(InventoryError {
            reason: InventoryErrorReason::CantDoRightNow,
            detail: Some(detail),
        });
    }
}
