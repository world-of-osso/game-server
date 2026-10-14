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

mod actor;
mod catalog;
mod economic;
mod mount_costs;
mod stock;
mod transactions;
use actor::{PendingMerchant, drive_merchant_commits};

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
            .init_resource::<PendingMerchant>()
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
                    drive_merchant_commits,
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

/// Queue decoded requests with the interaction validated at receipt time.
fn process<M: lightyear::prelude::Message + std::fmt::Debug>(
    requests: &mut Requests<M>,
    shop: &mut Shop,
    pending: &mut PendingMerchant,
    npc_of: impl Fn(&M) -> u64,
    operation: impl Fn(M) -> transactions::Operation,
) {
    if !shop.ready() {
        return;
    }
    for (connection, in_world, link, interaction, _, mut receiver) in requests.iter_mut() {
        for request in receiver.receive() {
            let npc = npc_of(&request);
            let Some(vendor) = open_vendor(interaction).filter(|vendor| vendor.to_bits() == npc)
            else {
                shop.fail(connection, npc, MerchantError::NotInteracting);
                continue;
            };
            info!(
                "merchant request character={} {request:?}",
                in_world.character_id
            );
            pending.requests.push_back(actor::Request {
                connection,
                character: in_world.character_id,
                player: link.0,
                vendor,
                operation: operation(request),
            });
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

fn process_buy_requests(
    mut requests: Requests<BuyItem>,
    mut pending: ResMut<PendingMerchant>,
    mut shop: Shop,
) {
    process(
        &mut requests,
        &mut shop,
        &mut pending,
        |req| req.npc,
        transactions::Operation::Buy,
    );
}

fn process_sell_requests(
    mut requests: Requests<SellItem>,
    mut pending: ResMut<PendingMerchant>,
    mut shop: Shop,
) {
    process(
        &mut requests,
        &mut shop,
        &mut pending,
        |req| req.npc,
        transactions::Operation::Sell,
    );
}

fn process_sell_all_junk_requests(
    mut requests: Requests<SellAllJunkItems>,
    mut pending: ResMut<PendingMerchant>,
    mut shop: Shop,
) {
    process(
        &mut requests,
        &mut shop,
        &mut pending,
        |req| req.npc,
        transactions::Operation::Junk,
    );
}

fn process_buyback_requests(
    mut requests: Requests<BuybackItemRequest>,
    mut pending: ResMut<PendingMerchant>,
    mut shop: Shop,
) {
    process(
        &mut requests,
        &mut shop,
        &mut pending,
        |req| req.npc,
        transactions::Operation::Buyback,
    );
}

fn process_repair_requests(
    mut requests: Requests<RepairItem>,
    mut pending: ResMut<PendingMerchant>,
    mut shop: Shop,
) {
    process(
        &mut requests,
        &mut shop,
        &mut pending,
        |req| req.npc,
        transactions::Operation::Repair,
    );
}

#[cfg(test)]
#[path = "merchant_tests.rs"]
mod tests;
