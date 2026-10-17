//! Owned Trading Post actions run on the same FIFO as Guide contributions.
use super::*;

pub(super) struct OwnedOffer {
    vendor: u32,
    price: u32,
    grant: store::Grant,
    refund_duration: u32,
}
impl OwnedOffer {
    fn borrowed(&self) -> store::Offer<'_> {
        store::Offer {
            vendor: self.vendor,
            price: self.price,
            grant: &self.grant,
            refund_duration: self.refund_duration,
        }
    }
}

pub(super) enum Action {
    Read,
    Purchase(OwnedOffer),
    Checkout(Vec<OwnedOffer>),
    Refund(u32),
    Freeze(Option<u32>),
}

fn own_offers(
    catalog: &catalog::CatalogData,
    vendors: &[u32],
    refund_duration: u32,
) -> Result<Vec<OwnedOffer>, String> {
    vendors
        .iter()
        .map(|vendor| {
            let (row, grant) = catalog
                .items
                .get(vendor)
                .ok_or("Unknown Trading Post item.")?;
            Ok(OwnedOffer {
                vendor: *vendor,
                price: row.price,
                grant: grant.as_ref().map_err(Clone::clone)?.clone(),
                refund_duration,
            })
        })
        .collect()
}

pub(super) fn prepare_action(
    world: &World,
    connection: Entity,
    player: Entity,
    request: &TradingPostRequest,
) -> Result<Action, String> {
    if matches!(
        request,
        TradingPostRequest::Query | TradingPostRequest::Balance
    ) {
        return Ok(Action::Read);
    }
    active_vendor(world, connection, player)?;
    let catalog = world
        .resource::<catalog::Catalog>()
        .0
        .as_ref()
        .map_err(Clone::clone)?;
    let duration = world
        .get_resource::<RefundWindow>()
        .map_or(0, |window| window.0);
    match request {
        TradingPostRequest::Query | TradingPostRequest::Balance => Ok(Action::Read),
        TradingPostRequest::Checkout { vendor_item_ids } => Ok(Action::Checkout(own_offers(
            catalog,
            vendor_item_ids,
            duration,
        )?)),
        TradingPostRequest::Purchase { vendor_item_id } => {
            let offer = own_offers(catalog, &[*vendor_item_id], duration)?.remove(0);
            Ok(Action::Purchase(offer))
        }
        TradingPostRequest::Refund { vendor_item_id } => Ok(Action::Refund(*vendor_item_id)),
        TradingPostRequest::Freeze { vendor_item_id } => {
            if let Some(id) = vendor_item_id {
                let (_, grant) = catalog
                    .items
                    .get(id)
                    .ok_or("Unknown frozen Trading Post item.")?;
                grant.as_ref().map_err(Clone::clone)?;
            }
            Ok(Action::Freeze(*vendor_item_id))
        }
    }
}

pub(super) fn execute_action(
    db: &redb::Database,
    account: u64,
    action: Action,
    now: i64,
) -> Result<Vec<(u32, store::Grant)>, String> {
    match action {
        Action::Read => Ok(Vec::new()),
        Action::Purchase(offer) => {
            store::purchase(db, account, offer.borrowed(), now)?;
            Ok(vec![(offer.vendor, offer.grant)])
        }
        Action::Checkout(offers) => {
            let borrowed: Vec<_> = offers.iter().map(OwnedOffer::borrowed).collect();
            let state = store::checkout(db, account, &borrowed, now)?;
            Ok(offers
                .iter()
                .map(|offer| (offer.vendor, state.purchases[&offer.vendor].granted.clone()))
                .collect())
        }
        Action::Refund(vendor) => {
            let state = store::load(db, account);
            let grant = state
                .purchases
                .get(&vendor)
                .ok_or("Trading Post receipt unavailable.")?
                .granted
                .clone();
            store::refund(db, account, vendor, now)?;
            Ok(vec![(vendor, grant)])
        }
        Action::Freeze(vendor) => {
            store::freeze(db, account, vendor);
            Ok(Vec::new())
        }
    }
}

#[derive(Clone)]
struct Completed {
    snapshot: TradingPostSnapshot,
    changed: Vec<(u32, store::Grant)>,
}
struct PendingReply {
    account: u64,
    receipt: crate::persistence_writer::CommitReceipt<Completed>,
    failed_snapshot: TradingPostSnapshot,
}
#[derive(Component, Default)]
struct PendingReplies(std::collections::VecDeque<PendingReply>);

fn requested_rows(request: &TradingPostRequest) -> Rows {
    match request {
        TradingPostRequest::Query => Rows::Catalog,
        TradingPostRequest::Purchase { vendor_item_id }
        | TradingPostRequest::Refund { vendor_item_id } => Rows::AffectedBy(vec![*vendor_item_id]),
        TradingPostRequest::Checkout { vendor_item_ids } => {
            Rows::AffectedBy(vendor_item_ids.clone())
        }
        _ => Rows::None,
    }
}

pub(super) fn queue_request(
    world: &mut World,
    connection: Entity,
    account: u64,
    player: Entity,
    request: &TradingPostRequest,
) {
    let action = prepare_action(world, connection, player, request);
    let rows = requested_rows(request);
    let (snapshot, catalog, month) = prepare_snapshot(world, connection, &rows);
    let failed_snapshot = snapshot.clone();
    let settlement = world
        .get::<crate::adventure_guide::MonthSettlement>(connection)
        .cloned();
    let receipt = world.resource::<GameDatabase>().writer.enqueue_result(
        "Trading Post request and snapshot settlement",
        move |db| {
            if let Some(settlement) = settlement {
                settlement
                    .0
                    .result()
                    .expect("preceding FIFO settlement completed")?;
            }
            execute_and_snapshot(db, account, action, snapshot, catalog, month)
        },
    );
    world
        .entity_mut(connection)
        .entry::<PendingReplies>()
        .or_default()
        .into_mut()
        .0
        .push_back(PendingReply {
            account,
            receipt,
            failed_snapshot,
        });
}

fn execute_and_snapshot(
    db: &redb::Database,
    account: u64,
    action: Result<Action, String>,
    mut snapshot: TradingPostSnapshot,
    catalog: Option<catalog::CatalogData>,
    month: i64,
) -> Result<Completed, String> {
    let result = action.and_then(|action| execute_action(db, account, action, snapshot.now_unix));
    // The reply's rollover settlement is ordered too; never commit it on the tick.
    let state = store::settle(db, account, month, 0, &[]);
    snapshot.tender = state.tender;
    snapshot.frozen_item_id = state.frozen;
    for threshold in &mut snapshot.thresholds {
        threshold.claimed = state.claimed.contains(&threshold.id);
    }
    let changed = match result {
        Ok(changed) => changed,
        Err(error) => {
            snapshot.error = Some(error);
            Vec::new()
        }
    };
    if (snapshot.catalog_complete || !changed.is_empty())
        && let Some(catalog) = catalog
    {
        snapshot.items = collection_rows(db, account, &catalog, &state, snapshot.now_unix);
    }
    Ok(Completed { snapshot, changed })
}

pub(super) fn flush_replies(world: &mut World) {
    let mut completed = Vec::new();
    for (connection, mut pending) in world
        .query::<(Entity, &mut PendingReplies)>()
        .iter_mut(world)
    {
        while let Some(front) = pending.0.front() {
            let Some(result) = front.receipt.result() else {
                break;
            };
            let request = pending.0.pop_front().expect("completed FIFO reply");
            let reply = match result {
                Ok(reply) => reply,
                Err(error) => {
                    let mut snapshot = request.failed_snapshot;
                    snapshot.error = Some(format!("Trading Post persistence failed: {error}"));
                    Completed {
                        snapshot,
                        changed: Vec::new(),
                    }
                }
            };
            completed.push((connection, request.account, reply));
        }
    }
    for (connection, account, reply) in completed {
        let same_account = world
            .get::<Authenticated>(connection)
            .is_some_and(|auth| auth.account_id == account);
        if !same_account || world.get::<InWorld>(connection).is_none() {
            continue;
        }
        publish_collections(world, account, &reply.changed);
        if let Some(mut sender) = world.get_mut::<MessageSender<TradingPostSnapshot>>(connection) {
            sender.send::<InstanceChannel>(reply.snapshot);
        } else {
            error!(?connection, "TP_DELIVERY_SERVER_MISSING_SENDER");
        }
    }
}
