//! Finite crafted stock; static vendor rules remain for unassigned NPCs.
use super::catalog::VendorGoods;
use super::*;
use crate::economy::{
    model,
    store::{self, MarketChanges},
};

pub(super) fn is_business(shop: &Shop, entry: u32) -> bool {
    shop.economy_config
        .as_ref()
        .is_some_and(|config| config.merchant(entry).is_some())
}

pub(super) fn vendor_inventory(
    shop: &Shop,
    vendor: Entity,
    entry: u32,
    buyer: Buyer,
    can_repair: bool,
) -> VendorInventory {
    let now = crate::auction_house::unix_now();
    let stock = shop
        .economy
        .as_ref()
        .and_then(|economy| economy.state.merchants.get(&entry));
    let items = stock
        .into_iter()
        .flat_map(|merchant| merchant.stock.iter())
        .filter(|batch| batch.expires_at > now && batch.count > 0)
        .filter_map(|batch| offered_item(shop, buyer, batch))
        .collect();
    VendorInventory {
        npc: vendor.to_bits(),
        can_repair,
        items,
    }
}

fn offered_item(shop: &Shop, buyer: Buyer, batch: &model::StockBatch) -> Option<VendorItem> {
    let template = shop.template(batch.item_id)?;
    if !buyer.offered(&template) {
        return None;
    }
    let goods = shop.catalog.goods.get(&batch.item_id)?;
    let slot = match u32::try_from(batch.id) {
        Ok(slot) => slot,
        Err(error) => {
            error!(
                "economic vendor batch {} slot conversion: {error}",
                batch.id
            );
            return None;
        }
    };
    let price = match u32::try_from(batch.unit_price) {
        Ok(price) => price,
        Err(error) => {
            error!(
                "economic vendor batch {} price conversion: {error}",
                batch.id
            );
            return None;
        }
    };
    Some(VendorItem {
        slot,
        item_id: batch.item_id,
        name: goods.name.clone(),
        quality: goods.quality,
        price,
        stack_count: 1,
        max_stack: template.max_stack,
        num_available: Some(batch.count),
        usable: buyer.can_use(&template),
    })
}

pub(super) fn buy(
    shop: &mut Shop,
    customer: &mut Customer,
    vendor: Entity,
    entry: u32,
    req: BuyItem,
) -> MerchantResult {
    let business = shop
        .economy_config
        .as_ref()
        .and_then(|config| config.merchant(entry))
        .cloned()
        .ok_or(MerchantError::ItemNotFound)?;
    let mut state = shop
        .economy
        .as_ref()
        .ok_or(MerchantError::ItemNotFound)?
        .state
        .clone();
    let merchant = state
        .merchants
        .get_mut(&entry)
        .ok_or(MerchantError::ItemNotFound)?;
    let batch = merchant
        .stock
        .iter()
        .find(|batch| batch.id == u64::from(req.slot) && batch.item_id == req.item_id)
        .cloned()
        .ok_or(MerchantError::ItemNotFound)?;
    if batch.expires_at <= crate::auction_house::unix_now() {
        return Err(MerchantError::SoldOut);
    }
    let count = req.count.max(1);
    if count > batch.count {
        return Err(MerchantError::SoldOut);
    }
    let template = shop
        .template(batch.item_id)
        .ok_or(MerchantError::ItemNotFound)?;
    let buyer = shop.buyer(customer.player);
    if !buyer.offered(&template) {
        return Err(MerchantError::ItemNotFound);
    }
    let goods = shop
        .catalog
        .goods
        .get(&batch.item_id)
        .cloned()
        .ok_or(MerchantError::ItemNotFound)?;
    let (_, price) = model::sell_stock(merchant, batch.id, count).map_err(|error| {
        error!("economic vendor {entry} quote: {error}");
        MerchantError::NotEnoughMoney
    })?;
    let mut inventory = shop.load(customer.character_id)?;
    if inventory.gold() < price {
        return Err(MerchantError::NotEnoughMoney);
    }
    if goods.max_count > 0 && inventory.owned_count(batch.item_id) + count > goods.max_count {
        return Err(MerchantError::CantCarryMore);
    }
    store_purchase_item(
        shop,
        &mut inventory,
        &goods,
        template,
        batch.item_id,
        count,
        req.destination,
    )?;
    inventory.set_gold(inventory.gold() - price);
    let excess = model::credit_capital(merchant, price, business.capital_limit);
    state.withdrawn = state
        .withdrawn
        .checked_add(excess)
        .ok_or(MerchantError::TooMuchGold)?;
    let gold = inventory.gold();
    let changes = MarketChanges {
        records: vec![(customer.character_id, inventory.into_record())],
        ..Default::default()
    };
    store::commit_market(shop.db(), Some(&state), &changes).map_err(|error| {
        error!("economic vendor {entry} purchase aborted: {error}");
        MerchantError::ItemNotFound
    })?;
    shop.economy
        .as_mut()
        .expect("economic state validated")
        .state = state;
    shop.gold.record(customer.character_id, gold);
    shop.send_inventory(customer.connection, vendor, buyer);
    Ok(())
}

fn store_purchase_item(
    shop: &Shop,
    inventory: &mut InventoryModel,
    goods: &VendorGoods,
    template: ItemTemplate,
    item_id: u32,
    count: u32,
    destination: Option<shared::protocol::ItemLocation>,
) -> MerchantResult {
    let mut mint = shop.mint();
    let item = InventoryItemData {
        item_guid: mint(),
        item_id,
        name: goods.name.clone(),
        quality: goods.quality,
        required_level: template.required_level,
        stack_count: count,
        vendor_sell_price: goods.sell_price,
        unique_auction: false,
    };
    let result = match destination {
        Some(location) => inventory.store_item_at(item, template, location, &mut mint),
        None => inventory.store_item(item, template, &mut mint),
    };
    result.map_err(|error| match error.reason {
        InventoryErrorReason::CantStack => MerchantError::CantStack,
        _ => MerchantError::InventoryFull,
    })
}
