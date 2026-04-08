//! Warband Bank: account-wide shared storage with tabs and gold.
//!
//! Simpler than the guild bank — no permissions, no daily limits, no transaction log.

use serde::{Deserialize, Serialize};

use crate::item_data::InvSlot;

/// Maximum warband bank tabs.
pub const MAX_TABS: usize = 5;
/// Slots per tab.
pub const SLOTS_PER_TAB: usize = 98;

/// Cost to purchase each tab after the first (in copper).
const TAB_COSTS: [u32; MAX_TABS - 1] = [
    10_000_000, // 1000g
    25_000_000, // 2500g
    50_000_000, // 5000g
    50_000_000, // 5000g
];

/// A single warband bank tab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WarbankTab {
    /// Tab display name.
    pub name: String,
    /// 98 item slots.
    pub slots: Vec<InvSlot>,
}

impl WarbankTab {
    pub fn new(name: String) -> Self {
        Self {
            name,
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

/// Why a warband bank operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarbankError {
    /// All 5 tabs already purchased.
    MaxTabsReached,
    /// Not enough gold to purchase the tab.
    NotEnoughGold,
    /// Tab index out of range.
    InvalidTab,
    /// Slot index out of range.
    InvalidSlot,
}

/// The warband bank — account-wide personal storage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Warbank {
    pub tabs: Vec<WarbankTab>,
    /// Account gold pool in copper.
    pub gold: u32,
}

impl Warbank {
    /// Create a new warband bank. The first tab ("Warband") is free and included by default.
    pub fn new() -> Self {
        Self {
            tabs: vec![WarbankTab::new("Warband".into())],
            gold: 0,
        }
    }

    /// Number of purchased tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Cost to purchase the next tab (copper). `None` if all tabs bought.
    pub fn next_tab_cost(&self) -> Option<u32> {
        // tabs.len() == 1 → index 0 in TAB_COSTS (second tab costs 1000g)
        TAB_COSTS.get(self.tabs.len() - 1).copied()
    }

    /// Purchase a new tab. Deducts gold from the warband bank.
    pub fn purchase_tab(&mut self, name: String) -> Result<usize, WarbankError> {
        let cost = self.next_tab_cost().ok_or(WarbankError::MaxTabsReached)?;
        if self.gold < cost {
            return Err(WarbankError::NotEnoughGold);
        }
        self.gold -= cost;
        self.tabs.push(WarbankTab::new(name));
        Ok(self.tabs.len() - 1)
    }

    /// Deposit an item into a specific tab and slot.
    pub fn deposit(&mut self, tab: usize, slot: usize, item: InvSlot) -> Result<(), WarbankError> {
        let tab = self.tabs.get_mut(tab).ok_or(WarbankError::InvalidTab)?;
        let dest = tab.slots.get_mut(slot).ok_or(WarbankError::InvalidSlot)?;
        *dest = item;
        Ok(())
    }

    /// Withdraw an item from a specific tab and slot. Returns the item (empty slot if nothing there).
    pub fn withdraw(&mut self, tab: usize, slot: usize) -> Result<InvSlot, WarbankError> {
        let tab = self.tabs.get_mut(tab).ok_or(WarbankError::InvalidTab)?;
        let src = tab.slots.get_mut(slot).ok_or(WarbankError::InvalidSlot)?;
        let item = *src;
        *src = InvSlot::default();
        Ok(item)
    }

    /// Deposit gold into the warband bank.
    pub fn deposit_gold(&mut self, amount: u32) {
        self.gold = self.gold.saturating_add(amount);
    }

    /// Withdraw gold from the warband bank.
    pub fn withdraw_gold(&mut self, amount: u32) -> Result<u32, WarbankError> {
        if self.gold < amount {
            return Err(WarbankError::NotEnoughGold);
        }
        self.gold -= amount;
        Ok(amount)
    }
}

impl Default for Warbank {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_with_one_tab() {
        let bank = Warbank::new();
        assert_eq!(bank.tab_count(), 1);
        assert_eq!(bank.tabs[0].name, "Warband");
    }

    #[test]
    fn purchase_second_tab() {
        let mut bank = Warbank::new();
        bank.deposit_gold(10_000_000); // 1000g exact
        let idx = bank.purchase_tab("Gear".into()).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(bank.tab_count(), 2);
        assert_eq!(bank.gold, 0);
    }

    #[test]
    fn purchase_costs_increase() {
        let mut bank = Warbank::new();
        bank.deposit_gold(999_999_999);
        let cost1 = bank.next_tab_cost().unwrap();
        bank.purchase_tab("T2".into()).unwrap();
        let cost2 = bank.next_tab_cost().unwrap();
        assert!(cost2 > cost1);
    }

    #[test]
    fn purchase_max_tabs() {
        let mut bank = Warbank::new();
        bank.deposit_gold(999_999_999);
        for i in 2..=MAX_TABS {
            bank.purchase_tab(format!("Tab {i}")).unwrap();
        }
        assert_eq!(bank.tab_count(), MAX_TABS);
        assert_eq!(bank.next_tab_cost(), None);
        assert_eq!(
            bank.purchase_tab("Extra".into()),
            Err(WarbankError::MaxTabsReached)
        );
    }

    #[test]
    fn purchase_not_enough_gold() {
        let mut bank = Warbank::new();
        bank.deposit_gold(9_999_999); // 999.99g, need 1000g
        assert_eq!(
            bank.purchase_tab("T".into()),
            Err(WarbankError::NotEnoughGold)
        );
    }

    #[test]
    fn tab_has_98_slots() {
        let bank = Warbank::new();
        assert_eq!(bank.tabs[0].slots.len(), SLOTS_PER_TAB);
        assert_eq!(bank.tabs[0].used_slots(), 0);
    }

    #[test]
    fn deposit_and_withdraw() {
        let mut bank = Warbank::new();
        let item = InvSlot {
            item_id: 100,
            count: 5,
        };
        bank.deposit(0, 0, item).unwrap();
        assert_eq!(bank.tabs[0].slots[0], item);

        let withdrawn = bank.withdraw(0, 0).unwrap();
        assert_eq!(withdrawn, item);
        assert!(bank.tabs[0].slots[0].is_empty());
    }

    #[test]
    fn deposit_invalid_tab() {
        let mut bank = Warbank::new();
        let item = InvSlot {
            item_id: 1,
            count: 1,
        };
        assert_eq!(bank.deposit(1, 0, item), Err(WarbankError::InvalidTab));
    }

    #[test]
    fn withdraw_empty_slot() {
        let mut bank = Warbank::new();
        let result = bank.withdraw(0, 0).unwrap();
        assert_eq!(result, InvSlot::default());
    }

    #[test]
    fn gold_deposit_withdraw() {
        let mut bank = Warbank::new();
        bank.deposit_gold(10_000);
        assert_eq!(bank.gold, 10_000);
        bank.withdraw_gold(3_000).unwrap();
        assert_eq!(bank.gold, 7_000);
        assert_eq!(bank.withdraw_gold(99_999), Err(WarbankError::NotEnoughGold));
    }

    #[test]
    fn gold_deposit_saturates() {
        let mut bank = Warbank::new();
        bank.deposit_gold(u32::MAX);
        bank.deposit_gold(1); // saturating_add — must not panic or wrap
        assert_eq!(bank.gold, u32::MAX);
    }

    #[test]
    fn first_empty_slot() {
        let mut bank = Warbank::new();
        let item = InvSlot {
            item_id: 42,
            count: 1,
        };
        bank.deposit(0, 0, item).unwrap();
        bank.deposit(0, 1, item).unwrap();
        let first = bank.tabs[0].first_empty().unwrap();
        assert_eq!(first, 2);
    }

    #[test]
    fn first_empty_slot_none_when_full() {
        let mut bank = Warbank::new();
        let item = InvSlot {
            item_id: 1,
            count: 1,
        };
        for slot in 0..SLOTS_PER_TAB {
            bank.deposit(0, slot, item).unwrap();
        }
        assert_eq!(bank.tabs[0].first_empty(), None);
        assert_eq!(bank.tabs[0].used_slots(), SLOTS_PER_TAB);
    }

    #[test]
    fn deposit_invalid_slot() {
        let mut bank = Warbank::new();
        let item = InvSlot {
            item_id: 1,
            count: 1,
        };
        assert_eq!(
            bank.deposit(0, SLOTS_PER_TAB, item),
            Err(WarbankError::InvalidSlot)
        );
        assert_eq!(bank.deposit(0, 999, item), Err(WarbankError::InvalidSlot));
    }

    #[test]
    fn withdraw_invalid_tab() {
        let mut bank = Warbank::new();
        assert_eq!(bank.withdraw(5, 0), Err(WarbankError::InvalidTab));
    }

    #[test]
    fn withdraw_invalid_slot() {
        let mut bank = Warbank::new();
        assert_eq!(
            bank.withdraw(0, SLOTS_PER_TAB),
            Err(WarbankError::InvalidSlot)
        );
    }

    #[test]
    fn deposit_overwrites_existing_item() {
        let mut bank = Warbank::new();
        let item_a = InvSlot {
            item_id: 10,
            count: 1,
        };
        let item_b = InvSlot {
            item_id: 20,
            count: 3,
        };
        bank.deposit(0, 0, item_a).unwrap();
        bank.deposit(0, 0, item_b).unwrap();
        assert_eq!(bank.tabs[0].slots[0], item_b);
    }

    #[test]
    fn withdraw_gold_zero_is_noop() {
        let mut bank = Warbank::new();
        bank.deposit_gold(100);
        assert_eq!(bank.withdraw_gold(0).unwrap(), 0);
        assert_eq!(bank.gold, 100);
    }

    #[test]
    fn withdraw_gold_exact_balance() {
        let mut bank = Warbank::new();
        bank.deposit_gold(5000);
        assert_eq!(bank.withdraw_gold(5000).unwrap(), 5000);
        assert_eq!(bank.gold, 0);
    }

    #[test]
    fn purchase_deducts_correct_amounts() {
        let mut bank = Warbank::new();
        bank.deposit_gold(999_999_999);
        let before = bank.gold;
        bank.purchase_tab("T2".into()).unwrap();
        assert_eq!(before - bank.gold, 10_000_000); // 1000g
        let before = bank.gold;
        bank.purchase_tab("T3".into()).unwrap();
        assert_eq!(before - bank.gold, 25_000_000); // 2500g
        let before = bank.gold;
        bank.purchase_tab("T4".into()).unwrap();
        assert_eq!(before - bank.gold, 50_000_000); // 5000g
        let before = bank.gold;
        bank.purchase_tab("T5".into()).unwrap();
        assert_eq!(before - bank.gold, 50_000_000); // 5000g
    }

    #[test]
    fn tabs_are_independent() {
        let mut bank = Warbank::new();
        bank.deposit_gold(999_999_999);
        bank.purchase_tab("T2".into()).unwrap();
        let item = InvSlot {
            item_id: 42,
            count: 1,
        };
        bank.deposit(0, 0, item).unwrap();
        assert!(bank.tabs[1].slots[0].is_empty());
    }

    #[test]
    fn new_tab_starts_empty() {
        let mut bank = Warbank::new();
        bank.deposit_gold(999_999_999);
        bank.purchase_tab("New".into()).unwrap();
        assert_eq!(bank.tabs[1].used_slots(), 0);
        assert_eq!(bank.tabs[1].slots.len(), SLOTS_PER_TAB);
    }

    #[test]
    fn default_equals_new() {
        let from_new = Warbank::new();
        let from_default = Warbank::default();
        assert_eq!(from_new, from_default);
    }

    #[test]
    fn serde_round_trip() {
        let mut bank = Warbank::new();
        bank.deposit_gold(50_000);
        let item = InvSlot {
            item_id: 77,
            count: 20,
        };
        bank.deposit(0, 5, item).unwrap();

        let json = serde_json::to_string(&bank).unwrap();
        let restored: Warbank = serde_json::from_str(&json).unwrap();
        assert_eq!(bank, restored);
    }
}
