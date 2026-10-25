use super::store;
use super::{
    Config, VendorSupply,
    model::{
        self, ConsumerPolicy, EconomyState, MaterialLot, MerchantConfig, MerchantState, Recipe,
    },
    store::MarketChanges,
};
use crate::auction_house::{AuctionHouseState, deposit_cost, duration_secs};
use crate::persistence::mail_store;
use crate::persistence::{self, AuctionData, InventoryData, InventoryItemData};
use shared::protocol::AuctionDuration;
use std::collections::{BTreeMap, HashSet};

pub(super) struct CycleStorage<'a> {
    pub db: &'a redb::Database,
    pub ids: &'a mut super::ids::ReservedIds,
}

#[derive(Clone)]
pub(super) struct PreparedCycle {
    pub state: EconomyState,
    pub changes: MarketChanges,
}

impl PreparedCycle {
    pub fn publish(self, auctions: &mut AuctionHouseState, economy: &mut EconomyState) -> Vec<u64> {
        auctions.pending_criteria.extend(self.changes.criteria);
        for id in &self.changes.deleted_auctions {
            auctions.auctions.remove(id);
        }
        for auction in self.changes.auctions {
            auctions.auctions.insert(auction.auction_id, auction);
        }
        *economy = self.state;
        self.changes.inventories.iter().map(|(id, _)| *id).collect()
    }
}

pub(super) fn load_and_prepare_cycle(
    storage: &mut CycleStorage<'_>,
    auctions: &AuctionHouseState,
    economy: &EconomyState,
    config: &Config,
    now: u64,
    draw: &mut impl FnMut() -> u64,
) -> Result<PreparedCycle, String> {
    config.validate_vendor_supplies()?;
    let mut next = economy.clone();
    let mut changes = MarketChanges {
        expected_economy: store::load_state(storage.db)?,
        ..Default::default()
    };
    gather_resources(
        storage,
        auctions,
        &mut next,
        config,
        now,
        draw,
        &mut changes,
    )?;
    let supplies = changes.auctions.clone();
    let mut book = PurchaseBook::new(auctions.auctions.values().chain(supplies.iter()), now);
    rotate_merchants(
        storage,
        &mut next,
        config,
        now,
        draw,
        &mut book,
        &mut changes,
    )?;
    next.last_cycle = now;
    Ok(PreparedCycle {
        state: next,
        changes,
    })
}

#[cfg(test)]
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
    let counts = super::ids::reservation_counts(config, economy)?;
    let mut ids = super::ids::reserve(db, counts)?;
    let plan = load_and_prepare_cycle(
        &mut CycleStorage { db, ids: &mut ids },
        auctions,
        economy,
        config,
        now,
        draw,
    )?;
    store::commit_market(db, Some(&plan.state), &plan.changes)?;
    Ok(plan.publish(auctions, economy))
}

fn gather_resources(
    storage: &mut CycleStorage<'_>,
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
            let seller = storage.ids.take("character")?;
            next.gatherers.insert(item_id, seller);
            let inventory = InventoryData {
                gold: config.gatherer_capital,
                items: Vec::new(),
            };
            post_resource_supply(
                storage, config, now, draw, changes, item_id, seller, inventory, 0,
            )?;
            continue;
        };
        let mut inventory = persistence::load_inventory(storage.db, seller);
        changes
            .expected_inventories
            .push((seller, inventory.clone()));
        collect_gatherer_mail(
            storage.db,
            next,
            config,
            now,
            seller,
            &mut inventory,
            changes,
        )?;
        if market.popularity_bps == 0 {
            changes.inventories.push((seller, inventory));
            continue;
        }
        let listed = active.get(&(seller, item_id)).copied().unwrap_or(0);
        post_resource_supply(
            storage, config, now, draw, changes, item_id, seller, inventory, listed,
        )?;
    }
    Ok(())
}

fn post_resource_supply(
    storage: &mut CycleStorage<'_>,
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
    for _ in listed..target {
        let requested = (draw() % u64::from(config.gatherer_stack_max) + 1) as u32;
        let retained = inventory
            .items
            .iter()
            .position(|item| item.item_id == item_id);
        let (count, vendor_value) = match retained {
            Some(index) => {
                let item = &inventory.items[index];
                (requested.min(item.stack_count), item.vendor_sell_price)
            }
            None => (requested, config.items[&item_id].vendor_sell_price),
        };
        let durations = [
            AuctionDuration::Short,
            AuctionDuration::Medium,
            AuctionDuration::Long,
        ];
        let duration = durations[(draw() % 3) as usize];
        let deposit = deposit_cost(vendor_value, count, duration);
        if inventory.gold < deposit {
            break;
        }
        let unit_price = model::quote_price(market, draw())?;
        let price = unit_price
            .checked_mul(u64::from(count))
            .ok_or("gatherer stack price overflows")?;
        let item =
            take_or_gather_resource(storage, config, &mut inventory, item_id, retained, count)?;
        inventory.gold -= deposit;
        let auction_id = storage.ids.take("auction")?;
        changes.auctions.push(AuctionData {
            auction_id,
            seller_character_id: seller,
            seller_name: format!("Gatherer {item_id}"),
            item,
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
    // Reserved high-water marks are durable before preparing market rows.
    Ok(())
}

fn collect_gatherer_mail(
    db: &redb::Database,
    next: &mut EconomyState,
    config: &Config,
    now: u64,
    seller: u64,
    inventory: &mut InventoryData,
    changes: &mut MarketChanges,
) -> Result<(), String> {
    let mailbox = mail_store::load_mailbox(db, seller);
    changes.expected_mailboxes.push((seller, mailbox.clone()));
    let (delivered, pending): (Vec<_>, Vec<_>) =
        mailbox.into_iter().partition(|mail| mail.is_delivered(now));
    let proceeds = delivered.iter().try_fold(0u64, |sum, mail| {
        sum.checked_add(mail.money)
            .ok_or("gatherer mail money overflows")
    })?;
    inventory.items.extend(
        delivered
            .into_iter()
            .flat_map(|mail| mail.items)
            .map(|attachment| attachment.item),
    );
    changes.mailboxes.push((seller, pending));
    let room = config.gatherer_capital.saturating_sub(inventory.gold);
    let credit = proceeds.min(room);
    inventory.gold += credit;
    next.withdrawn = next
        .withdrawn
        .checked_add(proceeds - credit)
        .ok_or("withdrawal accounting overflows")?;
    Ok(())
}

fn take_or_gather_resource(
    storage: &mut CycleStorage<'_>,
    config: &Config,
    inventory: &mut InventoryData,
    item_id: u32,
    retained: Option<usize>,
    count: u32,
) -> Result<InventoryItemData, String> {
    if let Some(index) = retained {
        if inventory.items[index].stack_count == count {
            return Ok(inventory.items.remove(index));
        }
        let mut split = inventory.items[index].clone();
        inventory.items[index].stack_count -= count;
        split.stack_count = count;
        split.item_guid = storage.ids.take("item_instance")?;
        return Ok(split);
    }
    let metadata = &config.items[&item_id];
    Ok(InventoryItemData {
        item_guid: storage.ids.take("item_instance")?,
        item_id,
        name: metadata.name.clone(),
        quality: metadata.quality,
        required_level: metadata.required_level,
        stack_count: count,
        vendor_sell_price: metadata.vendor_sell_price,
        unique_auction: false,
    })
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
    storage: &mut CycleStorage<'_>,
    next: &mut EconomyState,
    config: &Config,
    now: u64,
    draw: &mut impl FnMut() -> u64,
    book: &mut PurchaseBook<'_>,
    changes: &mut MarketChanges,
) -> Result<(), String> {
    let mut policy = ConsumerPolicy {
        quantity_limit: config.customer_quantity,
        budget: config.customer_budget,
    };
    for business in &config.merchants {
        expire_merchant_stock(next, business, now)?;
        consume_merchant_stock(next, business, config, &mut policy, draw)?;
        produce_merchant_stock(storage, next, business, config, now, draw, book, changes)?;
    }
    Ok(())
}

fn expire_merchant_stock(
    next: &mut EconomyState,
    business: &MerchantConfig,
    now: u64,
) -> Result<(), String> {
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
    Ok(())
}

fn consume_merchant_stock(
    next: &mut EconomyState,
    business: &MerchantConfig,
    config: &Config,
    policy: &mut ConsumerPolicy,
    draw: &mut impl FnMut() -> u64,
) -> Result<(), String> {
    let merchant = next
        .merchants
        .get_mut(&business.entry)
        .ok_or_else(|| format!("merchant {} lacks persisted capital", business.entry))?;
    let draws = (0..policy.quantity_limit)
        .map(|_| draw())
        .collect::<Vec<_>>();
    let (spent, bought) = model::consume_stock(merchant, &config.markets, policy, &draws)?;
    policy.budget -= spent;
    policy.quantity_limit -= bought;
    next.customer_spent = next
        .customer_spent
        .checked_add(spent)
        .ok_or("customer accounting overflows")?;
    let excess = model::credit_capital(merchant, spent, business.capital_limit);
    next.withdrawn = next
        .withdrawn
        .checked_add(excess)
        .ok_or("withdrawal accounting overflows")?;
    Ok(())
}

fn produce_merchant_stock(
    storage: &mut CycleStorage<'_>,
    next: &mut EconomyState,
    business: &MerchantConfig,
    config: &Config,
    now: u64,
    draw: &mut impl FnMut() -> u64,
    book: &mut PurchaseBook<'_>,
    changes: &mut MarketChanges,
) -> Result<(), String> {
    let merchant = next
        .merchants
        .get_mut(&business.entry)
        .ok_or_else(|| format!("merchant {} lacks persisted capital", business.entry))?;
    let Some(recipe) = select_recipe_with_capacity(merchant, business, draw) else {
        return Ok(());
    };
    acquire_ingredients(
        storage,
        merchant,
        recipe,
        business.capacity,
        config,
        book,
        changes,
    )?;
    if !has_recipe_materials(merchant, recipe) {
        return Ok(());
    }
    let market = &config.markets[&recipe.output_item_id];
    let price = model::quote_price(market, draw())?;
    let batch_id = next
        .next_batch_id
        .checked_add(1)
        .ok_or("batch ID exhausted")?;
    model::craft_from_materials(merchant, business, recipe, price, now, batch_id)?;
    next.next_batch_id = batch_id;
    Ok(())
}

fn select_recipe_with_capacity<'a>(
    merchant: &MerchantState,
    business: &'a MerchantConfig,
    draw: &mut impl FnMut() -> u64,
) -> Option<&'a Recipe> {
    if business.recipes.is_empty() {
        return None;
    }
    let recipe = &business.recipes[(draw() % business.recipes.len() as u64) as usize];
    let stock_count: u64 = merchant
        .stock
        .iter()
        .map(|batch| u64::from(batch.count))
        .sum();
    let fits = stock_count + u64::from(recipe.output_count) <= u64::from(business.capacity);
    fits.then_some(recipe)
}

fn has_recipe_materials(merchant: &MerchantState, recipe: &Recipe) -> bool {
    recipe.ingredients.iter().all(|(id, count)| {
        merchant
            .materials
            .iter()
            .filter(|lot| lot.item_id == *id)
            .map(|lot| u64::from(lot.count))
            .sum::<u64>()
            >= u64::from(*count)
    })
}

/// Buy only whole bundles covering the deficit; retain every paid surplus item.
fn purchase_supplier_material(
    merchant: &mut MerchantState,
    item_id: u32,
    deficit: u64,
    supply: &VendorSupply,
) -> Result<(), String> {
    let bundles = deficit.div_ceil(u64::from(supply.buy_count));
    let count = bundles * u64::from(supply.buy_count);
    let count = u32::try_from(count)
        .map_err(|_| format!("supplier item {item_id} bundle quantity overflows"))?;
    let price = bundles
        .checked_mul(supply.purchase_price)
        .ok_or_else(|| format!("supplier item {item_id} purchase cost overflows"))?;
    if price > merchant.gold {
        return Ok(());
    }
    merchant.gold -= price;
    merchant.materials.push(MaterialLot {
        item_id,
        count,
        ingredient_cost: price,
    });
    Ok(())
}

fn acquire_ingredients(
    storage: &mut CycleStorage<'_>,
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
        if let Some(supply) = config.vendor_supplies.get(&item_id) {
            purchase_supplier_material(merchant, item_id, u64::from(needed) - owned, supply)?;
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
        changes.mails.push(crate::auction_house::seller_sale_mail(
            storage.ids.take("mail")?,
            auction,
            price,
            None,
        ));
        changes
            .criteria
            .extend(crate::auction_house::auction_sale_events(
                auction, None, price,
            ));
    }
    Ok(())
}
