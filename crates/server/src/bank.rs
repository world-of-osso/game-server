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
            .init_resource::<PendingBankCommits>()
            .add_systems(
                Update,
                (
                    send_bank_contents,
                    publish_bank_commits,
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

mod actor;
mod transactions;
pub(crate) use actor::PendingBankCommits;
use actor::publish_bank_commits;
pub(crate) use transactions::criteria_bank_type;

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
}

/// The requesting connection and its open banker.
#[derive(Clone)]
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

    fn load_bank(&self, patron: &Patron, bank: BankType) -> BankData {
        match bank {
            BankType::Character => load_character_bank(self.db(), patron.character_id),
            BankType::Account => load_account_bank(self.db(), patron.account_id),
        }
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
                    slots: bank_storage::tab_slots(tab, soulbound, |guid| {
                        persistence::inventory_store::load_item_definition_source(self.db(), guid)
                    }),
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

/// Validate the banker now; execute owned requests against fresh FIFO rows.
fn process<M: lightyear::prelude::Message>(
    requests: &mut Requests<M>,
    vault: &mut Vault,
    pending: &mut PendingBankCommits,
    npc_of: impl Fn(&M) -> u64,
    handle: impl Fn(&mut transactions::Transaction, &Patron, M) -> BankResult + Copy + Send + 'static,
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
            if let Err(error) = patron.banker_for(npc) {
                vault.fail(connection, npc, error);
                continue;
            }
            let templates = vault.templates.as_deref().expect("ready").clone();
            let prices = vault.prices.clone();
            let owner = patron.clone();
            let receipt =
                vault
                    .db
                    .as_ref()
                    .expect("ready")
                    .writer
                    .enqueue("bank transaction", move |db| {
                        Ok(transactions::execute(
                            db, templates, prices, owner, request, handle,
                        ))
                    });
            pending.push(patron.clone(), npc, receipt);
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

fn process_deposits(
    mut requests: Requests<BankDeposit>,
    mut pending: ResMut<PendingBankCommits>,
    mut vault: Vault,
) {
    process(
        &mut requests,
        &mut vault,
        &mut pending,
        |req| req.npc,
        transactions::deposit,
    );
}

fn process_withdrawals(
    mut requests: Requests<BankWithdraw>,
    mut pending: ResMut<PendingBankCommits>,
    mut vault: Vault,
) {
    process(
        &mut requests,
        &mut vault,
        &mut pending,
        |req| req.npc,
        transactions::withdraw,
    );
}

fn process_tab_purchases(
    mut requests: Requests<BankPurchaseTab>,
    mut pending: ResMut<PendingBankCommits>,
    mut vault: Vault,
) {
    process(
        &mut requests,
        &mut vault,
        &mut pending,
        |req| req.npc,
        transactions::purchase_tab,
    );
}

fn process_money_transfers(
    mut requests: Requests<BankMoneyTransfer>,
    mut pending: ResMut<PendingBankCommits>,
    mut vault: Vault,
) {
    process(
        &mut requests,
        &mut vault,
        &mut pending,
        |req| req.npc,
        transactions::transfer_money,
    );
}

fn process_auto_deposits(
    mut requests: Requests<BankAutoDeposit>,
    mut pending: ResMut<PendingBankCommits>,
    mut vault: Vault,
) {
    process(
        &mut requests,
        &mut vault,
        &mut pending,
        |req| req.npc,
        transactions::auto_deposit,
    );
}

fn process_tab_settings(
    mut requests: Requests<BankUpdateTabSettings>,
    mut pending: ResMut<PendingBankCommits>,
    mut vault: Vault,
) {
    process(
        &mut requests,
        &mut vault,
        &mut pending,
        |req| req.npc,
        transactions::update_tab_settings,
    );
}

#[cfg(test)]
#[path = "bank_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "bank_move_tests.rs"]
mod move_tests;
