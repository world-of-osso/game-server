use super::*;
use crate::economy::{
    model,
    store::{self, MarketChanges},
};
use crate::merchant::catalog::VendorGoods;

struct Purchase {
    item_id: u32,
    count: u32,
    template: ItemTemplate,
    buyer: Buyer,
    goods: VendorGoods,
    price: u64,
}

pub(super) fn purchase(
    shop: &mut Transaction,
    customer: &mut Customer,
    vendor: Entity,
    entry: u32,
    req: BuyItem,
) -> MerchantResult {
    let (business, mut state) = clone_purchase_state(shop)?;
    let merchant = state
        .merchants
        .get_mut(&entry)
        .ok_or(MerchantError::ItemNotFound)?;
    let purchase = validate_and_sell_requested_stock(shop, customer, merchant, entry, &req)?;
    let (inventory, expected_inventory) =
        load_and_store_purchase(shop, customer, &purchase, req.destination)?;
    let excess = model::credit_capital(merchant, purchase.price, business.capital_limit);
    state.withdrawn = state
        .withdrawn
        .checked_add(excess)
        .ok_or(MerchantError::TooMuchGold)?;
    commit_and_publish_purchase(
        shop,
        customer,
        vendor,
        entry,
        purchase.buyer,
        state,
        inventory,
        expected_inventory,
    )?;
    shop.gold
        .record_acquisition(customer.character_id, purchase.item_id, purchase.count);
    Ok(())
}

fn clone_purchase_state(
    shop: &Transaction,
) -> Result<(model::MerchantConfig, model::EconomyState), MerchantError> {
    let business = shop.business.clone().ok_or(MerchantError::ItemNotFound)?;
    let state = shop
        .economy
        .as_ref()
        .ok_or(MerchantError::ItemNotFound)?
        .state
        .clone();
    Ok((business, state))
}

fn validate_and_sell_requested_stock(
    shop: &Transaction,
    customer: &Customer,
    merchant: &mut model::MerchantState,
    entry: u32,
    req: &BuyItem,
) -> Result<Purchase, MerchantError> {
    let batch = merchant
        .stock
        .iter()
        .find(|batch| batch.id == u64::from(req.slot) && batch.item_id == req.item_id)
        .cloned()
        .ok_or(MerchantError::ItemNotFound)?;
    let count = validate_stock_count(&batch, req.count, crate::auction_house::unix_now())?;
    let (template, buyer, goods) = validate_offered_goods(shop, customer.player, batch.item_id)?;
    let (_, price) = model::sell_stock(merchant, batch.id, count).map_err(|error| {
        error!("economic vendor {entry} quote: {error}");
        MerchantError::NotEnoughMoney
    })?;
    Ok(Purchase {
        item_id: batch.item_id,
        count,
        template,
        buyer,
        goods,
        price,
    })
}

fn validate_offered_goods(
    shop: &Transaction,
    player: Entity,
    item_id: u32,
) -> Result<(ItemTemplate, Buyer, VendorGoods), MerchantError> {
    let template = shop
        .template(shared::item_data::ItemDefinitionSource::Retail, item_id)
        .ok_or(MerchantError::ItemNotFound)?;
    let buyer = shop.buyer(player);
    if !buyer.offered(&template) {
        return Err(MerchantError::ItemNotFound);
    }
    let goods = shop
        .catalog
        .goods
        .get(&item_id)
        .cloned()
        .ok_or(MerchantError::ItemNotFound)?;
    Ok((template, buyer, goods))
}

fn validate_stock_count(
    batch: &model::StockBatch,
    requested: u32,
    now: u64,
) -> Result<u32, MerchantError> {
    if batch.expires_at <= now {
        return Err(MerchantError::SoldOut);
    }
    let count = requested.max(1);
    if count > batch.count {
        return Err(MerchantError::SoldOut);
    }
    Ok(count)
}

fn validate_purchase_inventory(inventory: &InventoryModel, purchase: &Purchase) -> MerchantResult {
    if inventory.gold() < purchase.price {
        return Err(MerchantError::NotEnoughMoney);
    }
    if purchase.goods.max_count > 0
        && inventory.owned_count(
            shared::item_data::ItemDefinitionSource::Retail,
            purchase.item_id,
        ) + purchase.count
            > purchase.goods.max_count
    {
        return Err(MerchantError::CantCarryMore);
    }
    Ok(())
}

fn load_and_store_purchase(
    shop: &Transaction,
    customer: &Customer,
    purchase: &Purchase,
    destination: Option<shared::protocol::ItemLocation>,
) -> Result<(InventoryModel, persistence::InventoryData), MerchantError> {
    let expected_inventory = persistence::load_inventory(shop.db(), customer.character_id);
    let mut inventory = shop.load(customer.character_id)?;
    validate_purchase_inventory(&inventory, purchase)?;
    store_purchase_item(
        shop,
        &mut inventory,
        &purchase.goods,
        purchase.template,
        purchase.item_id,
        purchase.count,
        destination,
    )?;
    inventory.set_gold(inventory.gold() - purchase.price);
    Ok((inventory, expected_inventory))
}

fn commit_and_publish_purchase(
    shop: &mut Transaction,
    customer: &Customer,
    vendor: Entity,
    entry: u32,
    buyer: Buyer,
    state: model::EconomyState,
    inventory: InventoryModel,
    expected_inventory: persistence::InventoryData,
) -> MerchantResult {
    let gold = inventory.gold();
    let changes = MarketChanges {
        expected_inventories: vec![(customer.character_id, expected_inventory)],
        records: vec![(customer.character_id, inventory.into_record())],
        ..Default::default()
    };
    store::commit_market(shop.db(), Some(&state), &changes).map_err(|error| {
        shop.persistence_error = Some(format!("economic vendor {entry} purchase aborted: {error}"));
        MerchantError::ItemNotFound
    })?;
    shop.gold.economy = Some(state);
    shop.gold.record(customer.character_id, gold);
    shop.send_inventory(customer.connection, vendor, buyer, customer.character_id);
    Ok(())
}

fn store_purchase_item(
    shop: &Transaction,
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
        Some(location) => inventory.store_item_at(
            item,
            shared::item_data::ItemDefinitionSource::Retail,
            template,
            location,
            &mut mint,
        ),
        None => inventory.store_item(
            item,
            shared::item_data::ItemDefinitionSource::Retail,
            template,
            &mut mint,
        ),
    };
    result.map_err(|error| match error.reason {
        InventoryErrorReason::CantStack => MerchantError::CantStack,
        _ => MerchantError::InventoryFull,
    })
}
