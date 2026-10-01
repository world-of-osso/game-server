use std::collections::BTreeMap;

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub const STOCK_LIFETIME_SECONDS: u64 = 24 * 60 * 60;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct MarketItem {
    pub item_id: u32,
    pub anchor_price: u64,
    pub min_price: u64,
    pub max_price: u64,
    pub popularity_bps: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Recipe {
    pub spell_id: u32,
    pub expansion: i32,
    pub output_item_id: u32,
    pub output_count: u32,
    pub ingredients: Vec<(u32, u32)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct MerchantConfig {
    pub entry: u32,
    pub expansion: i32,
    pub recipes: Vec<Recipe>,
    pub capital_limit: u64,
    pub capacity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct StockBatch {
    pub id: u64,
    pub item_id: u32,
    pub count: u32,
    /// Total ingredient cost still attributable to unsold units.
    pub ingredient_cost: u64,
    pub unit_price: u64,
    pub expires_at: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct MerchantState {
    pub gold: u64,
    pub stock: Vec<StockBatch>,
    pub materials: Vec<MaterialLot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct MaterialLot {
    pub item_id: u32,
    pub count: u32,
    /// Total paid cost still attributable to unused ingredients.
    pub ingredient_cost: u64,
}

/// Craft from already-paid lots in purchase order, retaining unused quantities and costs.
/// Errors leave materials, stock and gold unchanged; crafting never charges the wallet.
pub fn craft_from_materials(
    state: &mut MerchantState,
    config: &MerchantConfig,
    recipe: &Recipe,
    unit_price: u64,
    now: u64,
    batch_id: u64,
) -> Result<(), String> {
    let expires_at = validate_craft(state, config, recipe, unit_price, now, batch_id)?;
    let mut materials = state.materials.clone();
    let ingredient_cost =
        recipe
            .ingredients
            .iter()
            .try_fold(0_u64, |total, &(item_id, count)| {
                let cost = consume_material_lots(&mut materials, item_id, count)?;
                total
                    .checked_add(cost)
                    .ok_or_else(|| "consumed ingredient cost overflows".to_string())
            })?;
    materials.retain(|lot| lot.count > 0);
    state.stock.push(StockBatch {
        id: batch_id,
        item_id: recipe.output_item_id,
        count: recipe.output_count,
        ingredient_cost,
        unit_price,
        expires_at,
    });
    state.materials = materials;
    Ok(())
}

fn consume_material_lots(
    materials: &mut [MaterialLot],
    item_id: u32,
    mut count: u32,
) -> Result<u64, String> {
    let mut cost = 0_u64;
    for lot in materials
        .iter_mut()
        .filter(|lot| lot.item_id == item_id && lot.count > 0)
    {
        if count == 0 {
            break;
        }
        let consumed = count.min(lot.count);
        let consumed_cost = prorate_cost(lot.ingredient_cost, consumed, lot.count);
        cost = cost
            .checked_add(consumed_cost)
            .ok_or_else(|| "consumed ingredient cost overflows".to_string())?;
        lot.count -= consumed;
        lot.ingredient_cost -= consumed_cost;
        count -= consumed;
    }
    if count != 0 {
        return Err(format!("missing {count} units of ingredient {item_id}"));
    }
    Ok(cost)
}

fn prorate_cost(cost: u64, count: u32, total_count: u32) -> u64 {
    (u128::from(cost) * u128::from(count) / u128::from(total_count)) as u64
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct EconomyState {
    /// Economic seller IDs allocated from the character ID sequence; not login characters.
    pub gatherers: BTreeMap<u32, u64>,
    pub merchants: BTreeMap<u32, MerchantState>,
    pub last_cycle: u64,
    pub next_batch_id: u64,
    pub customer_spent: u64,
    pub refund_issued: u64,
    pub withdrawn: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct ConsumerPolicy {
    pub quantity_limit: u32,
    pub budget: u64,
}

/// Remove expired stock and return its remaining ingredient cost, without crediting gold.
/// The caller records the refund and credits it through `credit_capital`.
pub fn expire_stock(state: &mut MerchantState, now: u64) -> Result<u64, String> {
    let refund = state
        .stock
        .iter()
        .filter(|batch| batch.expires_at <= now)
        .try_fold(0_u64, |total, batch| {
            total
                .checked_add(batch.ingredient_cost)
                .ok_or_else(|| "expired ingredient refund exceeds u64".to_string())
        })?;
    state.stock.retain(|batch| batch.expires_at > now);
    Ok(refund)
}

/// Return the item and total sale price; wallet/accounting credits belong to the caller.
/// Allocate the sold cost by floor division, retaining every rounding remainder in stock.
pub fn sell_stock(
    state: &mut MerchantState,
    batch_id: u64,
    count: u32,
) -> Result<(u32, u64), String> {
    let index = state
        .stock
        .iter()
        .position(|batch| batch.id == batch_id)
        .ok_or_else(|| format!("stock batch {batch_id} not found"))?;
    let batch = &mut state.stock[index];
    if count == 0 || count > batch.count {
        return Err(format!(
            "invalid sale count {count} for batch {batch_id} with {} units",
            batch.count
        ));
    }
    let price = batch
        .unit_price
        .checked_mul(u64::from(count))
        .ok_or_else(|| format!("sale price overflows for batch {batch_id}"))?;
    let sold_cost = prorate_cost(batch.ingredient_cost, count, batch.count);
    let item_id = batch.item_id;
    batch.ingredient_cost -= sold_cost;
    batch.count -= count;
    if batch.count == 0 {
        state.stock.remove(index);
    }
    Ok((item_id, price))
}

fn validate_craft(
    state: &MerchantState,
    config: &MerchantConfig,
    recipe: &Recipe,
    unit_price: u64,
    now: u64,
    batch_id: u64,
) -> Result<u64, String> {
    validate_recipe(config, recipe)?;
    validate_craft_capacity(state, config.capacity, recipe.output_count, batch_id)?;
    if unit_price == 0 {
        return Err("crafted unit price must be positive".to_string());
    }
    now.checked_add(STOCK_LIFETIME_SECONDS)
        .ok_or_else(|| "stock expiry timestamp overflows".to_string())
}

fn validate_recipe(config: &MerchantConfig, recipe: &Recipe) -> Result<(), String> {
    if recipe.expansion != config.expansion {
        return Err(format!(
            "recipe {} expansion differs from merchant {}",
            recipe.spell_id, config.entry
        ));
    }
    if !config.recipes.contains(recipe) {
        return Err(format!(
            "recipe {} is not configured for merchant {}",
            recipe.spell_id, config.entry
        ));
    }
    if recipe.spell_id == 0 || recipe.output_item_id == 0 || recipe.output_count == 0 {
        return Err("recipe spell, output item and output quantity must be nonzero".to_string());
    }
    if recipe.ingredients.is_empty() {
        return Err("recipe must specify purchased ingredients".to_string());
    }
    if recipe
        .ingredients
        .iter()
        .any(|&(item_id, count)| item_id == 0 || count == 0)
    {
        return Err("ingredient item and quantity must be nonzero".to_string());
    }
    Ok(())
}

fn validate_craft_capacity(
    state: &MerchantState,
    capacity: u32,
    output_count: u32,
    batch_id: u64,
) -> Result<(), String> {
    if state.stock.iter().any(|batch| batch.id == batch_id) {
        return Err(format!("stock batch {batch_id} already exists"));
    }
    let used = state.stock.iter().try_fold(0_u32, |total, batch| {
        total
            .checked_add(batch.count)
            .ok_or_else(|| "stock quantity overflows".to_string())
    })?;
    let required = used
        .checked_add(output_count)
        .ok_or_else(|| "crafted stock quantity overflows".to_string())?;
    if required > capacity {
        return Err(format!(
            "shared stock capacity {capacity} cannot hold {required} units"
        ));
    }
    Ok(())
}

/// Credit only available headroom. Return uncredited gold for the withdrawn ledger.
/// An already-over-limit wallet receives no credit; existing gold is not confiscated.
pub fn credit_capital(state: &mut MerchantState, amount: u64, limit: u64) -> u64 {
    if state.gold >= limit {
        return amount;
    }
    let credited = amount.min(limit - state.gold);
    state.gold += credited;
    amount - credited
}

/// Map the supplied draw into the inclusive price interval; no randomness is generated here.
pub fn quote_price(item: &MarketItem, random: u64) -> Result<u64, String> {
    validate_market_item(item)?;
    let range = item.max_price - item.min_price + 1;
    Ok(item.min_price + random % range)
}

fn validate_market_item(item: &MarketItem) -> Result<(), String> {
    if item.item_id == 0 || item.min_price == 0 {
        return Err("market item and minimum price must be positive".to_string());
    }
    if item.min_price > item.max_price {
        return Err(format!(
            "market item {} has reversed price bounds",
            item.item_id
        ));
    }
    if !(item.min_price..=item.max_price).contains(&item.anchor_price) {
        return Err(format!(
            "market item {} anchor is outside price bounds",
            item.item_id
        ));
    }
    Ok(())
}

/// Spend at most the supplied budget/quantity. One draw purchases at most one unit.
/// Return (total spending, purchased quantity); do not credit the merchant wallet.
/// Errors leave the original stock untouched. Caller expires stock before consumption.
pub fn consume_stock(
    state: &mut MerchantState,
    market: &BTreeMap<u32, MarketItem>,
    policy: &ConsumerPolicy,
    draws: &[u64],
) -> Result<(u64, u32), String> {
    let mut remaining = state.clone();
    let mut spending = 0;
    let mut quantity = 0;
    for &draw in draws.iter().take(policy.quantity_limit as usize) {
        let budget = policy.budget - spending;
        let Some(batch_id) = select_consumer_batch(&remaining, market, budget, draw)? else {
            break;
        };
        let (_, price) = sell_stock(&mut remaining, batch_id, 1)?;
        spending += price;
        quantity += 1;
    }
    *state = remaining;
    Ok((spending, quantity))
}

fn select_consumer_batch(
    state: &MerchantState,
    market: &BTreeMap<u32, MarketItem>,
    budget: u64,
    draw: u64,
) -> Result<Option<u64>, String> {
    let candidates = collect_consumer_candidates(state, market, budget)?;
    let total_weight: u128 = candidates
        .values()
        .map(|&(_, weight)| u128::from(weight))
        .sum();
    if total_weight == 0 {
        return Ok(None);
    }
    let mut ticket = u128::from(draw) % total_weight;
    for (batch_id, weight) in candidates.into_values() {
        if ticket < u128::from(weight) {
            return Ok(Some(batch_id));
        }
        ticket -= u128::from(weight);
    }
    Err("consumer selection exhausted a nonzero weight interval".to_string())
}

fn collect_consumer_candidates(
    state: &MerchantState,
    market: &BTreeMap<u32, MarketItem>,
    budget: u64,
) -> Result<BTreeMap<u32, (u64, u32)>, String> {
    let mut candidates = BTreeMap::new();
    for batch in &state.stock {
        let item = market
            .get(&batch.item_id)
            .ok_or_else(|| format!("stock item {} missing market data", batch.item_id))?;
        validate_market_item(item)?;
        if item.item_id != batch.item_id {
            return Err(format!(
                "market key {} differs from item {}",
                batch.item_id, item.item_id
            ));
        }
        if batch.count == 0 || batch.unit_price == 0 {
            return Err(format!(
                "stock batch {} has zero quantity or price",
                batch.id
            ));
        }
        if batch.unit_price <= budget && item.popularity_bps > 0 {
            // Weight each product once, independent of how its stock is split into batches.
            candidates
                .entry(batch.item_id)
                .or_insert((batch.id, item.popularity_bps));
        }
    }
    Ok(candidates)
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
