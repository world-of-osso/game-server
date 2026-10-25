//! Persistent bot supply, crafting businesses and simulated local consumption.
mod cycle;
mod ids;
pub mod model;
mod publication;
pub mod store;

use crate::auction_house::{AuctionHouseState, unix_now};
use crate::persistence::GameDatabase;
use bevy::prelude::*;
#[cfg(test)]
pub use cycle::run_cycle;
use model::{EconomyState, MarketItem, MerchantConfig, MerchantState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemMetadata {
    pub name: String,
    pub quality: u8,
    pub required_level: u16,
    pub vendor_sell_price: u32,
}

/// One supplier row from the expanded VendorCatalog. Limited stock is unsupported.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VendorSupply {
    pub vendor_entry: u32,
    /// Zero-based index in the expanded vendor catalog.
    pub slot: u32,
    /// Actual BuyPrice in copper per purchase yielding `buy_count` items.
    pub purchase_price: u64,
    pub buy_count: u32,
    /// VendorSlot.maxcount (supplier stock), not the item ownership limit.
    pub max_count: u32,
    pub restock_secs: u32,
    /// Omitted by copper-only importer rows; nonzero token/item costs are unsupported.
    #[serde(default)]
    pub extended_cost: u32,
}

#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub cycle_secs: u64,
    pub gatherer_listing_limit: u32,
    pub gatherer_stack_max: u32,
    pub gatherer_capital: u64,
    pub gatherables: Vec<u32>,
    pub vendor_supplies: BTreeMap<u32, VendorSupply>,
    pub customer_budget: u64,
    pub customer_quantity: u32,
    pub items: BTreeMap<u32, ItemMetadata>,
    pub markets: BTreeMap<u32, MarketItem>,
    pub merchants: Vec<MerchantConfig>,
}

impl Config {
    pub fn merchant(&self, entry: u32) -> Option<&MerchantConfig> {
        self.merchants
            .iter()
            .find(|merchant| merchant.entry == entry)
    }

    fn validate_vendor_supplies(&self) -> Result<(), String> {
        for (&item_id, supply) in &self.vendor_supplies {
            if supply.max_count != 0 || supply.restock_secs != 0 {
                return Err(format!(
                    "supplier item {item_id} has unsupported limited stock"
                ));
            }
            if supply.extended_cost != 0 {
                return Err(format!(
                    "supplier item {item_id} has unsupported extended cost"
                ));
            }
            if supply.vendor_entry == 0 || supply.buy_count == 0 || supply.purchase_price == 0 {
                return Err(format!(
                    "supplier item {item_id} requires a vendor, positive bundle and price"
                ));
            }
            if !self.items.contains_key(&item_id) || !self.markets.contains_key(&item_id) {
                return Err(format!(
                    "supplier item {item_id} lacks market/item metadata"
                ));
            }
            if self.gatherables.contains(&item_id) {
                return Err(format!(
                    "supplier item {item_id} cannot also be a gatherable"
                ));
            }
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), String> {
        self.validate_vendor_supplies()?;
        if self.cycle_secs == 0 || self.gatherer_stack_max == 0 {
            return Err("economy cycle and gathering stack limit must be positive".into());
        }
        self.validate_merchants()?;
        self.validate_gatherables()?;
        self.validate_markets()
    }

    fn validate_merchants(&self) -> Result<(), String> {
        for merchant in &self.merchants {
            if merchant.capital_limit > crate::gold::MAX_MONEY || merchant.capacity == 0 {
                return Err(format!(
                    "invalid capital/capacity for merchant {}",
                    merchant.entry
                ));
            }
            for recipe in &merchant.recipes {
                self.validate_recipe(merchant, recipe)?;
            }
        }
        Ok(())
    }

    fn validate_recipe(
        &self,
        merchant: &MerchantConfig,
        recipe: &model::Recipe,
    ) -> Result<(), String> {
        if recipe.expansion != merchant.expansion {
            return Err(format!(
                "recipe {} crosses merchant expansion",
                recipe.spell_id
            ));
        }
        for item_id in std::iter::once(recipe.output_item_id)
            .chain(recipe.ingredients.iter().map(|(id, _)| *id))
        {
            if !self.markets.contains_key(&item_id) || !self.items.contains_key(&item_id) {
                return Err(format!(
                    "recipe {} lacks market/item metadata for {item_id}",
                    recipe.spell_id
                ));
            }
        }
        self.validate_output_price(merchant, recipe)
    }

    fn validate_output_price(
        &self,
        merchant: &MerchantConfig,
        recipe: &model::Recipe,
    ) -> Result<(), String> {
        let output_market = &self.markets[&recipe.output_item_id];
        if output_market.max_price > u64::from(u32::MAX) {
            return Err(format!(
                "merchant {} recipe {} output {} max price {} exceeds vendor protocol limit {}",
                merchant.entry,
                recipe.spell_id,
                recipe.output_item_id,
                output_market.max_price,
                u32::MAX
            ));
        }
        Ok(())
    }

    fn validate_gatherables(&self) -> Result<(), String> {
        for item_id in &self.gatherables {
            if !self.markets.contains_key(item_id) || !self.items.contains_key(item_id) {
                return Err(format!(
                    "gathering item {item_id} lacks market/item metadata"
                ));
            }
        }
        if self.gatherer_capital > crate::gold::MAX_MONEY {
            return Err("gatherer capital exceeds the game money limit".into());
        }
        Ok(())
    }

    fn validate_markets(&self) -> Result<(), String> {
        for market in self.markets.values() {
            model::quote_price(market, 0)?;
            if market.popularity_bps > 10_000 {
                return Err(format!(
                    "market {} popularity {} exceeds 10000 bps",
                    market.item_id, market.popularity_bps
                ));
            }
        }
        Ok(())
    }
}

#[derive(Resource, Default)]
pub struct Economy {
    pub state: EconomyState,
    initialized: bool,
    pending: Option<publication::PendingCycle>,
    last_attempt: Option<u64>,
}

/// Snapshot only: do not initialize, cycle, persist or read the clock.
pub(crate) fn admin_status(
    economy: Option<&Economy>,
    config: Option<&Config>,
) -> crate::admin_types::AdminResponse {
    use crate::admin_types::AdminResponse;
    let Some(economy) = economy.filter(|economy| economy.initialized) else {
        return AdminResponse::Error("economy not initialized".into());
    };
    let Some(config) = config else {
        return AdminResponse::Error("economy config not loaded".into());
    };
    let state = &economy.state;
    let merchants: Vec<_> = state
        .merchants
        .iter()
        .map(|(&entry, merchant)| {
            serde_json::json!({
                "entry": entry,
                "gold": merchant.gold,
                "capital_cap": config.merchant(entry).map(|merchant| merchant.capital_limit),
                "stock": merchant.stock,
                "materials": merchant.materials,
            })
        })
        .collect();
    AdminResponse::EconomyStatus {
        status: serde_json::json!({
            "last_cycle": state.last_cycle,
            "merchants": merchants,
            "gatherers": state.gatherers,
            "gatherer_count": state.gatherers.len(),
            "customer_spent": state.customer_spent,
            "refund_issued": state.refund_issued,
            "withdrawn": state.withdrawn,
        }),
    }
}

pub struct EconomyPlugin;
impl Economy {
    pub(crate) fn market_commit_pending(&self) -> bool {
        matches!(self.pending, Some(publication::PendingCycle::Commit { .. }))
    }

    pub fn from_state(state: EconomyState) -> Self {
        let last_attempt = (state.last_cycle != 0).then_some(state.last_cycle);
        Self {
            state,
            initialized: true,
            pending: None,
            last_attempt,
        }
    }
}

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Economy>()
            .add_systems(Startup, load_config)
            .add_systems(Update, initialize_economy)
            .add_systems(
                Update,
                publish_economy_market.before(crate::merchant::MerchantRequests),
            )
            .add_systems(FixedUpdate, update_economy);
    }
}

fn load_config(mut commands: Commands) {
    let path = std::env::var_os("GAME_ECONOMY_CONFIG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("data/economy/config.json"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "read economy config {}: {error}; generate it with scripts/economy_catalog.py",
            path.display()
        )
    });
    let config: Config = serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("decode economy config {}: {error}", path.display()));
    config
        .validate()
        .unwrap_or_else(|error| panic!("validate economy config {}: {error}", path.display()));
    commands.insert_resource(config);
}

fn initialize_economy(
    db: Option<Res<GameDatabase>>,
    config: Res<Config>,
    mut economy: ResMut<Economy>,
) {
    let Some(db) = db else { return };
    if economy.initialized {
        return;
    }
    store::init_state_table(&db.db).expect("initialize economy persistence");
    economy.state = store::load_state(&db.db)
        .expect("load economy state")
        .unwrap_or_default();
    for merchant in &config.merchants {
        economy
            .state
            .merchants
            .entry(merchant.entry)
            .or_insert_with(|| MerchantState {
                gold: merchant.capital_limit,
                ..Default::default()
            });
    }
    store::commit_market(
        &db.db,
        Some(&economy.state),
        &store::MarketChanges::default(),
    )
    .expect("persist initialized economy");
    *economy = Economy::from_state(std::mem::take(&mut economy.state));
}

pub(crate) fn publish_economy_market(
    db: Option<Res<GameDatabase>>,
    mut economy: ResMut<Economy>,
    mut auctions: ResMut<AuctionHouseState>,
    mut gold: ResMut<crate::gold::GoldUpdateQueue>,
) {
    if let Some(db) = db {
        publication::publish_ready(&db, &mut economy, &mut auctions, &mut gold);
    }
}

fn update_economy(
    db: Option<Res<GameDatabase>>,
    config: Res<Config>,
    mut economy: ResMut<Economy>,
    mut auctions: ResMut<AuctionHouseState>,
    mut gold: ResMut<crate::gold::GoldUpdateQueue>,
) {
    let Some(db) = db else { return };
    if !economy.initialized || !auctions.initialized {
        return;
    }
    publication::advance_cycle(
        &db,
        &config,
        &mut economy,
        &mut auctions,
        &mut gold,
        unix_now(),
    );
}

#[cfg(test)]
pub(crate) mod fifo_tests;
#[cfg(test)]
mod tests;
