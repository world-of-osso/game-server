//! Guild bank: shared storage with tabs, permissions, and transaction logging.
//!
//! Ref: AzerothCore `GuildBank.cpp`.

use serde::{Deserialize, Serialize};

use crate::item_data::InvSlot;

/// Maximum guild bank tabs.
pub const MAX_TABS: usize = 8;
/// Slots per tab.
pub const SLOTS_PER_TAB: usize = 98;

/// Cost to purchase each tab (in copper). Tabs get progressively more expensive.
const TAB_COSTS: [u32; MAX_TABS] = [
    1_000_000,  // 100g
    2_500_000,  // 250g
    5_000_000,  // 500g
    10_000_000, // 1000g
    25_000_000, // 2500g
    50_000_000, // 5000g
    50_000_000, // 5000g
    50_000_000, // 5000g
];

/// A single guild bank tab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuildBankTab {
    /// Tab display name.
    pub name: String,
    /// Tab icon identifier.
    pub icon: String,
    /// 98 item slots.
    pub slots: Vec<InvSlot>,
}

impl GuildBankTab {
    pub fn new(name: String) -> Self {
        Self {
            name,
            icon: String::new(),
            slots: vec![InvSlot::default(); SLOTS_PER_TAB],
        }
    }

    /// Find the first empty slot, if any.
    pub fn first_empty(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.is_empty())
    }

    /// Number of used slots.
    pub fn used_slots(&self) -> usize {
        self.slots.iter().filter(|s| !s.is_empty()).count()
    }
}

/// Why a guild bank operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildBankError {
    /// All 8 tabs already purchased.
    MaxTabsReached,
    /// Not enough gold to purchase the tab.
    NotEnoughGold,
    /// Tab index out of range.
    InvalidTab,
    /// Slot index out of range.
    InvalidSlot,
    /// Tab is full.
    TabFull,
}

/// The guild bank.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GuildBank {
    pub tabs: Vec<GuildBankTab>,
    /// Guild gold pool in copper.
    pub gold: u32,
}

impl GuildBank {
    /// Number of purchased tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Cost to purchase the next tab (copper). `None` if all tabs bought.
    pub fn next_tab_cost(&self) -> Option<u32> {
        TAB_COSTS.get(self.tabs.len()).copied()
    }

    /// Purchase a new tab. Deducts gold from the guild bank.
    pub fn purchase_tab(&mut self, name: String, guild_gold: &mut u32) -> Result<usize, GuildBankError> {
        let cost = self.next_tab_cost().ok_or(GuildBankError::MaxTabsReached)?;
        if *guild_gold < cost {
            return Err(GuildBankError::NotEnoughGold);
        }
        *guild_gold -= cost;
        self.tabs.push(GuildBankTab::new(name));
        Ok(self.tabs.len() - 1)
    }

    /// Deposit an item into a specific tab and slot.
    pub fn deposit(&mut self, tab: usize, slot: usize, item: InvSlot) -> Result<(), GuildBankError> {
        let tab = self.tabs.get_mut(tab).ok_or(GuildBankError::InvalidTab)?;
        let dest = tab.slots.get_mut(slot).ok_or(GuildBankError::InvalidSlot)?;
        *dest = item;
        Ok(())
    }

    /// Withdraw an item from a specific tab and slot.
    pub fn withdraw(&mut self, tab: usize, slot: usize) -> Result<InvSlot, GuildBankError> {
        let tab = self.tabs.get_mut(tab).ok_or(GuildBankError::InvalidTab)?;
        let src = tab.slots.get_mut(slot).ok_or(GuildBankError::InvalidSlot)?;
        let item = *src;
        *src = InvSlot::default();
        Ok(item)
    }

    /// Deposit gold into the guild bank.
    pub fn deposit_gold(&mut self, amount: u32) {
        self.gold = self.gold.saturating_add(amount);
    }

    /// Withdraw gold from the guild bank.
    pub fn withdraw_gold(&mut self, amount: u32) -> Result<u32, GuildBankError> {
        if self.gold < amount {
            return Err(GuildBankError::NotEnoughGold);
        }
        self.gold -= amount;
        Ok(amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purchase_first_tab() {
        let mut bank = GuildBank::default();
        let mut gold = 2_000_000; // 200g
        let idx = bank.purchase_tab("General".into(), &mut gold).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(bank.tab_count(), 1);
        assert_eq!(gold, 1_000_000); // 200g - 100g
    }

    #[test]
    fn purchase_costs_increase() {
        let mut bank = GuildBank::default();
        let cost1 = bank.next_tab_cost().unwrap();
        let mut gold = 999_999_999;
        bank.purchase_tab("T1".into(), &mut gold).unwrap();
        let cost2 = bank.next_tab_cost().unwrap();
        assert!(cost2 > cost1);
    }

    #[test]
    fn purchase_max_tabs() {
        let mut bank = GuildBank::default();
        let mut gold = 999_999_999;
        for i in 0..MAX_TABS {
            bank.purchase_tab(format!("Tab {i}"), &mut gold).unwrap();
        }
        assert_eq!(bank.tab_count(), 8);
        assert_eq!(bank.next_tab_cost(), None);
        assert_eq!(
            bank.purchase_tab("Extra".into(), &mut gold),
            Err(GuildBankError::MaxTabsReached)
        );
    }

    #[test]
    fn purchase_not_enough_gold() {
        let mut bank = GuildBank::default();
        let mut gold = 500_000; // 50g, need 100g
        assert_eq!(
            bank.purchase_tab("T".into(), &mut gold),
            Err(GuildBankError::NotEnoughGold)
        );
    }

    #[test]
    fn tab_has_98_slots() {
        let tab = GuildBankTab::new("Test".into());
        assert_eq!(tab.slots.len(), 98);
        assert_eq!(tab.used_slots(), 0);
    }

    #[test]
    fn deposit_and_withdraw() {
        let mut bank = GuildBank::default();
        let mut gold = 2_000_000;
        bank.purchase_tab("T".into(), &mut gold).unwrap();

        let item = InvSlot { item_id: 100, count: 5 };
        bank.deposit(0, 0, item).unwrap();
        assert_eq!(bank.tabs[0].slots[0], item);

        let withdrawn = bank.withdraw(0, 0).unwrap();
        assert_eq!(withdrawn, item);
        assert!(bank.tabs[0].slots[0].is_empty());
    }

    #[test]
    fn deposit_invalid_tab() {
        let mut bank = GuildBank::default();
        let item = InvSlot { item_id: 1, count: 1 };
        assert_eq!(bank.deposit(0, 0, item), Err(GuildBankError::InvalidTab));
    }

    #[test]
    fn gold_deposit_withdraw() {
        let mut bank = GuildBank::default();
        bank.deposit_gold(10000);
        assert_eq!(bank.gold, 10000);
        bank.withdraw_gold(3000).unwrap();
        assert_eq!(bank.gold, 7000);
        assert_eq!(bank.withdraw_gold(99999), Err(GuildBankError::NotEnoughGold));
    }
}
