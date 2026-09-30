use super::*;
use std::collections::HashMap;

#[cfg(test)]
mod tests;

#[derive(Resource, Default)]
struct MerchantState {
    vendor: Option<VendorInventory>,
    inventory_ready: bool,
    slots: HashMap<ItemLocation, ItemStack>,
    expected_count: Option<u64>,
}

impl MerchantState {
    fn count(&self, item_id: u32) -> u64 {
        self.slots
            .iter()
            .filter(|(location, item)| {
                matches!(location, ItemLocation::Bag { .. }) && item.item_id == item_id
            })
            .map(|(_, item)| u64::from(item.count))
            .sum()
    }

    fn snapshot(&mut self, snapshot: InventorySnapshot) {
        self.slots.clear();
        for bag in snapshot.bags {
            for slot in bag.items {
                self.slots.insert(
                    ItemLocation::Bag {
                        bag: bag.bag,
                        slot: slot.slot,
                    },
                    slot.item,
                );
            }
        }
        self.inventory_ready = true;
    }

    fn delta(&mut self, delta: InventoryDelta) {
        for change in delta.changes {
            match change.item {
                Some(item) => {
                    self.slots.insert(change.location, item);
                }
                None => {
                    self.slots.remove(&change.location);
                }
            }
        }
    }
}

#[derive(SystemParam)]
struct Senders<'w, 's> {
    interact: Query<'w, 's, &'static mut MessageSender<InteractNpc>>,
    gossip: Query<'w, 's, &'static mut MessageSender<SelectGossipOption>>,
    open: Query<'w, 's, &'static mut MessageSender<OpenAuctionHouse>>,
    search: Query<'w, 's, &'static mut MessageSender<QueryAuctions>>,
    inventory: Query<'w, 's, &'static mut MessageSender<QueryAuctionInventory>>,
    owned: Query<'w, 's, &'static mut MessageSender<QueryOwnedAuctions>>,
    bids: Query<'w, 's, &'static mut MessageSender<QueryBidAuctions>>,
    create: Query<'w, 's, &'static mut MessageSender<CreateAuction>>,
    bid: Query<'w, 's, &'static mut MessageSender<PlaceBid>>,
    buyout: Query<'w, 's, &'static mut MessageSender<BuyoutAuction>>,
    cancel: Query<'w, 's, &'static mut MessageSender<CancelAuction>>,
    buy: Query<'w, 's, &'static mut MessageSender<BuyItem>>,
}

#[derive(SystemParam)]
struct Receivers<'w, 's> {
    opened: Query<'w, 's, &'static mut MessageReceiver<InteractionOpened>>,
    failed: Query<'w, 's, &'static mut MessageReceiver<InteractionFailed>>,
    closed: Query<'w, 's, &'static mut MessageReceiver<InteractionClosed>>,
    house: Query<'w, 's, &'static mut MessageReceiver<AuctionHouseOpened>>,
    search: Query<'w, 's, &'static mut MessageReceiver<AuctionSearchResults>>,
    inventory: Query<'w, 's, &'static mut MessageReceiver<AuctionInventorySnapshot>>,
    owned: Query<'w, 's, &'static mut MessageReceiver<OwnedAuctionListResponse>>,
    bids: Query<'w, 's, &'static mut MessageReceiver<BidAuctionListResponse>>,
    operation: Query<'w, 's, &'static mut MessageReceiver<AuctionOperationResponse>>,
    vendor: Query<'w, 's, &'static mut MessageReceiver<VendorInventory>>,
    merchant_failed: Query<'w, 's, &'static mut MessageReceiver<MerchantFailed>>,
    bag_snapshot: Query<'w, 's, &'static mut MessageReceiver<InventorySnapshot>>,
    bag_delta: Query<'w, 's, &'static mut MessageReceiver<InventoryDelta>>,
}

pub(crate) fn register(app: &mut App) {
    app.init_resource::<EconomyFlow>();
    app.init_resource::<MerchantState>();
    app.add_systems(Update, economy_system);
}

fn send<M: lightyear::prelude::Message + Clone, C: Channel>(
    senders: &mut Query<&mut MessageSender<M>>,
    message: M,
) -> Result<(), String> {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<C>(message.clone());
        sent = true;
    }
    if sent {
        Ok(())
    } else {
        Err(format!("No sender for {}", std::any::type_name::<M>()))
    }
}

fn finish(
    flow: &mut EconomyFlow,
    exit: &mut MessageWriter<AppExit>,
    result: Result<String, String>,
) {
    if flow.phase == Phase::Done {
        return;
    }
    match result {
        Ok(message) => {
            println!("[economy] {message}");
            exit.write(AppExit::Success);
        }
        Err(error) => {
            eprintln!("[economy] Failed: {error}");
            exit.write(AppExit::Error(1.try_into().unwrap()));
        }
    }
    flow.phase = Phase::Done;
}

fn economy_system(
    state: Res<State<CliState>>,
    action: Res<CliAction>,
    mut flow: ResMut<EconomyFlow>,
    mut merchant: ResMut<MerchantState>,
    mut senders: Senders,
    mut receivers: Receivers,
    mut exit: MessageWriter<AppExit>,
) {
    let CliCommand::Economy(action) = &action.command else {
        return;
    };
    if flow.phase == Phase::Done {
        return;
    }
    // Inventory and vendor responses use different channels from interaction.
    // Retain them even if they arrive before InteractionOpened.
    for mut receiver in &mut receivers.bag_snapshot {
        for snapshot in receiver.receive() {
            merchant.snapshot(snapshot);
        }
    }
    for mut receiver in &mut receivers.bag_delta {
        for delta in receiver.receive() {
            merchant.delta(delta);
        }
    }
    for mut receiver in &mut receivers.vendor {
        for inventory in receiver.receive() {
            if inventory.npc == action.npc {
                merchant.vendor = Some(inventory);
            }
        }
    }
    if *state.get() != CliState::InWorld {
        return;
    }
    if let Some(request) = flow.begin(action) {
        if let Err(error) = send::<_, InteractionChannel>(&mut senders.interact, request) {
            finish(&mut flow, &mut exit, Err(error));
            return;
        }
    }
    receive_interaction(action, &mut flow, &mut senders, &mut receivers, &mut exit);
    receive_auction(action, &mut flow, &mut receivers, &mut exit);
    receive_merchant_failure(action, &mut flow, &mut receivers, &mut exit);
    if flow.phase == Phase::Done {
        return;
    }
    if flow.phase == Phase::Opening && !flow.open_sent {
        match send::<_, AuctionChannel>(&mut senders.open, OpenAuctionHouse) {
            Ok(()) => {
                flow.open_sent = true;
            }
            Err(error) => finish(&mut flow, &mut exit, Err(error)),
        }
    }
    if flow.phase == Phase::Request && matches!(action.operation, EconomyOperation::MerchantList) {
        if let Some(vendor) = &merchant.vendor {
            finish(&mut flow, &mut exit, Ok(format!("merchant {vendor:#?}")));
            return;
        }
    }
    if flow.phase == Phase::Request {
        match dispatch_request(action, &mut merchant, &mut senders) {
            Ok(true) => {
                flow.phase = Phase::Awaiting;
                flow.started = std::time::Instant::now();
            }
            Ok(false) => {}
            Err(error) => finish(&mut flow, &mut exit, Err(error)),
        }
    }
    if let EconomyOperation::MerchantBuy { item_id, .. } = action.operation {
        if flow.phase == Phase::Awaiting
            && merchant
                .expected_count
                .is_some_and(|expected| merchant.count(item_id) >= expected)
        {
            finish(
                &mut flow,
                &mut exit,
                Ok(format!(
                    "purchase confirmed by inventory: item_id={item_id} count={}",
                    merchant.count(item_id)
                )),
            );
        }
    }
    if let Some(error) = flow.timeout(flow.started.elapsed()) {
        finish(&mut flow, &mut exit, Err(error));
    }
}

fn receive_interaction(
    action: &EconomyAction,
    flow: &mut EconomyFlow,
    senders: &mut Senders,
    receivers: &mut Receivers,
    exit: &mut MessageWriter<AppExit>,
) {
    for mut receiver in &mut receivers.failed {
        for response in receiver.receive() {
            if response.npc == action.npc {
                finish(flow, exit, Err(response.error.message().into()));
            }
        }
    }
    for mut receiver in &mut receivers.closed {
        for response in receiver.receive() {
            if response.npc == action.npc {
                finish(flow, exit, Err("NPC interaction closed".into()));
            }
        }
    }
    for mut receiver in &mut receivers.opened {
        for response in receiver.receive() {
            match flow.opened(action, response) {
                Ok(Some(choice)) => {
                    if let Err(error) = send::<_, InteractionChannel>(&mut senders.gossip, choice) {
                        finish(flow, exit, Err(error));
                    }
                }
                Ok(None) => {}
                Err(error) => finish(flow, exit, Err(error)),
            }
        }
    }
}

fn receive_auction(
    action: &EconomyAction,
    flow: &mut EconomyFlow,
    receivers: &mut Receivers,
    exit: &mut MessageWriter<AppExit>,
) {
    if action.operation.is_merchant() {
        return;
    }
    for mut receiver in &mut receivers.house {
        for response in receiver.receive() {
            if flow.phase != Phase::Opening {
                continue;
            }
            match flow.house_opened(response) {
                Err(error) => finish(flow, exit, Err(error)),
                Ok(()) if matches!(action.operation, EconomyOperation::Open) => {
                    finish(flow, exit, Ok("auction house opened".into()))
                }
                Ok(()) => {}
            }
        }
    }
    for mut receiver in &mut receivers.operation {
        for response in receiver.receive() {
            if !response.success {
                finish(flow, exit, Err(response.message));
            } else if flow.phase == Phase::Awaiting
                && matches!(
                    action.operation,
                    EconomyOperation::Create(_)
                        | EconomyOperation::Bid(_)
                        | EconomyOperation::Buyout(_)
                        | EconomyOperation::Cancel(_)
                )
            {
                finish(flow, exit, Ok(response.message));
            }
        }
    }
    for mut receiver in &mut receivers.search {
        for response in receiver.receive() {
            if flow.phase == Phase::Awaiting
                && matches!(action.operation, EconomyOperation::Search(_))
            {
                let elapsed = flow.started.elapsed().as_secs_f64() * 1000.0;
                finish(
                    flow,
                    exit,
                    Ok(format!(
                        "search request_response_ms={elapsed:.3} page={} page_size={} total_results={} returned={} query={:?}\n{:#?}",
                        response.query.page,
                        response.query.page_size,
                        response.total_results,
                        response.results.len(),
                        response.query,
                        response.results
                    )),
                );
            }
        }
    }
    for mut receiver in &mut receivers.inventory {
        for response in receiver.receive() {
            if flow.phase == Phase::Awaiting
                && matches!(action.operation, EconomyOperation::Inventory)
            {
                finish(flow, exit, Ok(format!("inventory {response:#?}")));
            }
        }
    }
    for mut receiver in &mut receivers.owned {
        for response in receiver.receive() {
            if flow.phase == Phase::Awaiting && matches!(action.operation, EconomyOperation::Owned)
            {
                finish(flow, exit, Ok(format!("owned {response:#?}")));
            }
        }
    }
    for mut receiver in &mut receivers.bids {
        for response in receiver.receive() {
            if flow.phase == Phase::Awaiting && matches!(action.operation, EconomyOperation::Bids) {
                finish(flow, exit, Ok(format!("bids {response:#?}")));
            }
        }
    }
}

fn receive_merchant_failure(
    action: &EconomyAction,
    flow: &mut EconomyFlow,
    receivers: &mut Receivers,
    exit: &mut MessageWriter<AppExit>,
) {
    for mut receiver in &mut receivers.merchant_failed {
        for response in receiver.receive() {
            if response.npc == action.npc && action.operation.is_merchant() {
                finish(flow, exit, Err(response.error.message().into()));
            }
        }
    }
}

fn dispatch_request(
    action: &EconomyAction,
    merchant: &mut MerchantState,
    senders: &mut Senders,
) -> Result<bool, String> {
    match &action.operation {
        EconomyOperation::Open => return Ok(false),
        EconomyOperation::Search(query) => send::<_, AuctionChannel>(
            &mut senders.search,
            QueryAuctions {
                query: query.clone(),
            },
        )?,
        EconomyOperation::Inventory => {
            send::<_, AuctionChannel>(&mut senders.inventory, QueryAuctionInventory)?
        }
        EconomyOperation::Owned => {
            send::<_, AuctionChannel>(&mut senders.owned, QueryOwnedAuctions)?
        }
        EconomyOperation::Bids => send::<_, AuctionChannel>(&mut senders.bids, QueryBidAuctions)?,
        EconomyOperation::Create(request) => {
            send::<_, AuctionChannel>(&mut senders.create, request.clone())?
        }
        EconomyOperation::Bid(request) => {
            send::<_, AuctionChannel>(&mut senders.bid, request.clone())?
        }
        EconomyOperation::Buyout(request) => {
            send::<_, AuctionChannel>(&mut senders.buyout, request.clone())?
        }
        EconomyOperation::Cancel(request) => {
            send::<_, AuctionChannel>(&mut senders.cancel, request.clone())?
        }
        EconomyOperation::MerchantList => return Ok(false),
        EconomyOperation::MerchantBuy {
            slot,
            item_id,
            count,
        } => {
            let Some(vendor) = &merchant.vendor else {
                return Ok(false);
            };
            if !merchant.inventory_ready {
                return Ok(false);
            }
            let item = vendor
                .items
                .iter()
                .find(|item| item.slot == *slot && item.item_id == *item_id)
                .ok_or_else(|| format!("Vendor does not offer item {item_id} in slot {slot}"))?;
            merchant.expected_count =
                Some(merchant.count(*item_id) + u64::from(item.stack_count) * u64::from(*count));
            send::<_, MerchantChannel>(
                &mut senders.buy,
                BuyItem {
                    npc: action.npc,
                    slot: *slot,
                    item_id: *item_id,
                    count: *count,
                    destination: None,
                },
            )?;
        }
    }
    Ok(true)
}
