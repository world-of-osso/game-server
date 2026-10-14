//! Owned merchant inputs executed by the single persistence writer.
use super::*;
use shared::currency::CurrencyWallet;
mod economic;

#[derive(Clone)]
pub(super) enum Operation {
    Buy(BuyItem),
    Sell(SellItem),
    Junk(SellAllJunkItems),
    Buyback(BuybackItemRequest),
    Repair(RepairItem),
}

#[derive(Clone, Default)]
pub(super) struct Publication {
    pub gold: Option<u64>,
    pub criteria: Vec<crate::achievement::CriteriaEvent>,
    pub acquisitions: Vec<(u32, u32)>,
    pub buyback: Option<BuybackSlots>,
    pub stock: Option<VendorStock>,
    pub durability: Option<EquippedDurability>,
    pub wallet: Option<CurrencyWallet>,
    pub economy: Option<crate::economy::model::EconomyState>,
    pub guild: Option<u32>,
    pub refresh_inventory: bool,
    pub refresh_buyback: bool,
    pub refusal: Option<MerchantError>,
}
impl Publication {
    fn record(&mut self, _: u64, gold: u64) {
        self.gold = Some(gold);
    }
    fn record_acquisition(&mut self, _: u64, item: u32, count: u32) {
        self.acquisitions.push((item, count));
    }
    fn record_criteria(&mut self, _: u64, event: crate::achievement::CriteriaEvent) {
        self.criteria.push(event);
    }
}

pub(super) struct Inputs {
    pub templates: ItemTemplates,
    pub catalog: VendorCatalog,
    pub quests: Option<crate::quest_data::QuestDb>,
    pub business: Option<crate::economy::model::MerchantConfig>,
    pub stock: VendorStock,
    pub buyer: Buyer,
    pub entry: u32,
    pub can_repair: bool,
    pub reputation: Option<shared::reputation::CharacterReputation>,
    pub durability_rules: ItemDurability,
    pub durability: Option<EquippedDurability>,
    pub has_wallet: bool,
    pub reset: DailyReset,
    pub now: f64,
}

struct Transaction<'a> {
    database: &'a Database,
    templates: ItemTemplates,
    catalog: VendorCatalog,
    quests: Option<crate::quest_data::QuestDb>,
    business: Option<crate::economy::model::MerchantConfig>,
    economy: Option<crate::economy::Economy>,
    stock: VendorStock,
    buyer: Buyer,
    entry: u32,
    can_repair: bool,
    reputation: Option<shared::reputation::CharacterReputation>,
    durability_rules: ItemDurability,
    durability: Option<EquippedDurability>,
    wallet: Option<CurrencyWallet>,
    reset: DailyReset,
    now: f64,
    gold: Publication,
    persistence_error: Option<String>,
}

pub(super) fn execute(
    db: &Database,
    inputs: Inputs,
    mut buyback: BuybackSlots,
    connection: Entity,
    character_id: u64,
    vendor: Entity,
    operation: Operation,
) -> Result<Publication, String> {
    let economy = crate::economy::store::load_state(db)?.map(crate::economy::Economy::from_state);
    let wallet = inputs
        .has_wallet
        .then(|| persistence::load_currencies(db, character_id));
    let mut shop = Transaction {
        database: db,
        templates: inputs.templates,
        catalog: inputs.catalog,
        quests: inputs.quests,
        business: inputs.business,
        economy,
        stock: inputs.stock,
        buyer: inputs.buyer,
        entry: inputs.entry,
        can_repair: inputs.can_repair,
        reputation: inputs.reputation,
        durability_rules: inputs.durability_rules,
        durability: inputs.durability,
        wallet,
        reset: inputs.reset,
        now: inputs.now,
        gold: Publication::default(),
        persistence_error: None,
    };
    let mut customer = Customer {
        connection,
        character_id,
        player: shop.buyer.player,
        vendor: Some(vendor),
        buyback: &mut buyback,
    };
    let before_durability = shop.durability.clone();
    shop.gold.refusal = match operation {
        Operation::Buy(req) => buy(&mut shop, &mut customer, req),
        Operation::Sell(req) => sell(&mut shop, &mut customer, req),
        Operation::Junk(req) => sell_all_junk(&mut shop, &mut customer, req),
        Operation::Buyback(req) => buy_back(&mut shop, &mut customer, req),
        Operation::Repair(req) => repair(&mut shop, &mut customer, req),
    }
    .err();
    if let Some(error) = shop.persistence_error {
        return Err(error);
    }
    if shop.durability != before_durability {
        shop.gold.durability = shop.durability;
    }
    if shop.gold.refresh_buyback {
        shop.gold.buyback = Some(buyback);
    }
    if shop.gold.refresh_inventory {
        shop.gold.stock = Some(shop.stock);
    }
    Ok(shop.gold)
}

impl Transaction<'_> {
    fn db(&self) -> &Database {
        self.database
    }
    fn now(&self) -> f64 {
        self.now
    }
    fn buyer(&self, _: Entity) -> Buyer {
        self.buyer
    }
    fn vendor_entry(&self, _: Entity) -> Result<u32, MerchantError> {
        Ok(self.entry)
    }
    fn template(
        &self,
        source: shared::item_data::ItemDefinitionSource,
        item: u32,
    ) -> Option<ItemTemplate> {
        self.templates.get(source, item).copied()
    }
    fn mint(&self) -> impl FnMut() -> u64 + '_ {
        move || persistence::next_id(self.database, "item_instance")
    }
    fn load(&self, character: u64) -> Result<InventoryModel, MerchantError> {
        load_model(self.database, &self.templates, character)
            .map_err(|_| MerchantError::ItemNotFound)
    }
    fn save(&mut self, character: u64, model: InventoryModel) {
        let gold = model.gold();
        save_inventory_record(self.database, character, &model.into_record());
        self.gold.record(character, gold);
    }
    fn mount_reputation_met(&self, _: Buyer, item: u32) -> bool {
        self.catalog
            .mount_reputation
            .get(&item)
            .is_none_or(|requirement| requirement.met(self.reputation.as_ref()))
    }
    fn send_inventory(&mut self, _: Entity, _: Entity, _: Buyer, _: u64) {
        self.gold.refresh_inventory = true;
    }
    fn send_buyback(&mut self, _: Entity, _: &BuybackSlots) {
        self.gold.refresh_buyback = true;
    }
}

/// AzerothCore `Player::BuyItemFromVendorSlot`: `count` purchases of
/// `VendorStackCount` items each, `BuyPrice` copper per purchase.
fn buy(shop: &mut Transaction, customer: &mut Customer, req: BuyItem) -> MerchantResult {
    let vendor = customer.vendor_for(req.npc)?;
    let entry = shop.vendor_entry(vendor)?;
    if shop.business.is_some() {
        return economic::purchase(shop, customer, vendor, entry, req);
    }
    let slot = shop
        .catalog
        .items(entry)
        .get(req.slot as usize)
        .copied()
        .filter(|slot| slot.item_id == req.item_id)
        .ok_or(MerchantError::ItemNotFound)?;
    let mut goods = shop
        .catalog
        .goods
        .get(&slot.item_id)
        .cloned()
        .ok_or(MerchantError::ItemNotFound)?;
    let template = shop
        .template(
            shared::item_data::ItemDefinitionSource::Retail,
            slot.item_id,
        )
        .ok_or(MerchantError::ItemNotFound)?;
    let buyer = shop.buyer(customer.player);
    if !buyer.offered(&template) {
        return Err(MerchantError::ItemNotFound);
    }
    if !shop.mount_reputation_met(buyer, slot.item_id) {
        return Err(MerchantError::ReputationRequired);
    }
    let mount_cost = shop.catalog.mount_costs.get(&(entry, req.slot)).cloned();
    if let Some(cost) = &mount_cost {
        goods.buy_price = cost.money;
    }
    let count = if mount_cost.is_some() {
        req.count.max(1)
    } else {
        match goods.buy_price {
            0 => req.count.max(1),
            price => req
                .count
                .max(1)
                .min(u32::try_from(MAX_MONEY / u64::from(price)).unwrap_or(u32::MAX)),
        }
    };
    let items = goods
        .buy_count
        .checked_mul(count)
        .ok_or(MerchantError::CantCarryMore)?;
    let now = shop.now();
    let available = shop
        .stock
        .available(vendor, req.slot, slot, goods.buy_count, now);
    if available.is_some_and(|left| left < items) {
        return Err(MerchantError::SoldOut);
    }
    let price = u64::from(goods.buy_price) * u64::from(count);
    let mut model = shop.load(customer.character_id)?;
    if model.gold() < price {
        return Err(MerchantError::NotEnoughMoney);
    }
    if goods.max_count > 0
        && model.owned_count(
            shared::item_data::ItemDefinitionSource::Retail,
            slot.item_id,
        ) + items
            > goods.max_count
    {
        return Err(MerchantError::CantCarryMore);
    }
    let wallet = mount_cost
        .as_ref()
        .map(|cost| debit_turnins(shop, &mut model, cost, count))
        .transpose()?
        .flatten();
    let mut mint = shop.mint();
    let item = InventoryItemData {
        item_guid: mint(),
        item_id: slot.item_id,
        name: goods.name.clone(),
        quality: goods.quality,
        required_level: template.required_level,
        stack_count: items,
        vendor_sell_price: goods.sell_price,
        unique_auction: false,
    };
    match req.destination {
        Some(location) => model.store_item_at(
            item,
            shared::item_data::ItemDefinitionSource::Retail,
            template,
            location,
            &mut mint,
        ),
        None => model.store_item(
            item,
            shared::item_data::ItemDefinitionSource::Retail,
            template,
            &mut mint,
        ),
    }
    .map_err(|error| match error.reason {
        InventoryErrorReason::CantStack => MerchantError::CantStack,
        _ => MerchantError::InventoryFull,
    })?;
    drop(mint);
    model.set_gold(model.gold() - price);
    if mount_cost.is_some() {
        persist_purchase(shop, customer, model, wallet);
    } else {
        shop.save(customer.character_id, model);
    }
    shop.gold
        .record_acquisition(customer.character_id, slot.item_id, items);
    // TC Player::BuyItemFromVendorSlot (Player.cpp:23902): one committed purchase.
    shop.gold.record_criteria(
        customer.character_id,
        crate::achievement::CriteriaEvent::BuyItemsFromVendors,
    );
    if let Some(quests) = &shop.quests {
        let mut log =
            crate::persistence::quest_store::load_quest_log(shop.db(), customer.character_id);
        if crate::housing_quests::credit_acquired_item(quests, &mut log, slot.item_id, items) {
            crate::persistence::quest_store::save_quest_log(shop.db(), customer.character_id, &log);
        }
    }
    if slot.maxcount > 0 {
        shop.stock
            .consume(vendor, req.slot, slot, goods.buy_count, items, now);
        shop.send_inventory(customer.connection, vendor, buyer, customer.character_id);
    }
    Ok(())
}

/// AzerothCore `HandleSellItemOpcode`: items without a sell price are refused;
/// the sale moves the item into a buyback slot priced at what it paid.
fn sell(shop: &mut Transaction, customer: &mut Customer, req: SellItem) -> MerchantResult {
    customer.vendor_for(req.npc)?;
    let mut model = shop.load(customer.character_id)?;
    let item = model
        .bag_item(req.item_guid)
        .cloned()
        .ok_or(MerchantError::ItemNotFound)?;
    let count = if req.count == 0 {
        item.stack_count
    } else {
        req.count
    };
    if count > item.stack_count {
        return Err(MerchantError::NotInterested);
    }
    let sell_price = item.vendor_sell_price;
    if sell_price == 0 {
        return Err(MerchantError::NotInterested);
    }
    let money = u64::from(sell_price) * u64::from(count);
    if model.gold() + money > MAX_MONEY {
        return Err(MerchantError::TooMuchGold);
    }
    let mut mint = shop.mint();
    let sold = model
        .take_bag_item(req.item_guid, count, &mut mint)
        .map_err(|_| MerchantError::ItemNotFound)?;
    drop(mint);
    model.collect_disposed_appearance(&sold);
    model.set_gold(model.gold() + money);
    shop.save(customer.character_id, model);
    // TC Player::SellItemToVendor (Player.cpp:23946-23950).
    for event in [
        crate::achievement::CriteriaEvent::MoneyEarnedFromSales(money),
        crate::achievement::CriteriaEvent::SellItemsToVendors,
    ] {
        shop.gold.record_criteria(customer.character_id, event);
    }
    let sold_at = shop.now();
    customer.buyback.add(BuybackEntry {
        item: sold,
        price: sell_price.saturating_mul(count),
        sold_at,
    });
    shop.send_buyback(customer.connection, customer.buyback);
    Ok(())
}

/// TrinityCore `HandleSellAllJunkItems` (ItemHandler.cpp:427-484): every
/// poor-quality bag stack the vendor buys is sold whole in bag order, each into a
/// buyback slot. The first refused sale (gold cap) ends it without a message,
/// as TrinityCore discards the `SellItemToVendor` error.
fn sell_all_junk(
    shop: &mut Transaction,
    customer: &mut Customer,
    req: SellAllJunkItems,
) -> MerchantResult {
    customer.vendor_for(req.npc)?;
    let junk: Vec<u64> = shop
        .load(customer.character_id)?
        .bag_items()
        .filter(|item| item.quality == QUALITY_POOR && item.vendor_sell_price > 0)
        .map(|item| item.item_guid)
        .collect();
    for item_guid in junk {
        let sale = SellItem {
            npc: req.npc,
            item_guid,
            count: 0,
        };
        if sell(shop, customer, sale).is_err() {
            break;
        }
    }
    Ok(())
}

/// AzerothCore `HandleBuybackItem`: pay the sale price back and store the item.
fn buy_back(
    shop: &mut Transaction,
    customer: &mut Customer,
    req: BuybackItemRequest,
) -> MerchantResult {
    customer.vendor_for(req.npc)?;
    let entry = customer
        .buyback
        .get(req.slot)
        .cloned()
        .ok_or(MerchantError::ItemNotFound)?;
    let mut model = shop.load(customer.character_id)?;
    if model.gold() < u64::from(entry.price) {
        return Err(MerchantError::NotEnoughMoney);
    }
    let source =
        persistence::inventory_store::load_item_definition_source(shop.db(), entry.item.item_guid);
    let template = shop
        .template(source, entry.item.item_id)
        .ok_or(MerchantError::ItemNotFound)?;
    let mut mint = shop.mint();
    model
        .store_item(entry.item, source, template, &mut mint)
        .map_err(|_| MerchantError::InventoryFull)?;
    drop(mint);
    model.set_gold(model.gold() - u64::from(entry.price));
    shop.save(customer.character_id, model);
    customer.buyback.remove(req.slot);
    shop.send_buyback(customer.connection, customer.buyback);
    Ok(())
}

/// AzerothCore `DurabilityRepairAll` / `DurabilityRepair`: each damaged item costs
/// `ItemDurability::repair_cost`; an item that costs more than the money left stays
/// damaged (AzerothCore skips it silently; we answer "You don't have enough money.").
/// Per-item repair also accepts damaged owned bag items.
fn repair(shop: &mut Transaction, customer: &mut Customer, req: RepairItem) -> MerchantResult {
    let vendor = customer.vendor_for(req.npc)?;
    let repairs = shop.can_repair;
    if !repairs {
        return Err(MerchantError::NotInteracting);
    }
    // TrinityCore `HandleRepairItemOpcode` uses the guild bank for repair-all only.
    if req.guild_bank && req.item_guid.is_none() {
        return guild_repair(shop, customer, vendor);
    }
    let mut model = shop.load(customer.character_id)?;
    // TrinityCore `Player::DurabilityRepair` resolves any owned item by position,
    // not just equipment. The request identifies that item by guid.
    let only: Option<EquipmentVisualSlot> = match req.item_guid {
        None => None,
        Some(guid) => match model.equipped_visual_slot(guid) {
            Some(slot) => Some(slot),
            None => return repair_bag_item(shop, customer, model, guid),
        },
    };
    let Ok(durability) = shop.durability.as_mut().ok_or(()) else {
        return Err(MerchantError::ItemNotFound);
    };
    if let Some(slot) = only {
        let damaged = durability
            .slots
            .iter()
            .any(|entry| entry.slot == slot && entry.current < entry.max);
        if !damaged {
            return Err(MerchantError::ItemNotFound);
        }
    }
    let mut gold = model.gold();
    let mut short = false;
    for entry in durability
        .slots
        .iter_mut()
        .filter(|entry| only.is_none_or(|slot| slot == entry.slot))
    {
        let Some(cost) = shop
            .durability_rules
            .repair_cost(entry.item_id, entry.max - entry.current)
        else {
            continue;
        };
        if cost == 0 {
            continue;
        }
        if u64::from(cost) > gold {
            short = true;
            continue;
        }
        gold -= u64::from(cost);
        entry.current = entry.max;
    }
    if gold != model.gold() {
        model.set_gold(gold);
        shop.save(customer.character_id, model);
    }
    if short {
        return Err(MerchantError::NotEnoughMoney);
    }
    Ok(())
}

/// Repair an owned bag item and save its damage and payment in one transaction.
fn repair_bag_item(
    shop: &mut Transaction,
    customer: &Customer,
    mut model: InventoryModel,
    guid: u64,
) -> MerchantResult {
    let item = model.bag_item(guid).ok_or(MerchantError::ItemNotFound)?;
    let state = model
        .bag_durability(guid)
        .ok_or(MerchantError::ItemNotFound)?;
    let cost = shop
        .durability_rules
        .repair_cost(item.item_id, state.max.saturating_sub(state.current))
        .filter(|&cost| cost > 0)
        .ok_or(MerchantError::ItemNotFound)?;
    if u64::from(cost) > model.gold() {
        return Err(MerchantError::NotEnoughMoney);
    }
    model.restore_bag_durability(guid);
    model.set_gold(model.gold() - u64::from(cost));
    shop.save(customer.character_id, model);
    Ok(())
}

/// TrinityCore `Player::DurabilityRepairAll(guildBank = true)` (Player.cpp:4636-4669):
/// damaged items are repaired cheapest first while the guild money left for the player
/// (`GetMemberAvailableMoneyForRepairItems`) covers them, and the total is taken once
/// (`Guild::HandleMemberWithdrawMoney(repair = true)`); the player's own money is never
/// touched. TrinityCore answers nothing; we answer `ERR_GUILD_PERMISSIONS` to a rank
/// without `GR_RIGHT_WITHDRAW_REPAIR` and `ERR_GUILD_NOT_ENOUGH_MONEY` when items stay
/// damaged, as the personal repair answers `ERR_NOT_ENOUGH_MONEY`.
fn guild_repair(shop: &mut Transaction, customer: &mut Customer, vendor: Entity) -> MerchantResult {
    let day = shop.reset.day(unix_now());
    let available = guild_repair_money(shop.db(), customer.character_id, day)
        .ok_or(MerchantError::GuildPermissions)?;
    let Ok(durability) = shop.durability.as_ref().ok_or(()) else {
        return Err(MerchantError::ItemNotFound);
    };
    let mut costs: Vec<(usize, u64)> = durability
        .slots
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let cost = shop
                .durability_rules
                .repair_cost(entry.item_id, entry.max - entry.current)?;
            (cost > 0).then_some((index, u64::from(cost)))
        })
        .collect();
    costs.sort_by_key(|&(_, cost)| cost);
    let mut total = 0;
    let mut repaired = Vec::new();
    for &(index, cost) in &costs {
        if total + cost > available {
            break;
        }
        total += cost;
        repaired.push(index);
    }
    if total > 0 {
        let durability = shop.durability.as_mut().expect("read above");
        for index in &repaired {
            let entry = &mut durability.slots[*index];
            entry.current = entry.max;
        }
        let guild_id = pay_guild_repair(shop.db(), customer.character_id, day, total);
        shop.gold.guild = Some(guild_id);
        let buyer = shop.buyer(customer.player);
        shop.send_inventory(customer.connection, vendor, buyer, customer.character_id);
    }
    if repaired.len() < costs.len() {
        return Err(MerchantError::GuildNotEnoughMoney);
    }
    Ok(())
}

fn debit_turnins(
    shop: &Transaction,
    model: &mut InventoryModel,
    cost: &mount_costs::MountCost,
    count: u32,
) -> Result<Option<CurrencyWallet>, MerchantError> {
    let mut wallet = if cost.extra.iter().any(|cost| cost.currency) {
        Some(shop.wallet.clone().ok_or(MerchantError::MissingTurnins)?)
    } else {
        None
    };
    for turnin in &cost.extra {
        let amount = turnin
            .amount
            .checked_mul(count)
            .ok_or(MerchantError::MissingTurnins)?;
        if turnin.currency {
            let entry = wallet
                .as_mut()
                .expect("currency payment wallet")
                .entries
                .iter_mut()
                .find(|entry| entry.id == turnin.id)
                .ok_or(MerchantError::MissingTurnins)?;
            entry.amount = entry
                .amount
                .checked_sub(amount)
                .ok_or(MerchantError::MissingTurnins)?;
        } else {
            model
                .destroy_bag_items(
                    shared::item_data::ItemDefinitionSource::Retail,
                    turnin.id,
                    amount,
                )
                .map_err(|_| MerchantError::MissingTurnins)?;
        }
    }
    Ok(wallet)
}

/// Gold, token destruction, delivery and currency share the same durable boundary.
fn persist_purchase(
    shop: &mut Transaction,
    customer: &Customer,
    model: InventoryModel,
    wallet: Option<CurrencyWallet>,
) {
    let gold = model.gold();
    let txn = shop.db().begin_write().expect("begin mount purchase");
    persistence::inventory_store::write_inventory_record(
        &txn,
        customer.character_id,
        &model.into_record(),
    );
    if let Some(wallet) = &wallet {
        let bytes = serde_json::to_vec(wallet).expect("encode mount payment wallet");
        txn.open_table(persistence::CURRENCIES)
            .expect("open mount payment currencies")
            .insert(customer.character_id, bytes.as_slice())
            .expect("write mount payment currencies");
    }
    txn.commit().expect("commit mount payment and delivery");
    shop.gold.record(customer.character_id, gold);
    if let Some(wallet) = wallet {
        shop.wallet = Some(wallet.clone());
        shop.gold.wallet = Some(wallet);
    }
}
