//! Persistent bot supply, crafting businesses and simulated local consumption.
mod cycle;
pub mod model;
pub mod store;

use crate::auction_house::{AuctionHouseState, unix_now};
use crate::persistence::GameDatabase;
use bevy::prelude::*;
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

#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub cycle_secs: u64,
    pub gatherer_listing_limit: u32,
    pub gatherer_stack_max: u32,
    pub gatherer_capital: u64,
    pub gatherables: Vec<u32>,
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

    fn validate(&self) -> Result<(), String> {
        if self.cycle_secs == 0 || self.gatherer_stack_max == 0 {
            return Err("economy cycle and gathering stack limit must be positive".into());
        }
        for merchant in &self.merchants {
            if merchant.capital_limit > crate::gold::MAX_MONEY || merchant.capacity == 0 {
                return Err(format!(
                    "invalid capital/capacity for merchant {}",
                    merchant.entry
                ));
            }
            for recipe in &merchant.recipes {
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
            }
        }
        for item_id in &self.gatherables {
            if !self.markets.contains_key(item_id) || !self.items.contains_key(item_id) {
                return Err(format!(
                    "gathering item {item_id} lacks market/item metadata"
                ));
            }
        }
        for market in self.markets.values() {
            model::quote_price(market, 0)?;
        }
        Ok(())
    }
}

#[derive(Resource, Default)]
pub struct Economy {
    pub state: EconomyState,
    initialized: bool,
}

pub struct EconomyPlugin;
impl Economy {
    pub fn from_state(state: EconomyState) -> Self {
        Self {
            state,
            initialized: true,
        }
    }
}

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Economy>()
            .add_systems(Startup, load_config)
            .add_systems(Update, initialize_economy)
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
    economy.initialized = true;
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
    let mut draw = || rand::random::<u64>();
    match run_cycle(
        &db.db,
        &mut auctions,
        &mut economy.state,
        &config,
        unix_now(),
        &mut draw,
    ) {
        Ok(changed) => {
            for id in changed {
                crate::gold::record_persisted_gold(&db.db, id, &mut gold);
            }
        }
        Err(error) => error!("economy cycle aborted: {error}"),
    }
}

#[cfg(test)]
mod tests;
