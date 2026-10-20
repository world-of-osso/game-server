//! Character bank and Warband bank (docs/wiki/systems/banks.md) for the banker whose
//! bank frame `npc_interaction` has open.
//!
//! Retail combined bank (`BankFrame`, `C_Bank`): both banks are purchased tabs of
//! `BANK_TAB_SLOTS` slots bought at the Retail `BankTab` prices; the Warband bank
//! is stored per account and alone holds money. Deposits and withdrawals follow
//! AzerothCore `Player::CanBankItem` / `BankItem` / `StoreItem`
//! (`bank_storage::deposit`, `InventoryModel::store_item`); soulbound items can't
//! enter the Warband bank (`ERR_NO_SOULBOUND_ITEM_IN_ACCOUNT_BANK`). Each move
//! writes the bank and the bags in one transaction. The account is signed in on one
//! connection at a time (`ConnectedAccounts`), so the Warband bank needs no
//! Retail `NoAccountInventoryLock`.
//!
//! Gaps: no sorting, no search, no expansion filter, no partial stack moves.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use redb::Database;
use rusqlite::Connection;
use shared::protocol::{
    BankAutoDeposit, BankChannel, BankContents, BankDeposit, BankError, BankFailed,
    BankMoneyTransfer, BankPurchaseTab, BankTabView, BankType, BankUpdateTabSettings, BankWithdraw,
    NpcRole, bank_tab_flags,
};

use crate::auth::{Authenticated, InWorld};
use crate::bank_storage::{self, BankTabData, TabsFull};
use crate::gold::{GoldUpdateQueue, MAX_MONEY};
use crate::inventory::{inventory_model::InventoryModel, load_model};
use crate::item_templates::{ItemTemplate, ItemTemplates};
use crate::npc_interaction::ActiveInteraction;
use crate::persistence::banks::{
    BankData, BankOwner, load_account_bank, load_character_bank, save_bank_move,
};
use crate::persistence::{self, GameDatabase, InventoryItemData};

/// Tab name length (Retail `GuildBankPopupEditBox` letters="15").
pub(crate) const TAB_NAME_LETTERS: usize = 15;
/// Retail `INV_Misc_QuestionMark`, the icon of a new tab.
pub(crate) const DEFAULT_TAB_ICON: u32 = 134400;
/// Retail `ITEM_CLASS_TRADEGOODS` ("Tradeskill"): crafting reagents.
const ITEM_CLASS_TRADEGOODS: u8 = 7;
const ITEM_CLASS_CONSUMABLE: u8 = 0;
const ITEM_CLASS_WEAPON: u8 = 2;
const ITEM_CLASS_ARMOR: u8 = 4;
/// Retail `ItemBondingType` ToWoWAccount / ToBnetAccount / ToBnetAccountUntilEquipped.
const WARBOUND_BONDING: [u8; 3] = [7, 8, 9];

/// Retail `BankTab` prices (world.db `bank_tab`), copper, in purchase order.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct BankTabPrices {
    pub character: Vec<u64>,
    pub guild: Vec<u64>,
    pub account: Vec<u64>,
}

impl BankTabPrices {
    fn of(&self, bank: BankType) -> &[u64] {
        match bank {
            BankType::Character => &self.character,
            BankType::Account => &self.account,
        }
    }
}

pub fn load_bank_tab_prices(conn: &Connection) -> rusqlite::Result<BankTabPrices> {
    let mut statement =
        conn.prepare("SELECT BankType, Cost FROM bank_tab ORDER BY BankType, OrderIndex")?;
    let mut prices = BankTabPrices::default();
    for row in statement.query_map([], |row| Ok((row.get::<_, u8>(0)?, row.get::<_, i64>(1)?)))? {
        let (bank, cost) = row?;
        let list = match bank {
            0 => &mut prices.character,
            1 => &mut prices.guild,
            2 => &mut prices.account,
            _ => continue,
        };
        list.push(cost as u64);
    }
    Ok(prices)
}

/// Retail `GUILDBANK_TAB_NUMBER` "Tab %d".
pub(crate) fn new_tab(number: usize) -> BankTabData {
    let mut tab = BankTabData::new(format!("Tab {number}"));
    tab.icon = DEFAULT_TAB_ICON;
    tab
}

/// Bank moves on InventoryChannel schedule a fresh character-bank snapshot.
#[derive(Resource, Default)]
pub(crate) struct ChangedCharacterBanks(pub std::collections::HashSet<Entity>);

pub struct BankPlugin;

impl Plugin for BankPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BankTabPrices>()
            .init_resource::<ChangedCharacterBanks>()
            .add_systems(
                Update,
                (
                    send_bank_contents,
                    process_deposits,
                    process_withdrawals,
                    process_tab_purchases,
                    process_money_transfers,
                    process_auto_deposits,
                    process_tab_settings,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                send_changed_character_banks.before(MessageSystems::Send),
            );
    }
}

type BankResult = Result<(), BankError>;

/// Everything a bank request reads and writes besides the connection.
#[derive(SystemParam)]
struct Vault<'w, 's> {
    db: Option<Res<'w, GameDatabase>>,
    templates: Option<Res<'w, ItemTemplates>>,
    prices: Res<'w, BankTabPrices>,
    gold: ResMut<'w, GoldUpdateQueue>,
    contents: Query<'w, 's, &'static mut MessageSender<BankContents>>,
    failures: Query<'w, 's, &'static mut MessageSender<BankFailed>>,
    commands: Commands<'w, 's>,
}

/// The requesting connection and its open banker.
struct Patron {
    connection: Entity,
    character_id: u64,
    account_id: u64,
    banker: Option<Entity>,
}

impl Patron {
    fn banker_for(&self, npc: u64) -> Result<Entity, BankError> {
        self.banker
            .filter(|banker| banker.to_bits() == npc)
            .ok_or(BankError::NotInteracting)
    }
}

impl Vault<'_, '_> {
    fn ready(&self) -> bool {
        self.db.is_some() && self.templates.is_some()
    }

    fn db(&self) -> &Database {
        &self.db.as_ref().expect("checked by ready").db
    }

    fn load_template(
        &self,
        item: &crate::persistence::InventoryItemData,
    ) -> Result<(shared::item_data::ItemDefinitionSource, ItemTemplate), BankError> {
        let source =
            persistence::inventory_store::load_item_definition_source(self.db(), item.item_guid);
        let template = self
            .templates
            .as_ref()
            .and_then(|templates| templates.get(source, item.item_id).copied())
            .ok_or(BankError::ItemNotFound)?;
        Ok((source, template))
    }

    fn mint(&self) -> impl FnMut() -> u64 + '_ {
        let db = self.db();
        move || persistence::next_id(db, "item_instance")
    }

    fn load_model(&self, character_id: u64) -> Result<InventoryModel, BankError> {
        let templates = self.templates.as_ref().expect("checked by ready");
        load_model(self.db(), templates, character_id).map_err(|_| BankError::ItemNotFound)
    }

    fn load_bank(&self, patron: &Patron, bank: BankType) -> BankData {
        match bank {
            BankType::Character => load_character_bank(self.db(), patron.character_id),
            BankType::Account => load_account_bank(self.db(), patron.account_id),
        }
    }

    /// Save the bank, plus the bags and money when `model` is given, then send the
    /// new bank contents.
    fn save(
        &mut self,
        patron: &Patron,
        bank: BankType,
        data: &BankData,
        model: Option<InventoryModel>,
    ) {
        let owner = match bank {
            BankType::Character => BankOwner::Character(patron.character_id, data),
            BankType::Account => BankOwner::Account(patron.account_id, data),
        };
        let gold = model.as_ref().map(InventoryModel::gold);
        let record = model.map(InventoryModel::into_record);
        save_bank_move(self.db(), patron.character_id, owner, record.as_ref());
        if let Some(gold) = gold {
            self.gold.record(patron.character_id, gold);
        }
        self.send_contents(patron.connection, bank, data);
    }

    fn contents(&self, bank: BankType, data: &BankData) -> BankContents {
        let guids = data.tabs.iter().flat_map(|tab| tab.slots.iter().flatten());
        let bound =
            persistence::inventory_store::soulbound_guids(self.db(), guids.map(|i| i.item_guid));
        let soulbound = |guid| bound.contains(&guid);
        BankContents {
            bank,
            tabs: data
                .tabs
                .iter()
                .map(|tab| BankTabView {
                    name: tab.name.clone(),
                    icon: tab.icon,
                    deposit_flags: tab.deposit_flags,
                    slots: bank_storage::tab_slots(
                        tab,
                        soulbound,
                        |guid| {
                            persistence::inventory_store::load_item_definition_source(
                                self.db(),
                                guid,
                            )
                        },
                        |guid| {
                            persistence::inventory_store::load_item_crafting_origin(self.db(), guid)
                        },
                    ),
                })
                .collect(),
            next_tab_cost: self.prices.of(bank).get(data.tabs.len()).copied(),
            money: (bank == BankType::Account).then_some(data.money),
        }
    }

    fn send_contents(&mut self, connection: Entity, bank: BankType, data: &BankData) {
        let contents = self.contents(bank, data);
        if let Ok(mut sender) = self.contents.get_mut(connection) {
            sender.send::<BankChannel>(contents);
        }
    }

    fn fail(&mut self, connection: Entity, npc: u64, error: BankError) {
        if let Ok(mut sender) = self.failures.get_mut(connection) {
            sender.send::<BankChannel>(BankFailed { npc, error });
        }
    }
}

type Requests<'w, 's, M> = Query<
    'w,
    's,
    (
        Entity,
        &'static InWorld,
        &'static Authenticated,
        Option<&'static ActiveInteraction>,
        &'static mut MessageReceiver<M>,
    ),
>;

fn open_banker(interaction: Option<&ActiveInteraction>) -> Option<Entity> {
    interaction
        .filter(|interaction| interaction.role == Some(NpcRole::Banker))
        .map(|interaction| interaction.npc)
}

/// Run `handle` for each request; a refusal answers `BankFailed` for its NPC.
fn process<M: lightyear::prelude::Message>(
    requests: &mut Requests<M>,
    vault: &mut Vault,
    npc_of: impl Fn(&M) -> u64,
    handle: impl Fn(&mut Vault, &Patron, M) -> BankResult,
) {
    if !vault.ready() {
        return;
    }
    for (connection, in_world, auth, interaction, mut receiver) in requests.iter_mut() {
        let patron = Patron {
            connection,
            character_id: in_world.character_id,
            account_id: auth.account_id,
            banker: open_banker(interaction),
        };
        for request in receiver.receive() {
            let npc = npc_of(&request);
            if let Err(error) = handle(vault, &patron, request) {
                vault.fail(connection, npc, error);
            }
        }
    }
}

/// Opening the banker role sends both banks (Retail `BANKFRAME_OPENED`).
fn send_bank_contents(
    opened: Query<
        (Entity, &InWorld, &Authenticated, &ActiveInteraction),
        Changed<ActiveInteraction>,
    >,
    mut vault: Vault,
) {
    if !vault.ready() {
        return;
    }
    for (connection, in_world, auth, interaction) in &opened {
        let Some(banker) = open_banker(Some(interaction)) else {
            continue;
        };
        let patron = Patron {
            connection,
            character_id: in_world.character_id,
            account_id: auth.account_id,
            banker: Some(banker),
        };
        for bank in [BankType::Character, BankType::Account] {
            let data = vault.load_bank(&patron, bank);
            vault.send_contents(connection, bank, &data);
        }
    }
}

fn send_changed_character_banks(
    mut changed: ResMut<ChangedCharacterBanks>,
    patrons: Query<(&InWorld, &Authenticated)>,
    mut vault: Vault,
) {
    if !vault.ready() {
        return;
    }
    for connection in changed.0.drain() {
        let Ok((in_world, auth)) = patrons.get(connection) else {
            continue;
        };
        let patron = Patron {
            connection,
            character_id: in_world.character_id,
            account_id: auth.account_id,
            banker: None,
        };
        let data = vault.load_bank(&patron, BankType::Character);
        vault.send_contents(connection, BankType::Character, &data);
    }
}

fn process_deposits(mut requests: Requests<BankDeposit>, mut vault: Vault) {
    process(&mut requests, &mut vault, |req| req.npc, deposit);
}

fn process_withdrawals(mut requests: Requests<BankWithdraw>, mut vault: Vault) {
    process(&mut requests, &mut vault, |req| req.npc, withdraw);
}

fn process_tab_purchases(mut requests: Requests<BankPurchaseTab>, mut vault: Vault) {
    process(&mut requests, &mut vault, |req| req.npc, purchase_tab);
}

fn process_money_transfers(mut requests: Requests<BankMoneyTransfer>, mut vault: Vault) {
    process(&mut requests, &mut vault, |req| req.npc, transfer_money);
}

fn process_auto_deposits(mut requests: Requests<BankAutoDeposit>, mut vault: Vault) {
    process(&mut requests, &mut vault, |req| req.npc, auto_deposit);
}

fn process_tab_settings(mut requests: Requests<BankUpdateTabSettings>, mut vault: Vault) {
    process(
        &mut requests,
        &mut vault,
        |req| req.npc,
        update_tab_settings,
    );
}

/// Move one bag stack into `data`, visiting tabs in `order`; nothing changes when
/// it doesn't fit.
fn move_into_bank(
    vault: &Vault,
    model: &mut InventoryModel,
    data: &mut BankData,
    bank: BankType,
    order: &[usize],
    item_guid: u64,
) -> BankResult {
    let item = model
        .bag_item(item_guid)
        .cloned()
        .ok_or(BankError::ItemNotFound)?;
    if bank == BankType::Account && model.is_soulbound(item_guid) {
        return Err(BankError::SoulboundInAccountBank);
    }
    let source = model.definition_source(item_guid);
    let template = vault
        .templates
        .as_ref()
        .and_then(|templates| templates.get(source, item.item_id))
        .ok_or(BankError::ItemNotFound)?;
    let mut sources = bank_storage::load_tab_definition_sources(vault.db(), &data.tabs);
    sources.insert(item_guid, source);
    let mut mint = vault.mint();
    bank_storage::deposit(
        &mut data.tabs,
        order,
        item,
        template.max_stack,
        &mut sources,
        &mut mint,
    )
    .map_err(|TabsFull| BankError::BankFull)?;
    model.remember_definition_sources(sources);
    model
        .take_bag_item(item_guid, 0, &mut mint)
        .map_err(|_| BankError::ItemNotFound)?;
    Ok(())
}

fn deposit(vault: &mut Vault, patron: &Patron, req: BankDeposit) -> BankResult {
    patron.banker_for(req.npc)?;
    let mut data = vault.load_bank(patron, req.bank);
    if usize::from(req.tab) >= data.tabs.len() {
        return Err(BankError::TabLocked);
    }
    let mut model = vault.load_model(patron.character_id)?;
    let order = bank_storage::tab_order(data.tabs.len(), usize::from(req.tab));
    move_into_bank(
        vault,
        &mut model,
        &mut data,
        req.bank,
        &order,
        req.item_guid,
    )?;
    vault.save(patron, req.bank, &data, Some(model));
    Ok(())
}

fn withdraw(vault: &mut Vault, patron: &Patron, req: BankWithdraw) -> BankResult {
    patron.banker_for(req.npc)?;
    let mut data = vault.load_bank(patron, req.bank);
    let item =
        bank_storage::take(&mut data.tabs, req.tab, req.slot).ok_or(BankError::ItemNotFound)?;
    let mut model = vault.load_model(patron.character_id)?;
    let (source, template) = vault.load_template(&item)?;
    let mut mint = vault.mint();
    model
        .store_item(item, source, template, &mut mint)
        .map_err(|_| BankError::InventoryFull)?;
    drop(mint);
    vault.save(patron, req.bank, &data, Some(model));
    Ok(())
}

/// `C_Bank.PurchaseBankTab`: the player pays the next `BankTab` price.
fn purchase_tab(vault: &mut Vault, patron: &Patron, req: BankPurchaseTab) -> BankResult {
    patron.banker_for(req.npc)?;
    let mut data = vault.load_bank(patron, req.bank);
    let cost = vault
        .prices
        .of(req.bank)
        .get(data.tabs.len())
        .copied()
        .ok_or(BankError::MaxTabs)?;
    let mut model = vault.load_model(patron.character_id)?;
    let gold = model.gold();
    if gold < cost {
        return Err(BankError::CantAfford);
    }
    model.set_gold(gold - cost);
    data.tabs.push(new_tab(data.tabs.len() + 1));
    vault.save(patron, req.bank, &data, Some(model));
    let (character, event) = (
        patron.character_id,
        crate::achievement::CriteriaEvent::BankTabsOwned(
            criteria_bank_type(req.bank),
            data.tabs.len() as u32,
        ),
    );
    vault.commands.queue(move |world: &mut World| {
        crate::achievement::record_character_event(world, character, event);
    });
    Ok(())
}

/// TC `BankType` numbering of achievement criteria (Character 0, Account 2).
pub(crate) fn criteria_bank_type(bank: BankType) -> u32 {
    match bank {
        BankType::Character => 0,
        BankType::Account => 2,
    }
}

/// Retail `C_Bank.DepositMoney` / `WithdrawMoney`; only the Warband bank holds money.
fn transfer_money(vault: &mut Vault, patron: &Patron, req: BankMoneyTransfer) -> BankResult {
    patron.banker_for(req.npc)?;
    if req.bank != BankType::Account {
        return Err(BankError::NoMoneyTransfer);
    }
    let mut data = vault.load_bank(patron, req.bank);
    let mut model = vault.load_model(patron.character_id)?;
    let gold = model.gold();
    let (gold, money) = if req.deposit {
        if gold < req.copper {
            return Err(BankError::NotEnoughMoney);
        }
        if data.money + req.copper > MAX_MONEY {
            return Err(BankError::TooMuchGold);
        }
        (gold - req.copper, data.money + req.copper)
    } else {
        if data.money < req.copper {
            return Err(BankError::NotEnoughMoney);
        }
        if gold + req.copper > MAX_MONEY {
            return Err(BankError::TooMuchGold);
        }
        (gold + req.copper, data.money - req.copper)
    };
    model.set_gold(gold);
    data.money = money;
    vault.save(patron, req.bank, &data, Some(model));
    Ok(())
}

/// Retail `Enum.BagSlotFlags` assignments an item matches.
fn item_flags(template: &ItemTemplate, quality: u8) -> u32 {
    let class = match template.scaling.class_id {
        ITEM_CLASS_CONSUMABLE => bank_tab_flags::CONSUMABLES,
        ITEM_CLASS_WEAPON | ITEM_CLASS_ARMOR => bank_tab_flags::EQUIPMENT,
        ITEM_CLASS_TRADEGOODS => bank_tab_flags::REAGENTS | bank_tab_flags::PROFESSION_GOODS,
        _ => 0,
    };
    let junk = if quality == 0 {
        bank_tab_flags::JUNK
    } else {
        0
    };
    class | junk
}

/// Tabs assigned to any of `flags` first, then the others (Retail tab deposit settings).
fn assigned_order(tabs: &[BankTabData], flags: u32) -> Vec<usize> {
    let (assigned, rest): (Vec<usize>, Vec<usize>) =
        (0..tabs.len()).partition(|&tab| tabs[tab].deposit_flags & flags != 0);
    assigned.into_iter().chain(rest).collect()
}

/// Bag stacks `C_Bank.AutoDepositItemsIntoBank` moves: reagents for the character
/// bank ("Deposit All Reagents"); Warbound items, plus tradeable reagents when asked,
/// for the Warband bank ("Deposit All Warbound Items").
fn auto_deposit_items(
    vault: &Vault,
    model: &InventoryModel,
    bank: BankType,
    include_reagents: bool,
) -> Vec<(u64, u32)> {
    model
        .bag_items()
        .filter_map(|item: &InventoryItemData| {
            let (_, template) = vault
                .load_template(item)
                .expect("loaded owned item has a definition");
            let reagent = template.scaling.class_id == ITEM_CLASS_TRADEGOODS;
            let selected = match bank {
                BankType::Character => reagent,
                BankType::Account => {
                    WARBOUND_BONDING.contains(&template.bonding)
                        || (include_reagents && reagent && !model.is_soulbound(item.item_guid))
                }
            };
            selected.then(|| (item.item_guid, item_flags(&template, item.quality)))
        })
        .collect()
}

fn auto_deposit(vault: &mut Vault, patron: &Patron, req: BankAutoDeposit) -> BankResult {
    patron.banker_for(req.npc)?;
    let mut data = vault.load_bank(patron, req.bank);
    let mut model = vault.load_model(patron.character_id)?;
    let items = auto_deposit_items(vault, &model, req.bank, req.include_reagents);
    let mut full = false;
    for (item_guid, flags) in items {
        let order = assigned_order(&data.tabs, flags);
        match move_into_bank(vault, &mut model, &mut data, req.bank, &order, item_guid) {
            Ok(()) => {}
            // The stack stays in the bags; later, smaller stacks may still fit.
            Err(BankError::BankFull) => full = true,
            Err(error) => return Err(error),
        }
    }
    vault.save(patron, req.bank, &data, Some(model));
    if full {
        return Err(BankError::BankFull);
    }
    Ok(())
}

fn update_tab_settings(
    vault: &mut Vault,
    patron: &Patron,
    req: BankUpdateTabSettings,
) -> BankResult {
    patron.banker_for(req.npc)?;
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > TAB_NAME_LETTERS {
        return Err(BankError::InvalidTabName);
    }
    let mut data = vault.load_bank(patron, req.bank);
    let tab = data
        .tabs
        .get_mut(usize::from(req.tab))
        .ok_or(BankError::TabLocked)?;
    tab.name = name.to_owned();
    tab.icon = req.icon;
    tab.deposit_flags = req.deposit_flags;
    vault.save(patron, req.bank, &data, None);
    Ok(())
}

#[cfg(test)]
#[path = "bank_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "bank_move_tests.rs"]
mod move_tests;
