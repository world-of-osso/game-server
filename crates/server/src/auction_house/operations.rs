use super::*;

/// Deposit = vendor_sell_price * stack_count * duration_multiplier.
pub(crate) fn deposit_cost(
    vendor_sell_price: u32,
    stack_count: u32,
    duration: AuctionDuration,
) -> u64 {
    let multiplier = match duration {
        AuctionDuration::Short => 1,
        AuctionDuration::Medium => 2,
        AuctionDuration::Long => 4,
    };
    u64::from(vendor_sell_price)
        .saturating_mul(u64::from(stack_count))
        .saturating_mul(multiplier)
}

/// Validated inputs for creating an auction.
struct ValidatedAuction {
    character_name: String,
    source: InventoryItemData,
    item_index: usize,
    deposit: u64,
}

fn validate_auction_request(req: &CreateAuction) -> Result<(), AuctionOperationResponse> {
    if req.stack_count == 0 {
        return Err(err("stack count must be greater than zero"));
    }
    if req.min_bid == 0 {
        return Err(err("minimum bid must be greater than zero"));
    }
    if let Some(buyout) = req.buyout_price
        && buyout < req.min_bid
    {
        return Err(err("buyout must be greater than or equal to minimum bid"));
    }
    Ok(())
}

fn validate_create_auction(
    db: &redb::Database,
    state: &AuctionHouseState,
    character_id: u64,
    req: &CreateAuction,
) -> Result<ValidatedAuction, AuctionOperationResponse> {
    validate_auction_request(req)?;
    validate_auction_slot_limit(state, character_id)?;
    let Some(character) = persistence::load_character(db, character_id) else {
        return Err(err("character not found"));
    };
    let inventory = persistence::load_inventory(db, character_id);
    let (index, source) = find_auction_source_item(&inventory, req)?;
    if req.stack_count > source.stack_count {
        return Err(err("requested stack exceeds available stack"));
    }
    validate_unique_auction_listing(state, &source)?;
    let deposit = deposit_cost(source.vendor_sell_price, req.stack_count, req.duration);
    validate_auction_deposit(&inventory, deposit)?;
    Ok(ValidatedAuction {
        character_name: character.name,
        source,
        item_index: index,
        deposit,
    })
}

fn validate_auction_slot_limit(
    state: &AuctionHouseState,
    character_id: u64,
) -> Result<(), AuctionOperationResponse> {
    let active_count = state
        .auctions
        .values()
        .filter(|auction| auction.seller_character_id == character_id)
        .count();
    if active_count >= MAX_AUCTIONS_PER_CHARACTER {
        return Err(err("maximum auction slots reached"));
    }
    Ok(())
}

fn find_auction_source_item(
    inventory: &InventoryData,
    req: &CreateAuction,
) -> Result<(usize, InventoryItemData), AuctionOperationResponse> {
    let Some(index) = inventory
        .items
        .iter()
        .position(|item| item.item_guid == req.item_guid)
    else {
        return Err(err("item not found in inventory"));
    };
    Ok((index, inventory.items[index].clone()))
}

fn validate_unique_auction_listing(
    state: &AuctionHouseState,
    source: &InventoryItemData,
) -> Result<(), AuctionOperationResponse> {
    if source.unique_auction
        && state
            .auctions
            .values()
            .any(|auction| auction.item.item_id == source.item_id)
    {
        return Err(err("unique item already listed on the auction house"));
    }
    Ok(())
}

fn validate_auction_deposit(
    inventory: &InventoryData,
    deposit: u64,
) -> Result<(), AuctionOperationResponse> {
    if deposit > 0 && inventory.gold < deposit {
        return Err(err("not enough gold for auction deposit"));
    }
    Ok(())
}

pub(super) fn create_auction(
    db: &redb::Database,
    state: &mut AuctionHouseState,
    character_id: u64,
    house: u8,
    req: &CreateAuction,
) -> AuctionOperationResponse {
    let validated = match validate_create_auction(db, state, character_id, req) {
        Ok(validated) => validated,
        Err(response) => return response,
    };

    let mut inventory = persistence::load_inventory(db, character_id);
    inventory.gold -= validated.deposit;
    if inventory.items[validated.item_index].stack_count == req.stack_count {
        inventory.items.remove(validated.item_index);
    } else {
        inventory.items[validated.item_index].stack_count -= req.stack_count;
    }
    persistence::save_inventory(db, character_id, &inventory);

    let auction = AuctionData {
        auction_id: persistence::next_id(db, "auction"),
        seller_character_id: character_id,
        seller_name: validated.character_name,
        item: InventoryItemData {
            item_guid: persistence::next_id(db, "item_instance"),
            stack_count: req.stack_count,
            ..validated.source
        },
        stack_count: req.stack_count,
        min_bid: req.min_bid,
        current_bid: None,
        current_bidder_character_id: None,
        buyout_price: req.buyout_price,
        expires_at_unix_secs: unix_now() + duration_secs(req.duration),
        faction: house,
        deposit: validated.deposit,
    };
    persistence::save_auction(db, &auction);
    state.auctions.insert(auction.auction_id, auction);
    ok("auction created")
}

/// Gold already held by the auction house from a previous bid by this character.
fn previous_bid_credit(auction: &AuctionData, character_id: u64) -> u64 {
    if auction.current_bidder_character_id == Some(character_id) {
        auction.current_bid.unwrap_or(0)
    } else {
        0
    }
}

/// Refund the previous bidder's gold and send outbid notification mail.
fn refund_previous_bidder(db: &redb::Database, auction: &AuctionData, new_bidder: u64) {
    let Some(prev_bidder) = auction.current_bidder_character_id else {
        return;
    };
    if prev_bidder == new_bidder {
        return;
    }
    let refund = auction.current_bid.unwrap_or(0);
    let mut prev_inventory = persistence::load_inventory(db, prev_bidder);
    prev_inventory.gold = prev_inventory.gold.saturating_add(refund);
    persistence::save_inventory(db, prev_bidder, &prev_inventory);
    deliver_mail(
        db,
        prev_bidder,
        "Outbid",
        &format!(
            "You have been outbid on {}. Your {} gold has been refunded.",
            auction.item.name, refund
        ),
        0,
        None,
    );
}

pub(super) fn place_bid(
    db: &redb::Database,
    state: &mut AuctionHouseState,
    character_id: u64,
    auction_id: u64,
    amount: u64,
) -> AuctionOperationResponse {
    let Some(auction) = state.auctions.get_mut(&auction_id) else {
        return err("auction not found");
    };
    if auction.seller_character_id == character_id {
        return err("cannot bid on your own auction");
    }
    let min_next = min_next_bid(auction);
    if amount < min_next {
        return err("bid is below the minimum next bid");
    }
    let mut bidder_inventory = persistence::load_inventory(db, character_id);
    let credit = previous_bid_credit(auction, character_id);
    let additional = amount.saturating_sub(credit);
    if bidder_inventory.gold < additional {
        return err("not enough gold");
    }
    bidder_inventory.gold -= additional;
    persistence::save_inventory(db, character_id, &bidder_inventory);

    refund_previous_bidder(db, auction, character_id);

    auction.current_bid = Some(amount);
    auction.current_bidder_character_id = Some(character_id);

    if let Some(buyout) = auction.buyout_price
        && amount >= buyout
    {
        let auction = state.auctions.remove(&auction_id).expect("auction present");
        persistence::delete_auction(db, auction_id);
        complete_sale(db, &auction, character_id, amount);
        return ok("bid met buyout — auction sold");
    }

    persistence::save_auction(db, auction);
    ok("bid placed")
}

pub(super) fn buyout_auction(
    db: &redb::Database,
    state: &mut AuctionHouseState,
    character_id: u64,
    auction_id: u64,
) -> AuctionOperationResponse {
    let Some(auction) = state.auctions.remove(&auction_id) else {
        return err("auction not found");
    };
    if auction.seller_character_id == character_id {
        state.auctions.insert(auction_id, auction);
        return err("cannot buy out your own auction");
    }
    let Some(price) = auction.buyout_price else {
        state.auctions.insert(auction_id, auction);
        return err("auction has no buyout price");
    };

    let mut buyer_inventory = persistence::load_inventory(db, character_id);
    let credit = previous_bid_credit(&auction, character_id);
    let additional = price.saturating_sub(credit);
    if buyer_inventory.gold < additional {
        state.auctions.insert(auction_id, auction);
        return err("not enough gold");
    }
    buyer_inventory.gold -= additional;
    persistence::save_inventory(db, character_id, &buyer_inventory);

    if let Some(prev_bidder) = auction.current_bidder_character_id
        && prev_bidder != character_id
    {
        let mut prev_inventory = persistence::load_inventory(db, prev_bidder);
        prev_inventory.gold = prev_inventory
            .gold
            .saturating_add(auction.current_bid.unwrap_or(0));
        persistence::save_inventory(db, prev_bidder, &prev_inventory);
    }

    persistence::delete_auction(db, auction_id);
    complete_sale(db, &auction, character_id, price);
    ok("auction bought out")
}

pub(super) fn cancel_auction(
    db: &redb::Database,
    state: &mut AuctionHouseState,
    character_id: u64,
    auction_id: u64,
) -> AuctionOperationResponse {
    let Some(auction) = state.auctions.get(&auction_id) else {
        return err("auction not found");
    };
    if auction.seller_character_id != character_id {
        return err("cannot cancel another player's auction");
    }
    if auction.current_bidder_character_id.is_some() {
        return err("cannot cancel an auction that already has bids");
    }
    let auction = state.auctions.remove(&auction_id).expect("auction present");
    persistence::delete_auction(db, auction_id);
    deliver_mail(
        db,
        character_id,
        "Auction cancelled",
        &format!("Your auction for {} was cancelled.", auction.item.name),
        0,
        Some(auction.item),
    );
    ok("auction cancelled")
}

pub(super) fn complete_sale(
    db: &redb::Database,
    auction: &AuctionData,
    buyer_character_id: u64,
    sale_price: u64,
) {
    let house_fee = sale_price.saturating_mul(HOUSE_CUT_BPS) / 10_000;
    let seller_proceeds = sale_price
        .saturating_sub(house_fee)
        .saturating_add(auction.deposit);
    deliver_mail(
        db,
        auction.seller_character_id,
        "Auction sold",
        &format!("Your auction for {} sold.", auction.item.name),
        seller_proceeds,
        None,
    );
    deliver_mail(
        db,
        buyer_character_id,
        "Auction won",
        &format!("You won {}.", auction.item.name),
        0,
        Some(auction.item.clone()),
    );
}

/// Instant Auction House mail (AzerothCore `MailDraft` from `MAIL_AUCTION`).
pub(super) fn deliver_mail(
    db: &redb::Database,
    character_id: u64,
    subject: &str,
    body: &str,
    money: u64,
    item: Option<InventoryItemData>,
) {
    let now = unix_now();
    mail_store::deliver_mail(
        db,
        character_id,
        MailData {
            mail_id: persistence::next_id(db, "mail"),
            sender: MailSender::AuctionHouse,
            subject: subject.to_string(),
            body: body.to_string(),
            items: item
                .into_iter()
                .map(|item| MailItemData { slot: 0, item })
                .collect(),
            money,
            cod: 0,
            delivers_at: now,
            expires_at: now + shared::mail::MAIL_EXPIRY_SECS,
            read: false,
            returned: false,
            cod_payment: false,
        },
    );
}
