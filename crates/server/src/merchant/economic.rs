//! Finite crafted stock; static vendor rules remain for unassigned NPCs.
use super::*;
use crate::economy::model;

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
        guild_repair_money: None,
        items,
    }
}

fn offered_item(shop: &Shop, buyer: Buyer, batch: &model::StockBatch) -> Option<VendorItem> {
    let template = shop.template(
        shared::item_data::ItemDefinitionSource::Retail,
        batch.item_id,
    )?;
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
        extra_costs: Vec::new(),
        stack_count: 1,
        max_stack: template.max_stack,
        num_available: Some(batch.count),
        usable: buyer.can_use(&template),
        max_durability: shop.durability_rules.max_durability(batch.item_id),
    })
}
