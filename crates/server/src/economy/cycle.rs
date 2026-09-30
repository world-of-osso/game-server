use super::{
    Config,
    model::{self, ConsumerPolicy, EconomyState, MaterialLot, MerchantState, Recipe},
    store::{self, MarketChanges},
};
use crate::auction_house::{AuctionHouseState, deposit_cost, duration_secs};
use crate::persistence::mail_store;
use crate::persistence::{self, AuctionData, InventoryData, InventoryItemData};
use shared::protocol::AuctionDuration;
use std::collections::{BTreeMap, HashSet};

/// One bounded wall-clock cycle; persistence commits before runtime state changes.
pub fn run_cycle(
    db: &redb::Database,
    auctions: &mut AuctionHouseState,
    economy: &mut EconomyState,
    config: &Config,
    now: u64,
    draw: &mut impl FnMut() -> u64,
) -> Result<Vec<u64>, String> {
    if economy.last_cycle != 0 && now.saturating_sub(economy.last_cycle) < config.cycle_secs {
        return Ok(Vec::new());
    }
    let mut next = economy.clone();
    let mut changes = MarketChanges::default();
    gather_resources(db, auctions, &mut next, config, now, draw, &mut changes)?;
    let supplies = changes.auctions.clone();
    let mut book = PurchaseBook::new(auctions.auctions.values().chain(supplies.iter()), now);
    rotate_merchants(db, &mut next, config, now, draw, &mut book, &mut changes)?;
    next.last_cycle = now;
    store::commit_market(db, Some(&next), &changes)?;
    for id in &changes.deleted_auctions {
        auctions.auctions.remove(id);
    }
    for auction in changes.auctions {
        auctions.auctions.insert(auction.auction_id, auction);
    }
    let changed = changes.inventories.iter().map(|(id, _)| *id).collect();
    *economy = next;
    Ok(changed)
}

fn gather_resources(
    db: &redb::Database,
    auctions: &AuctionHouseState,
    next: &mut EconomyState,
    config: &Config,
    now: u64,
    draw: &mut impl FnMut() -> u64,
    changes: &mut MarketChanges,
) -> Result<(), String> {
    let active =
        auctions
            .auctions
            .values()
            .fold(BTreeMap::<(u64, u32), u32>::new(), |mut counts, a| {
                *counts
                    .entry((a.seller_character_id, a.item.item_id))
                    .or_default() += 1;
                counts
            });
    for &item_id in &config.gatherables {
        let market = &config.markets[&item_id];
        let Some(&seller) = next.gatherers.get(&item_id) else {
            let seller = persistence::next_id(db, "character");
            next.gatherers.insert(item_id, seller);
            let inventory = InventoryData {
                gold: config.gatherer_capital,
                items: Vec::new(),
            };
            post_resource_supply(
                db, next, config, now, draw, changes, item_id, seller, inventory, 0,
            )?;
            continue;
        };
        let mut inventory = persistence::load_inventory(db, seller);
        let mail = mail_store::load_mailbox(db, seller);
        let proceeds =
            mail.iter()
                .filter(|mail| mail.is_delivered(now))
                .try_fold(0u64, |sum, mail| {
                    sum.checked_add(mail.money)
                        .ok_or("gatherer mail money overflows")
                })?;
        let kept = mail
            .into_iter()
            .filter(|mail| !mail.is_delivered(now))
            .collect();
        changes.mailboxes.push((seller, kept));
        let room = config.gatherer_capital.saturating_sub(inventory.gold);
        let credit = proceeds.min(room);
        inventory.gold += credit;
        next.withdrawn = next
            .withdrawn
            .checked_add(proceeds - credit)
            .ok_or("withdrawal accounting overflows")?;
        if market.popularity_bps == 0 {
            changes.inventories.push((seller, inventory));
            continue;
        }
        let listed = active.get(&(seller, item_id)).copied().unwrap_or(0);
        post_resource_supply(
            db, next, config, now, draw, changes, item_id, seller, inventory, listed,
        )?;
    }
    Ok(())
}

fn post_resource_supply(
    db: &redb::Database,
    next: &mut EconomyState,
    config: &Config,
    now: u64,
    draw: &mut impl FnMut() -> u64,
    changes: &mut MarketChanges,
    item_id: u32,
    seller: u64,
    mut inventory: InventoryData,
    listed: u32,
) -> Result<(), String> {
    let market = &config.markets[&item_id];
    let target = (u64::from(config.gatherer_listing_limit) * u64::from(market.popularity_bps)
        / 10_000) as u32;
    let metadata = &config.items[&item_id];
    for _ in listed..target {
        let count = (draw() % u64::from(config.gatherer_stack_max) + 1) as u32;
        let durations = [
            AuctionDuration::Short,
            AuctionDuration::Medium,
            AuctionDuration::Long,
        ];
        let duration = durations[(draw() % 3) as usize];
        let deposit = deposit_cost(metadata.vendor_sell_price, count, duration);
        if inventory.gold < deposit {
            break;
        }
        let unit_price = model::quote_price(market, draw())?;
        let price = unit_price
            .checked_mul(u64::from(count))
            .ok_or("gatherer stack price overflows")?;
        inventory.gold -= deposit;
        let auction_id = persistence::next_id(db, "auction");
        changes.auctions.push(AuctionData {
            auction_id,
            seller_character_id: seller,
            seller_name: format!("Gatherer {item_id}"),
            item: InventoryItemData {
                item_guid: persistence::next_id(db, "item_instance"),
                item_id,
                name: metadata.name.clone(),
                quality: metadata.quality,
                required_level: metadata.required_level,
                stack_count: count,
                vendor_sell_price: metadata.vendor_sell_price,
                unique_auction: false,
            },
            stack_count: count,
            min_bid: price,
            current_bid: None,
            current_bidder_character_id: None,
            buyout_price: Some(price),
            expires_at_unix_secs: now
                .checked_add(duration_secs(duration))
                .ok_or("auction deadline overflows")?,
            faction: 0,
            deposit,
        });
    }
    changes.inventories.push((seller, inventory));
    // IDs are allocated separately, but supply, deposits and actor identity commit together.
    let _ = next;
    Ok(())
}

struct PurchaseBook<'a> {
    by_item: BTreeMap<u32, Vec<&'a AuctionData>>,
    purchased: HashSet<u64>,
}
impl<'a> PurchaseBook<'a> {
    fn new(auctions: impl Iterator<Item = &'a AuctionData>, now: u64) -> Self {
        let mut by_item = BTreeMap::<u32, Vec<&AuctionData>>::new();
        for auction in auctions {
            if auction.expires_at_unix_secs > now
                && auction.buyout_price.is_some()
                && auction.current_bidder_character_id.is_none()
            {
                by_item
                    .entry(auction.item.item_id)
                    .or_default()
                    .push(auction);
            }
        }
        for offers in by_item.values_mut() {
            offers.sort_unstable_by(|a, b| {
                let a_price = u128::from(a.buyout_price.unwrap_or(0)) * u128::from(b.stack_count);
                let b_price = u128::from(b.buyout_price.unwrap_or(0)) * u128::from(a.stack_count);
                a_price
                    .cmp(&b_price)
                    .then_with(|| a.auction_id.cmp(&b.auction_id))
            });
        }
        Self {
            by_item,
            purchased: HashSet::new(),
        }
    }

    fn affordable(
        &self,
        item_id: u32,
        config: &Config,
        gold: u64,
        quantity_limit: u32,
    ) -> Option<&'a AuctionData> {
        let market = config.markets.get(&item_id)?;
        self.by_item.get(&item_id)?.iter().copied().find(|auction| {
            let price = auction.buyout_price.unwrap_or(0);
            let count = u64::from(auction.stack_count);
            let bounded = price >= market.min_price.saturating_mul(count)
                && price <= market.max_price.saturating_mul(count);
            !self.purchased.contains(&auction.auction_id)
                && auction.stack_count > 0
                && auction.stack_count <= quantity_limit
                && price <= gold
                && bounded
        })
    }
}

fn rotate_merchants(
    db: &redb::Database,
    next: &mut EconomyState,
    config: &Config,
    now: u64,
    draw: &mut impl FnMut() -> u64,
    book: &mut PurchaseBook<'_>,
    changes: &mut MarketChanges,
) -> Result<(), String> {
    let mut budget = config.customer_budget;
    let mut quantity = config.customer_quantity;
    for business in &config.merchants {
        let merchant = next
            .merchants
            .get_mut(&business.entry)
            .ok_or_else(|| format!("merchant {} lacks persisted capital", business.entry))?;
        let refund = model::expire_stock(merchant, now)?;
        next.refund_issued = next
            .refund_issued
            .checked_add(refund)
            .ok_or("refund accounting overflows")?;
        let excess = model::credit_capital(merchant, refund, business.capital_limit);
        next.withdrawn = next
            .withdrawn
            .checked_add(excess)
            .ok_or("withdrawal accounting overflows")?;
        let policy = ConsumerPolicy {
            quantity_limit: quantity,
            budget,
        };
        let draws = (0..quantity).map(|_| draw()).collect::<Vec<_>>();
        let (spent, bought) = model::consume_stock(merchant, &config.markets, &policy, &draws)?;
        budget -= spent;
        quantity -= bought;
        next.customer_spent = next
            .customer_spent
            .checked_add(spent)
            .ok_or("customer accounting overflows")?;
        let excess = model::credit_capital(merchant, spent, business.capital_limit);
        next.withdrawn = next
            .withdrawn
            .checked_add(excess)
            .ok_or("withdrawal accounting overflows")?;
        if business.recipes.is_empty() {
            continue;
        }
        let recipe = &business.recipes[(draw() % business.recipes.len() as u64) as usize];
        let stock_count: u64 = merchant
            .stock
            .iter()
            .map(|batch| u64::from(batch.count))
            .sum();
        if stock_count + u64::from(recipe.output_count) > u64::from(business.capacity) {
            continue;
        }
        acquire_ingredients(
            db,
            merchant,
            recipe,
            business.capacity,
            config,
            book,
            changes,
        )?;
        let available = recipe.ingredients.iter().all(|(id, count)| {
            merchant
                .materials
                .iter()
                .filter(|lot| lot.item_id == *id)
                .map(|lot| u64::from(lot.count))
                .sum::<u64>()
                >= u64::from(*count)
        });
        if !available {
            continue;
        }
        let market = &config.markets[&recipe.output_item_id];
        let price = model::quote_price(market, draw())?;
        let batch_id = next
            .next_batch_id
            .checked_add(1)
            .ok_or("batch ID exhausted")?;
        match model::craft_from_materials(merchant, business, recipe, price, now, batch_id) {
            Ok(()) => next.next_batch_id = batch_id,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn acquire_ingredients(
    db: &redb::Database,
    merchant: &mut MerchantState,
    recipe: &Recipe,
    capacity: u32,
    config: &Config,
    book: &mut PurchaseBook<'_>,
    changes: &mut MarketChanges,
) -> Result<(), String> {
    for &(item_id, needed) in &recipe.ingredients {
        let owned: u64 = merchant
            .materials
            .iter()
            .filter(|lot| lot.item_id == item_id)
            .map(|lot| u64::from(lot.count))
            .sum();
        if owned >= u64::from(needed) {
            continue;
        }
        let limit = capacity.saturating_mul(needed).saturating_sub(owned as u32);
        let Some(auction) = book.affordable(item_id, config, merchant.gold, limit) else {
            continue;
        };
        let price = auction.buyout_price.ok_or("purchase lost buyout quote")?;
        merchant.gold -= price;
        merchant.materials.push(MaterialLot {
            item_id,
            count: auction.stack_count,
            ingredient_cost: price,
        });
        book.purchased.insert(auction.auction_id);
        if let Some(index) = changes
            .auctions
            .iter()
            .position(|row| row.auction_id == auction.auction_id)
        {
            changes.auctions.remove(index);
        } else {
            changes
                .expected_auctions
                .push((auction.auction_id, auction.clone()));
            changes.deleted_auctions.push(auction.auction_id);
        }
        changes
            .mails
            .push(crate::auction_house::seller_sale_mail(db, auction, price));
    }
    Ok(())
}
