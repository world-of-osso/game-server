use super::*;
use crate::economy::store::{MarketChanges, commit_market};

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
    inventory: InventoryData,
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
    templates: &ItemTemplates,
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
    validate_auction_binding(db, templates, &source)?;
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
        inventory,
    })
}

fn validate_auction_binding(
    db: &redb::Database,
    templates: &ItemTemplates,
    source: &InventoryItemData,
) -> Result<(), AuctionOperationResponse> {
    let definition_source =
        persistence::inventory_store::load_item_definition_source(db, source.item_guid);
    let binds_on_pickup = templates
        .get(definition_source, source.item_id)
        .is_some_and(ItemTemplate::binds_on_pickup);
    let bound =
        persistence::inventory_store::soulbound_guids(db, std::iter::once(source.item_guid));
    if binds_on_pickup || bound.contains(&source.item_guid) {
        return Err(err("cannot auction a soulbound item"));
    }
    Ok(())
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
    templates: &ItemTemplates,
    state: &mut AuctionHouseState,
    character_id: u64,
    house: u8,
    req: &CreateAuction,
) -> AuctionOperationResponse {
    let validated = match validate_create_auction(db, templates, state, character_id, req) {
        Ok(validated) => validated,
        Err(response) => return response,
    };

    let mut inventory = validated.inventory.clone();
    inventory.gold -= validated.deposit;
    if inventory.items[validated.item_index].stack_count == req.stack_count {
        inventory.items.remove(validated.item_index);
    } else {
        inventory.items[validated.item_index].stack_count -= req.stack_count;
    }
    let source =
        persistence::inventory_store::load_item_definition_source(db, validated.source.item_guid);
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
    let changes = MarketChanges {
        auctions: vec![auction.clone()],
        definition_sources: vec![(auction.item.item_guid, source)],
        expected_inventories: vec![(character_id, validated.inventory)],
        inventories: vec![(character_id, inventory)],
        ..Default::default()
    };
    if let Err(error) = commit_market(db, None, &changes) {
        return err(&error);
    }
    state.auctions.insert(auction.auction_id, auction);
    state.pending_criteria.push((
        character_id,
        crate::achievement::CriteriaEvent::AuctionPosted,
    ));
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

/// Stage the refund alongside the new bid; never publish money before settlement.
fn stage_previous_bidder_refund(
    db: &redb::Database,
    auction: &AuctionData,
    new_bidder: u64,
    changes: &mut MarketChanges,
) {
    let Some(previous_bidder) = auction.current_bidder_character_id else {
        return;
    };
    if previous_bidder == new_bidder {
        return;
    }
    let refund = auction.current_bid.unwrap_or(0);
    let mut inventory = persistence::load_inventory(db, previous_bidder);
    changes
        .expected_inventories
        .push((previous_bidder, inventory.clone()));
    inventory.gold = inventory.gold.saturating_add(refund);
    changes.inventories.push((previous_bidder, inventory));
    changes.mails.push((
        previous_bidder,
        build_auction_mail(
            persistence::next_id(db, "mail"),
            &auction_mail_subject("AUCTION_OUTBID_MAIL_SUBJECT", auction),
            "",
            0,
            None,
        ),
    ));
}

pub(super) fn place_bid(
    db: &redb::Database,
    state: &mut AuctionHouseState,
    character_id: u64,
    auction_id: u64,
    amount: u64,
) -> AuctionOperationResponse {
    let Some(auction) = state.auctions.get(&auction_id) else {
        return err("auction not found");
    };
    if auction.expires_at_unix_secs <= unix_now() {
        return err("auction has expired");
    }
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
    let expected_inventory = bidder_inventory.clone();
    bidder_inventory.gold -= additional;
    let mut changes = MarketChanges {
        expected_auctions: vec![(auction_id, auction.clone())],
        expected_inventories: vec![(character_id, expected_inventory)],
        inventories: vec![(character_id, bidder_inventory)],
        ..Default::default()
    };
    stage_previous_bidder_refund(db, auction, character_id, &mut changes);
    let sold = auction.buyout_price.is_some_and(|buyout| amount >= buyout);
    let mut updated = auction.clone();
    updated.current_bid = Some(amount);
    updated.current_bidder_character_id = Some(character_id);
    if sold {
        changes.deleted_auctions.push(auction_id);
        changes
            .mails
            .extend(sale_mails(db, auction, character_id, amount));
    } else {
        changes.auctions.push(updated.clone());
    }
    if let Err(error) = commit_market(db, None, &changes) {
        return err(&error);
    }
    state.pending_criteria.push((
        character_id,
        crate::achievement::CriteriaEvent::HighestAuctionBid(amount),
    ));
    if sold {
        state
            .pending_criteria
            .extend(auction_sale_events(&updated, Some(character_id), amount));
        state.auctions.remove(&auction_id);
        ok("bid met buyout — auction sold")
    } else {
        state.auctions.insert(auction_id, updated);
        ok("bid placed")
    }
}

pub(super) fn buyout_auction(
    db: &redb::Database,
    state: &mut AuctionHouseState,
    character_id: u64,
    auction_id: u64,
) -> AuctionOperationResponse {
    let Some(auction) = state.auctions.get(&auction_id) else {
        return err("auction not found");
    };
    if auction.expires_at_unix_secs <= unix_now() {
        return err("auction has expired");
    }
    if auction.seller_character_id == character_id {
        return err("cannot buy out your own auction");
    }
    let Some(price) = auction.buyout_price else {
        return err("auction has no buyout price");
    };

    let mut buyer_inventory = persistence::load_inventory(db, character_id);
    let credit = previous_bid_credit(auction, character_id);
    let additional = price.saturating_sub(credit);
    if buyer_inventory.gold < additional {
        return err("not enough gold");
    }
    let expected_inventory = buyer_inventory.clone();
    buyer_inventory.gold -= additional;
    let mut changes = MarketChanges {
        expected_auctions: vec![(auction_id, auction.clone())],
        deleted_auctions: vec![auction_id],
        expected_inventories: vec![(character_id, expected_inventory)],
        inventories: vec![(character_id, buyer_inventory)],
        mails: sale_mails(db, auction, character_id, price),
        ..Default::default()
    };
    stage_previous_bidder_refund(db, auction, character_id, &mut changes);
    let credits = auction_sale_events(auction, Some(character_id), price);
    if let Err(error) = commit_market(db, None, &changes) {
        return err(&error);
    }
    state.pending_criteria.extend(credits);
    state.pending_criteria.push((
        character_id,
        crate::achievement::CriteriaEvent::HighestAuctionBid(price),
    ));
    state.auctions.remove(&auction_id);
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
    let changes = MarketChanges {
        expected_auctions: vec![(auction_id, auction.clone())],
        deleted_auctions: vec![auction_id],
        mails: vec![(
            character_id,
            build_auction_mail(
                persistence::next_id(db, "mail"),
                &auction_mail_subject("AUCTION_REMOVED_MAIL_SUBJECT", auction),
                "",
                0,
                Some(auction.item.clone()),
            ),
        )],
        ..Default::default()
    };
    if let Err(error) = commit_market(db, None, &changes) {
        return err(&error);
    }
    state.auctions.remove(&auction_id);
    ok("auction cancelled")
}

/// TC item-style settlement: seller profit includes deposit less cut; highest sale is gross bid.
pub(crate) fn auction_sale_events(
    auction: &AuctionData,
    buyer: Option<u64>,
    sale_price: u64,
) -> Vec<(u64, crate::achievement::CriteriaEvent)> {
    use crate::achievement::CriteriaEvent;
    let mut events = vec![
        (
            auction.seller_character_id,
            CriteriaEvent::MoneyEarnedFromAuctions(seller_proceeds(auction, sale_price)),
        ),
        (
            auction.seller_character_id,
            CriteriaEvent::HighestAuctionSale(sale_price),
        ),
    ];
    if let Some(buyer) = buyer {
        events.push((buyer, CriteriaEvent::AuctionWon));
    }
    events
}

fn seller_proceeds(auction: &AuctionData, sale_price: u64) -> u64 {
    let house_fee = (u128::from(sale_price) * u128::from(HOUSE_CUT_BPS) / 10_000) as u64;
    sale_price
        .saturating_sub(house_fee)
        .saturating_add(auction.deposit)
}

/// Seller settlement shared by player and economy purchases.
pub(crate) fn seller_sale_mail(
    mail_id: u64,
    auction: &AuctionData,
    sale_price: u64,
    buyer_name: Option<&str>,
) -> (u64, MailData) {
    let house_fee = (u128::from(sale_price) * u128::from(HOUSE_CUT_BPS) / 10_000) as u64;
    let seller_proceeds = seller_proceeds(auction, sale_price);
    (
        auction.seller_character_id,
        build_auction_mail(
            mail_id,
            &auction_mail_subject("AUCTION_SOLD_MAIL_SUBJECT", auction),
            &auction_invoice_body(auction, buyer_name, sale_price, house_fee),
            seller_proceeds,
            None,
        ),
    )
}

pub(crate) fn sale_mails(
    db: &redb::Database,
    auction: &AuctionData,
    buyer_character_id: u64,
    sale_price: u64,
) -> Vec<(u64, MailData)> {
    // Retail GetInboxInvoiceInfo permits a nil participant name (MailFrame.lua:556-558).
    let buyer_name = persistence::load_character(db, buyer_character_id).map(|buyer| buyer.name);
    vec![
        seller_sale_mail(
            persistence::next_id(db, "mail"),
            auction,
            sale_price,
            buyer_name.as_deref(),
        ),
        (
            buyer_character_id,
            build_auction_mail(
                persistence::next_id(db, "mail"),
                &auction_mail_subject("AUCTION_WON_MAIL_SUBJECT", auction),
                &auction_invoice_body(auction, Some(&auction.seller_name), sale_price, 0),
                0,
                Some(auction.item.clone()),
            ),
        ),
    ]
}

/// Custom transport carries the Retail subject tag and item separately from English.
pub(crate) fn auction_mail_subject(tag: &str, auction: &AuctionData) -> String {
    format!("{tag}:{}", auction.item.name)
}

/// Equivalent to Retail's numeric invoice body; character names replace GUID lookup.
fn auction_invoice_body(
    auction: &AuctionData,
    player: Option<&str>,
    bid: u64,
    consignment: u64,
) -> String {
    format!(
        "{}:{bid}:{}:{}:{consignment}:{}",
        player.unwrap_or(""),
        auction.buyout_price.unwrap_or(0),
        auction.deposit,
        auction.stack_count,
    )
}

/// Build instant Auction House mail without delivering it before the market commit.
pub(crate) fn build_auction_mail(
    mail_id: u64,
    subject: &str,
    body: &str,
    money: u64,
    item: Option<InventoryItemData>,
) -> MailData {
    let now = unix_now();
    MailData {
        mail_id,
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
    }
}
