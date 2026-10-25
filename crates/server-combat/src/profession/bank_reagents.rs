//! Recipe reagents held outside the bags. Retail `ItemUtil.GetCraftingReagentCount`
//! counts the character bank and the Warband bank. No source defines the order a
//! craft takes them in; project policy
//! CRAFTING_REAGENT_CONSUMPTION_ORDER_RETAIL_ACCESSIBLE_BAGS follows the Retail
//! `CraftingAccessibleBags` enumeration (Blizzard_Professions.lua): Backpack,
//! Bag_1..4, ReagentBag, CharacterBankTab_1..6, AccountBankTab_1..5. The
//! Mainline UI never sets `characterInventoryOnly`, so the Warband bank always
//! counts.

use std::collections::HashMap;

use redb::Database;
use shared::item_data::ItemDefinitionSource::{self, Retail};
use shared::protocol::BankType;

use crate::bank_storage::{BankTabData, load_tab_definition_sources};
use crate::inventory::inventory_model::InventoryModel;
use crate::persistence::banks::{
    BankData, BankOwner, load_account_bank, load_character_bank, save_bank_moves,
};

pub(super) struct ReagentBanks {
    character_id: u64,
    character: BankData,
    account: Option<(u64, BankData)>,
    sources: HashMap<u64, ItemDefinitionSource>,
    character_changed: bool,
    account_changed: bool,
}

impl ReagentBanks {
    /// The character's bank, and the Warband bank when the account is known.
    pub(super) fn load(db: &Database, character_id: u64, account_id: Option<u64>) -> Self {
        let character = load_character_bank(db, character_id);
        let account = account_id.map(|id| (id, load_account_bank(db, id)));
        let mut sources = load_tab_definition_sources(db, &character.tabs);
        if let Some((_, bank)) = &account {
            sources.extend(load_tab_definition_sources(db, &bank.tabs));
        }
        Self {
            character_id,
            character,
            account,
            sources,
            character_changed: false,
            account_changed: false,
        }
    }

    pub(super) fn count(&self, item_id: u32) -> u32 {
        self.banks()
            .flat_map(|tabs| tabs.iter().flat_map(|tab| tab.slots.iter().flatten()))
            .filter(|item| item.item_id == item_id && self.sources[&item.item_guid] == Retail)
            .map(|item| item.stack_count)
            .sum()
    }

    /// Take `count` of `item_id`: bags first, then the character bank, then the
    /// Warband bank. Nothing changes when they hold fewer together.
    pub(super) fn take(
        &mut self,
        model: &mut InventoryModel,
        item_id: u32,
        count: u32,
    ) -> Result<(), String> {
        let in_bags = model.bag_count(Retail, item_id);
        if in_bags + self.count(item_id) < count {
            return Err(format!("fewer than {count} of item {item_id} to take"));
        }
        let from_bags = in_bags.min(count);
        model.destroy_bag_items(Retail, item_id, from_bags)?;
        let wanted = count - from_bags;
        let left = take_from_tabs(&mut self.character.tabs, &self.sources, item_id, wanted);
        self.character_changed |= left < wanted;
        if let Some((_, bank)) = &mut self.account {
            self.account_changed |=
                take_from_tabs(&mut bank.tabs, &self.sources, item_id, left) < left;
        }
        Ok(())
    }

    /// Save the bags and every bank a `take` changed in one transaction; returns
    /// the changed banks.
    pub(super) fn save(&self, db: &Database, model: InventoryModel) -> Vec<BankType> {
        let character = self
            .character_changed
            .then_some(BankOwner::Character(self.character_id, &self.character));
        let account = self
            .account
            .as_ref()
            .filter(|_| self.account_changed)
            .map(|(id, bank)| BankOwner::Account(*id, bank));
        save_bank_moves(
            db,
            self.character_id,
            character.into_iter().chain(account),
            Some(&model.into_record()),
        );
        [
            (self.character_changed, BankType::Character),
            (self.account_changed, BankType::Account),
        ]
        .into_iter()
        .filter_map(|(changed, bank)| changed.then_some(bank))
        .collect()
    }

    fn banks(&self) -> impl Iterator<Item = &[BankTabData]> {
        std::iter::once(self.character.tabs.as_slice())
            .chain(self.account.iter().map(|(_, bank)| bank.tabs.as_slice()))
    }
}

/// Remove up to `count` Retail `item_id` in tab and slot order; returns how many
/// are still missing.
fn take_from_tabs(
    tabs: &mut [BankTabData],
    sources: &HashMap<u64, ItemDefinitionSource>,
    item_id: u32,
    mut count: u32,
) -> u32 {
    for cell in tabs.iter_mut().flat_map(|tab| tab.slots.iter_mut()) {
        if count == 0 {
            break;
        }
        let Some(item) = cell
            .as_mut()
            .filter(|item| item.item_id == item_id && sources[&item.item_guid] == Retail)
        else {
            continue;
        };
        let taken = count.min(item.stack_count);
        item.stack_count -= taken;
        count -= taken;
        if item.stack_count == 0 {
            *cell = None;
        }
    }
    count
}
