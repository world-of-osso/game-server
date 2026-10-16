//! Vendors (docs/wiki/systems/merchant.md): vendor lists, buy, sell, buyback and
//! repair for the NPC whose vendor frame `npc_interaction` has open.
//!
//! Rules follow AzerothCore: `WorldSession::SendListInventory`,
//! `HandleSellItemOpcode` and `HandleBuybackItem` (Handlers/ItemHandler.cpp) and
//! `Player::BuyItemFromVendorSlot` (Player.cpp:10651); repair is
//! `HandleRepairItemOpcode` (Handlers/NPCHandler.cpp:748) with
//! `Player::DurabilityRepairAll`/`DurabilityRepair` (Player.cpp:4736-4752); guild bank
//! repairs follow TrinityCore `DurabilityRepairAll(guildBank = true)`. A request whose NPC is not
//! the connection's open vendor is refused; `close_invalid_interactions` already
//! closes the frame once the player walks out of range. Money and bags change in
//! one `InventoryModel` save and reach the owner through `GoldUpdateQueue`.
//!
//! Gaps: no reputation price discount (`GetReputationPriceDiscount`; NPC factions
//! are not mapped to reputation factions), no non-mount `ExtendedCost` items, no vendor
//! conditions and no sell price reduction for damaged items.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use redb::Database;
use shared::components::{EquipmentVisualSlot, Level, Npc, Player};
use shared::protocol::{
    BuyItem, BuybackItem, BuybackItemRequest, BuybackList, InventoryErrorReason, MerchantChannel,
    MerchantError, MerchantFailed, NpcFlags, NpcRole, RepairItem, SellAllJunkItems, SellItem,
    VendorInventory, VendorItem,
};

use crate::auth::InWorld;
use crate::durability::{EquippedDurability, ItemDurability};
use crate::faction::faction_for_race;
use crate::gold::{GoldUpdateQueue, MAX_MONEY};
use crate::guild_bank::{
    ChangedGuilds, DailyReset, guild_repair_money, pay_guild_repair, unix_now,
};
use crate::inventory::{inventory_model::InventoryModel, load_model};
use crate::item_templates::{BIND_ON_PICKUP, ItemTemplate, ItemTemplates};
use crate::networking::PlayerLink;
use crate::npc_interaction::ActiveInteraction;
use crate::persistence::inventory_store::save_inventory_record;
use crate::persistence::{self, GameDatabase, InventoryItemData};

mod catalog;
mod economic;
mod mount_costs;
mod stock;

use catalog::VendorSlot;
pub use catalog::{VendorCatalog, load_vendor_catalog};
pub(crate) use catalog::{VendorGoods, load_skyborne_goods};
use stock::{BuybackEntry, BuybackSlots, VendorStock};

/// Retail `ITEM_FLAG2_FACTION_HORDE` / `ITEM_FLAG2_FACTION_ALLIANCE` (`Flags_1`).
const FLAG2_FACTION_HORDE: i64 = 0x1;
const FLAG2_FACTION_ALLIANCE: i64 = 0x2;
const TEAM_ALLIANCE: u8 = 1;
const TEAM_HORDE: u8 = 2;
/// `ITEM_QUALITY_POOR`.
const QUALITY_POOR: u8 = 0;

pub struct MerchantPlugin;

impl Plugin for MerchantPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VendorCatalog>()
            .init_resource::<VendorStock>()
            .init_resource::<DailyReset>()
            .init_resource::<ChangedGuilds>()
            .add_systems(
                Update,
                (
                    init_buyback_slots,
                    send_vendor_lists,
                    process_buy_requests,
                    process_sell_requests,
                    process_sell_all_junk_requests,
                    process_buyback_requests,
                    process_repair_requests,
                )
                    .chain(),
            );
    }
}

type MerchantResult = Result<(), MerchantError>;

/// Race, class and level of the player.
#[derive(Debug, Clone, Copy)]
struct Buyer {
    player: Entity,
    race: u8,
    class: u8,
    level: u16,
}

impl Buyer {
    /// AzerothCore `SendListInventory` filters: bind-on-pickup items of other
    /// classes, and items of the other faction.
    fn offered(self, template: &ItemTemplate) -> bool {
        let class_ok = template.allowable_class == -1
            || (1u64 << self.class.saturating_sub(1)) & template.allowable_class as u64 != 0;
        if !class_ok && template.bonding == BIND_ON_PICKUP {
            return false;
        }
        let flags2 = template.scaling.flags2;
        match faction_for_race(self.race) {
            TEAM_ALLIANCE => flags2 & FLAG2_FACTION_HORDE == 0,
            TEAM_HORDE => flags2 & FLAG2_FACTION_ALLIANCE == 0,
            _ => true,
        }
    }

    fn can_use(self, template: &ItemTemplate) -> bool {
        template.allows(self.class, self.race) && template.required_level <= self.level
    }
}

/// Everything a vendor request reads and writes besides the connection.
#[derive(SystemParam)]
struct Shop<'w, 's> {
    db: Option<Res<'w, GameDatabase>>,
    templates: Option<Res<'w, ItemTemplates>>,
    catalog: Res<'w, VendorCatalog>,
    quests: Option<Res<'w, crate::quest_data::QuestDb>>,
    economy_config: Option<Res<'w, crate::economy::Config>>,
    economy: Option<ResMut<'w, crate::economy::Economy>>,
    stock: ResMut<'w, VendorStock>,
    time: Res<'w, Time>,
    gold: ResMut<'w, GoldUpdateQueue>,
    npcs: Query<'w, 's, (&'static Npc, &'static NpcFlags)>,
    players: Query<'w, 's, (&'static Player, Option<&'static Level>)>,
    durability_rules: Res<'w, ItemDurability>,
    durability: Query<'w, 's, &'static mut EquippedDurability>,
    reputations: Query<'w, 's, &'static shared::reputation::CharacterReputation>,
    currencies: Query<'w, 's, &'static mut crate::currency::PlayerCurrencies>,
    currency_updates:
        Query<'w, 's, &'static mut MessageSender<shared::protocol::CurrencyStateUpdate>>,
    reset: Res<'w, DailyReset>,
    changed_guilds: ResMut<'w, ChangedGuilds>,
    inventories: Query<'w, 's, &'static mut MessageSender<VendorInventory>>,
    buybacks: Query<'w, 's, &'static mut MessageSender<BuybackList>>,
    failures: Query<'w, 's, &'static mut MessageSender<MerchantFailed>>,
}

impl Shop<'_, '_> {
    fn ready(&self) -> bool {
        self.db.is_some() && self.templates.is_some()
    }

    fn db(&self) -> &Database {
        &self.db.as_ref().expect("checked by ready").db
    }

    fn template(
        &self,
        source: shared::item_data::ItemDefinitionSource,
        item_id: u32,
    ) -> Option<ItemTemplate> {
        self.templates.as_ref()?.get(source, item_id).copied()
    }

    fn now(&self) -> f64 {
        self.time.elapsed_secs_f64()
    }

    fn buyer(&self, player: Entity) -> Buyer {
        let (race, class, level) = self.players.get(player).map_or((0, 0, 1), |(p, level)| {
            (p.race, p.class, level.map_or(1, |level| u16::from(level.0)))
        });
        Buyer {
            player,
            race,
            class,
            level,
        }
    }

    fn mount_reputation_met(&self, buyer: Buyer, item_id: u32) -> bool {
        self.catalog
            .mount_reputation
            .get(&item_id)
            .is_none_or(|requirement| requirement.met(self.reputations.get(buyer.player).ok()))
    }

    fn vendor_entry(&self, vendor: Entity) -> Result<u32, MerchantError> {
        self.npcs
            .get(vendor)
            .map(|(npc, _)| npc.template_id)
            .map_err(|_| MerchantError::NotInteracting)
    }

    fn mint(&self) -> impl FnMut() -> u64 + '_ {
        let db = self.db();
        move || persistence::next_id(db, "item_instance")
    }

    fn load(&self, character_id: u64) -> Result<InventoryModel, MerchantError> {
        let templates = self.templates.as_ref().expect("checked by ready");
        load_model(self.db(), templates, character_id).map_err(|_| MerchantError::ItemNotFound)
    }

    fn save(&mut self, character_id: u64, model: InventoryModel) {
        let gold = model.gold();
        save_inventory_record(self.db(), character_id, &model.into_record());
        self.gold.record(character_id, gold);
    }

    /// The vendor list `customer` sees, with the guild money they may spend here on
    /// repairs (Retail `CanGuildBankRepair` / `GetGuildBankWithdrawMoney`).
    fn vendor_inventory(
        &mut self,
        vendor: Entity,
        buyer: Buyer,
        character_id: u64,
    ) -> VendorInventory {
        let mut inventory = self.vendor_items(vendor, buyer);
        if inventory.can_repair {
            let day = self.reset.day(unix_now());
            inventory.guild_repair_money = guild_repair_money(self.db(), character_id, day);
        }
        inventory
    }

    /// The vendor list the player sees (AzerothCore `SendListInventory`):
    /// filtered items are left out, sold-out items too, and every item keeps
    /// its catalog slot index.
    fn vendor_items(&mut self, vendor: Entity, buyer: Buyer) -> VendorInventory {
        let (entry, can_repair) = self.npcs.get(vendor).map_or((0, false), |(npc, flags)| {
            (npc.template_id, flags.contains(NpcFlags::REPAIR))
        });
        if economic::is_business(self, entry) {
            return economic::vendor_inventory(self, vendor, entry, buyer, can_repair);
        }
        let slots: Vec<VendorSlot> = self.catalog.items(entry).to_vec();
        let now = self.now();
        let items = slots
            .into_iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                let template = *self.templates.as_ref()?.get(
                    shared::item_data::ItemDefinitionSource::Retail,
                    slot.item_id,
                )?;
                if !buyer.offered(&template) {
                    return None;
                }
                let goods = self.catalog.goods.get(&slot.item_id)?.clone();
                let available =
                    self.stock
                        .available(vendor, index as u32, slot, goods.buy_count, now);
                if available == Some(0) {
                    return None;
                }
                let cost = self.catalog.mount_costs.get(&(entry, index as u32));
                Some(VendorItem {
                    slot: index as u32,
                    item_id: slot.item_id,
                    name: goods.name,
                    quality: goods.quality,
                    price: cost.map_or(goods.buy_price, |cost| cost.money),
                    extra_costs: cost.map_or_else(Vec::new, |cost| cost.extra.clone()),
                    stack_count: goods.buy_count,
                    max_stack: template.max_stack.max(1),
                    num_available: available,
                    usable: buyer.can_use(&template)
                        && self.mount_reputation_met(buyer, slot.item_id),
                    max_durability: self.durability_rules.max_durability(slot.item_id),
                })
            })
            .collect();
        VendorInventory {
            npc: vendor.to_bits(),
            can_repair,
            guild_repair_money: None,
            items,
        }
    }

    fn send_inventory(
        &mut self,
        connection: Entity,
        vendor: Entity,
        buyer: Buyer,
        character_id: u64,
    ) {
        let inventory = self.vendor_inventory(vendor, buyer, character_id);
        if let Ok(mut sender) = self.inventories.get_mut(connection) {
            sender.send::<MerchantChannel>(inventory);
        }
    }

    fn send_buyback(&mut self, connection: Entity, slots: &BuybackSlots) {
        let items = slots
            .entries()
            .map(|(slot, entry)| BuybackItem {
                slot,
                item_id: entry.item.item_id,
                definition_source: persistence::inventory_store::load_item_definition_source(
                    self.db(),
                    entry.item.item_guid,
                ),
                name: entry.item.name.clone(),
                quality: entry.item.quality,
                count: entry.item.stack_count,
                price: entry.price,
            })
            .collect();
        if let Ok(mut sender) = self.buybacks.get_mut(connection) {
            sender.send::<MerchantChannel>(BuybackList { items });
        }
    }

    fn fail(&mut self, connection: Entity, npc: u64, error: MerchantError) {
        if let Ok(mut sender) = self.failures.get_mut(connection) {
            sender.send::<MerchantChannel>(MerchantFailed { npc, error });
        }
    }
}

/// The requesting connection and its open vendor.
struct Customer<'a> {
    connection: Entity,
    character_id: u64,
    player: Entity,
    vendor: Option<Entity>,
    buyback: &'a mut BuybackSlots,
}

impl Customer<'_> {
    fn vendor_for(&self, npc: u64) -> Result<Entity, MerchantError> {
        self.vendor
            .filter(|vendor| vendor.to_bits() == npc)
            .ok_or(MerchantError::NotInteracting)
    }
}

type Requests<'w, 's, M> = Query<
    'w,
    's,
    (
        Entity,
        &'static InWorld,
        &'static PlayerLink,
        Option<&'static ActiveInteraction>,
        &'static mut BuybackSlots,
        &'static mut MessageReceiver<M>,
    ),
>;

fn open_vendor(interaction: Option<&ActiveInteraction>) -> Option<Entity> {
    interaction
        .filter(|interaction| interaction.role == Some(NpcRole::Vendor))
        .map(|interaction| interaction.npc)
}

/// Run `handle` for each request; a refusal answers `MerchantFailed` for its NPC.
fn process<M: lightyear::prelude::Message + std::fmt::Debug>(
    requests: &mut Requests<M>,
    shop: &mut Shop,
    npc_of: impl Fn(&M) -> u64,
    handle: impl Fn(&mut Shop, &mut Customer, M) -> MerchantResult,
) {
    if !shop.ready() {
        return;
    }
    for (connection, in_world, link, interaction, mut buyback, mut receiver) in requests.iter_mut()
    {
        let mut customer = Customer {
            connection,
            character_id: in_world.character_id,
            player: link.0,
            vendor: open_vendor(interaction),
            buyback: &mut *buyback,
        };
        for request in receiver.receive() {
            let npc = npc_of(&request);
            // One line per decoded vendor request: the server-side record of what a client sent.
            info!(
                "merchant request character={} {request:?}",
                customer.character_id
            );
            if let Err(error) = handle(shop, &mut customer, request) {
                shop.fail(connection, npc, error);
            }
        }
    }
}

/// Buyback slots live for one session in the world.
fn init_buyback_slots(mut commands: Commands, entered: Query<Entity, Added<InWorld>>) {
    for connection in &entered {
        commands.entity(connection).insert(BuybackSlots::default());
    }
}

/// Opening a vendor frame sends its list and the buyback slots, as AzerothCore
/// sends `SMSG_LIST_INVENTORY` when the vendor gossip option is picked.
fn send_vendor_lists(
    opened: Query<
        (
            Entity,
            &InWorld,
            &PlayerLink,
            &ActiveInteraction,
            &BuybackSlots,
        ),
        Changed<ActiveInteraction>,
    >,
    mut shop: Shop,
) {
    for (connection, in_world, link, interaction, buyback) in &opened {
        let Some(vendor) = open_vendor(Some(interaction)) else {
            continue;
        };
        let buyer = shop.buyer(link.0);
        shop.send_inventory(connection, vendor, buyer, in_world.character_id);
        shop.send_buyback(connection, buyback);
    }
}

fn process_buy_requests(mut requests: Requests<BuyItem>, mut shop: Shop) {
    process(&mut requests, &mut shop, |req| req.npc, buy);
}

fn process_sell_requests(mut requests: Requests<SellItem>, mut shop: Shop) {
    process(&mut requests, &mut shop, |req| req.npc, sell);
}

fn process_sell_all_junk_requests(mut requests: Requests<SellAllJunkItems>, mut shop: Shop) {
    process(&mut requests, &mut shop, |req| req.npc, sell_all_junk);
}

fn process_buyback_requests(mut requests: Requests<BuybackItemRequest>, mut shop: Shop) {
    process(&mut requests, &mut shop, |req| req.npc, buy_back);
}

/// AzerothCore `Player::BuyItemFromVendorSlot`: `count` purchases of
/// `VendorStackCount` items each, `BuyPrice` copper per purchase.
fn buy(shop: &mut Shop, customer: &mut Customer, req: BuyItem) -> MerchantResult {
    let vendor = customer.vendor_for(req.npc)?;
    let entry = shop.vendor_entry(vendor)?;
    if economic::is_business(shop, entry) {
        return economic::buy(shop, customer, vendor, entry, req);
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
        .map(|cost| mount_costs::debit_turnins(shop, customer.player, &mut model, cost, count))
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
        mount_costs::persist_purchase(shop, customer, model, wallet);
    } else {
        shop.save(customer.character_id, model);
    }
    crate::guild_news::record_purchase(shop.db(), customer.character_id, slot.item_id, &template);
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
fn sell(shop: &mut Shop, customer: &mut Customer, req: SellItem) -> MerchantResult {
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
    shop: &mut Shop,
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
fn buy_back(shop: &mut Shop, customer: &mut Customer, req: BuybackItemRequest) -> MerchantResult {
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

fn process_repair_requests(mut requests: Requests<RepairItem>, mut shop: Shop) {
    process(&mut requests, &mut shop, |req| req.npc, repair);
}

/// AzerothCore `DurabilityRepairAll` / `DurabilityRepair`: each damaged item costs
/// `ItemDurability::repair_cost`; an item that costs more than the money left stays
/// damaged (AzerothCore skips it silently; we answer "You don't have enough money.").
/// Per-item repair also accepts damaged owned bag items.
fn repair(shop: &mut Shop, customer: &mut Customer, req: RepairItem) -> MerchantResult {
    let vendor = customer.vendor_for(req.npc)?;
    let repairs = shop
        .npcs
        .get(vendor)
        .is_ok_and(|(_, flags)| flags.contains(NpcFlags::REPAIR));
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
    let Ok(mut durability) = shop.durability.get_mut(customer.player) else {
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
    drop(durability);
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
    shop: &mut Shop,
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
fn guild_repair(shop: &mut Shop, customer: &mut Customer, vendor: Entity) -> MerchantResult {
    let day = shop.reset.day(unix_now());
    let available = guild_repair_money(shop.db(), customer.character_id, day)
        .ok_or(MerchantError::GuildPermissions)?;
    let Ok(durability) = shop.durability.get(customer.player) else {
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
        let mut durability = shop
            .durability
            .get_mut(customer.player)
            .expect("read above");
        for index in &repaired {
            let entry = &mut durability.slots[*index];
            entry.current = entry.max;
        }
        let guild_id = pay_guild_repair(shop.db(), customer.character_id, day, total);
        shop.changed_guilds.0.push(guild_id);
        let buyer = shop.buyer(customer.player);
        shop.send_inventory(customer.connection, vendor, buyer, customer.character_id);
    }
    if repaired.len() < costs.len() {
        return Err(MerchantError::GuildNotEnoughMoney);
    }
    Ok(())
}

#[cfg(test)]
#[path = "merchant_tests.rs"]
mod tests;
