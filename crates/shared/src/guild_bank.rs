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
    /// Member doesn't have permission for this operation.
    PermissionDenied,
    /// Daily withdrawal limit reached.
    DailyLimitReached,
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
    pub fn purchase_tab(
        &mut self,
        name: String,
        guild_gold: &mut u32,
    ) -> Result<usize, GuildBankError> {
        let cost = self.next_tab_cost().ok_or(GuildBankError::MaxTabsReached)?;
        if *guild_gold < cost {
            return Err(GuildBankError::NotEnoughGold);
        }
        *guild_gold -= cost;
        self.tabs.push(GuildBankTab::new(name));
        Ok(self.tabs.len() - 1)
    }

    /// Deposit an item into a specific tab and slot.
    pub fn deposit(
        &mut self,
        tab: usize,
        slot: usize,
        item: InvSlot,
    ) -> Result<(), GuildBankError> {
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

// --- Tab permissions ---

/// Permissions for a guild rank on a specific tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TabPermission {
    /// Can see tab contents.
    pub view: bool,
    /// Can deposit items.
    pub deposit: bool,
    /// Can withdraw items.
    pub withdraw: bool,
    /// Maximum withdrawals per day (0 = unlimited for guild master).
    pub daily_withdraw_limit: u32,
}

impl TabPermission {
    /// Full access (guild master).
    pub fn full() -> Self {
        Self {
            view: true,
            deposit: true,
            withdraw: true,
            daily_withdraw_limit: 0, // unlimited
        }
    }

    /// Deposit-only access (typical for new members).
    pub fn deposit_only() -> Self {
        Self {
            view: true,
            deposit: true,
            withdraw: false,
            daily_withdraw_limit: 0,
        }
    }
}

/// Per-rank permissions across all tabs.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RankPermissions {
    /// Permission per tab index. Absent entries = no access.
    pub tabs: Vec<TabPermission>,
}

/// Per-member daily withdrawal counter.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DailyWithdrawals {
    /// (tab_index, count_today) pairs.
    pub counts: Vec<(usize, u32)>,
}

impl DailyWithdrawals {
    /// Get today's withdrawal count for a tab.
    pub fn count_for(&self, tab: usize) -> u32 {
        self.counts
            .iter()
            .find(|(t, _)| *t == tab)
            .map_or(0, |(_, c)| *c)
    }

    /// Increment the withdrawal count for a tab.
    pub fn increment(&mut self, tab: usize) {
        if let Some(entry) = self.counts.iter_mut().find(|(t, _)| *t == tab) {
            entry.1 += 1;
        } else {
            self.counts.push((tab, 1));
        }
    }

    /// Reset all counters (called at daily reset).
    pub fn reset(&mut self) {
        self.counts.clear();
    }
}

/// Check if a member can withdraw from a tab given permissions and daily limit.
pub fn can_withdraw(perm: &TabPermission, daily: &DailyWithdrawals, tab: usize) -> bool {
    if !perm.withdraw {
        return false;
    }
    // daily_withdraw_limit == 0 means unlimited
    if perm.daily_withdraw_limit == 0 {
        return true;
    }
    daily.count_for(tab) < perm.daily_withdraw_limit
}

// --- Guild repair ---

/// Per-member daily repair spending from guild funds.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DailyRepairSpending {
    /// (player_name, copper_spent_today) pairs.
    entries: Vec<(String, u32)>,
}

impl DailyRepairSpending {
    /// Get today's repair spending for a player.
    pub fn spent_by(&self, player: &str) -> u32 {
        self.entries
            .iter()
            .find(|(p, _)| p == player)
            .map_or(0, |(_, s)| *s)
    }

    /// Record a repair charge.
    pub fn add(&mut self, player: &str, amount: u32) {
        if let Some(entry) = self.entries.iter_mut().find(|(p, _)| p == player) {
            entry.1 += amount;
        } else {
            self.entries.push((player.to_string(), amount));
        }
    }

    /// Reset all spending (daily reset).
    pub fn reset(&mut self) {
        self.entries.clear();
    }
}

/// Attempt to repair from guild funds.
///
/// `repair_cost`: total repair cost in copper.
/// `daily_limit`: max copper this rank can spend per day (0 = unlimited).
///
/// Returns the amount actually paid from guild funds, or an error.
pub fn repair_from_guild(
    bank: &mut GuildBank,
    spending: &mut DailyRepairSpending,
    player: &str,
    repair_cost: u32,
    daily_limit: u32,
) -> Result<u32, GuildBankError> {
    let already_spent = spending.spent_by(player);
    let remaining_allowance = if daily_limit == 0 {
        u32::MAX
    } else {
        daily_limit.saturating_sub(already_spent)
    };

    if remaining_allowance == 0 {
        return Err(GuildBankError::DailyLimitReached);
    }

    let pay = repair_cost.min(remaining_allowance);
    if bank.gold < pay {
        return Err(GuildBankError::NotEnoughGold);
    }

    bank.gold -= pay;
    spending.add(player, pay);
    Ok(pay)
}

// --- Transaction log ---

/// Type of guild bank transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    DepositItem {
        tab: usize,
        slot: usize,
        item_id: u32,
        count: u16,
    },
    WithdrawItem {
        tab: usize,
        slot: usize,
        item_id: u32,
        count: u16,
    },
    DepositGold {
        amount: u32,
    },
    WithdrawGold {
        amount: u32,
    },
    RepairFromGuild {
        amount: u32,
    },
    TabPurchased {
        tab: usize,
    },
}

/// A single transaction log entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// Character name who performed the action.
    pub player: String,
    /// What happened.
    pub action: TransactionType,
    /// Server timestamp.
    pub timestamp: u64,
}

/// Maximum log entries per tab (WoW keeps ~25 per tab).
const MAX_LOG_PER_TAB: usize = 25;
/// Maximum money log entries.
const MAX_MONEY_LOG: usize = 25;

/// Guild bank transaction log.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TransactionLog {
    /// Per-tab item transaction logs.
    pub item_logs: Vec<Vec<Transaction>>,
    /// Money transaction log (deposits, withdrawals, repairs).
    pub money_log: Vec<Transaction>,
}

impl TransactionLog {
    /// Initialize with the given number of tabs.
    pub fn new(tab_count: usize) -> Self {
        Self {
            item_logs: vec![Vec::new(); tab_count],
            money_log: Vec::new(),
        }
    }

    /// Add a tab log when a new tab is purchased.
    pub fn add_tab(&mut self) {
        self.item_logs.push(Vec::new());
    }

    /// Record an item transaction for a tab.
    pub fn log_item(&mut self, tab: usize, entry: Transaction) {
        if let Some(log) = self.item_logs.get_mut(tab) {
            log.push(entry);
            if log.len() > MAX_LOG_PER_TAB {
                log.remove(0);
            }
        }
    }

    /// Record a money transaction.
    pub fn log_money(&mut self, entry: Transaction) {
        self.money_log.push(entry);
        if self.money_log.len() > MAX_MONEY_LOG {
            self.money_log.remove(0);
        }
    }

    /// Get the item log for a tab.
    pub fn tab_log(&self, tab: usize) -> &[Transaction] {
        self.item_logs.get(tab).map_or(&[], |v| v.as_slice())
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
        let mut bank = GuildBank::default();
        let item = InvSlot {
            item_id: 1,
            count: 1,
        };
        assert_eq!(bank.deposit(0, 0, item), Err(GuildBankError::InvalidTab));
    }

    #[test]
    fn gold_deposit_withdraw() {
        let mut bank = GuildBank::default();
        bank.deposit_gold(10000);
        assert_eq!(bank.gold, 10000);
        bank.withdraw_gold(3000).unwrap();
        assert_eq!(bank.gold, 7000);
        assert_eq!(
            bank.withdraw_gold(99999),
            Err(GuildBankError::NotEnoughGold)
        );
    }

    // --- Permission tests ---

    #[test]
    fn full_permission_allows_all() {
        let perm = TabPermission::full();
        let daily = DailyWithdrawals::default();
        assert!(perm.view);
        assert!(perm.deposit);
        assert!(can_withdraw(&perm, &daily, 0));
    }

    #[test]
    fn deposit_only_blocks_withdraw() {
        let perm = TabPermission::deposit_only();
        let daily = DailyWithdrawals::default();
        assert!(perm.view);
        assert!(perm.deposit);
        assert!(!can_withdraw(&perm, &daily, 0));
    }

    #[test]
    fn daily_limit_enforced() {
        let perm = TabPermission {
            view: true,
            deposit: true,
            withdraw: true,
            daily_withdraw_limit: 3,
        };
        let mut daily = DailyWithdrawals::default();
        assert!(can_withdraw(&perm, &daily, 0));
        daily.increment(0);
        daily.increment(0);
        assert!(can_withdraw(&perm, &daily, 0)); // 2 < 3
        daily.increment(0);
        assert!(!can_withdraw(&perm, &daily, 0)); // 3 >= 3
    }

    #[test]
    fn daily_reset_clears_counts() {
        let mut daily = DailyWithdrawals::default();
        daily.increment(0);
        daily.increment(1);
        daily.reset();
        assert_eq!(daily.count_for(0), 0);
        assert_eq!(daily.count_for(1), 0);
    }

    #[test]
    fn unlimited_daily_always_allows() {
        let perm = TabPermission {
            withdraw: true,
            daily_withdraw_limit: 0, // unlimited
            ..Default::default()
        };
        let mut daily = DailyWithdrawals::default();
        for _ in 0..100 {
            daily.increment(0);
        }
        assert!(can_withdraw(&perm, &daily, 0));
    }

    #[test]
    fn no_permission_blocks() {
        let perm = TabPermission::default();
        let daily = DailyWithdrawals::default();
        assert!(!perm.view);
        assert!(!perm.deposit);
        assert!(!can_withdraw(&perm, &daily, 0));
    }

    // --- Transaction log tests ---

    #[test]
    fn log_item_transaction() {
        let mut log = TransactionLog::new(1);
        log.log_item(
            0,
            Transaction {
                player: "Alice".into(),
                action: TransactionType::DepositItem {
                    tab: 0,
                    slot: 0,
                    item_id: 100,
                    count: 5,
                },
                timestamp: 1000,
            },
        );
        assert_eq!(log.tab_log(0).len(), 1);
        assert_eq!(log.tab_log(0)[0].player, "Alice");
    }

    #[test]
    fn log_money_transaction() {
        let mut log = TransactionLog::new(0);
        log.log_money(Transaction {
            player: "Bob".into(),
            action: TransactionType::DepositGold { amount: 5000 },
            timestamp: 2000,
        });
        assert_eq!(log.money_log.len(), 1);
    }

    #[test]
    fn log_evicts_oldest() {
        let mut log = TransactionLog::new(1);
        for i in 0..30 {
            log.log_item(
                0,
                Transaction {
                    player: format!("P{i}"),
                    action: TransactionType::DepositItem {
                        tab: 0,
                        slot: 0,
                        item_id: 1,
                        count: 1,
                    },
                    timestamp: i as u64,
                },
            );
        }
        assert_eq!(log.tab_log(0).len(), 25); // capped
        assert_eq!(log.tab_log(0)[0].player, "P5"); // oldest 5 evicted
    }

    #[test]
    fn log_add_tab() {
        let mut log = TransactionLog::new(1);
        log.add_tab();
        assert_eq!(log.item_logs.len(), 2);
        assert!(log.tab_log(1).is_empty());
    }

    #[test]
    fn log_invalid_tab_empty() {
        let log = TransactionLog::new(1);
        assert!(log.tab_log(99).is_empty());
    }

    // --- Guild repair tests ---

    #[test]
    fn repair_from_guild_success() {
        let mut bank = GuildBank {
            gold: 100_000,
            ..Default::default()
        };
        let mut spending = DailyRepairSpending::default();
        let paid = repair_from_guild(&mut bank, &mut spending, "Alice", 5000, 10000).unwrap();
        assert_eq!(paid, 5000);
        assert_eq!(bank.gold, 95_000);
        assert_eq!(spending.spent_by("Alice"), 5000);
    }

    #[test]
    fn repair_daily_limit_caps() {
        let mut bank = GuildBank {
            gold: 100_000,
            ..Default::default()
        };
        let mut spending = DailyRepairSpending::default();
        // Limit is 3000, repair costs 5000 → only pays 3000
        let paid = repair_from_guild(&mut bank, &mut spending, "Alice", 5000, 3000).unwrap();
        assert_eq!(paid, 3000);
        // Second repair — no allowance left
        let err = repair_from_guild(&mut bank, &mut spending, "Alice", 1000, 3000);
        assert_eq!(err, Err(GuildBankError::DailyLimitReached));
    }

    #[test]
    fn repair_unlimited_daily() {
        let mut bank = GuildBank {
            gold: 100_000,
            ..Default::default()
        };
        let mut spending = DailyRepairSpending::default();
        // daily_limit = 0 means unlimited
        repair_from_guild(&mut bank, &mut spending, "Alice", 50_000, 0).unwrap();
        repair_from_guild(&mut bank, &mut spending, "Alice", 50_000, 0).unwrap();
        assert_eq!(bank.gold, 0);
    }

    #[test]
    fn repair_not_enough_guild_gold() {
        let mut bank = GuildBank {
            gold: 100,
            ..Default::default()
        };
        let mut spending = DailyRepairSpending::default();
        let err = repair_from_guild(&mut bank, &mut spending, "Alice", 5000, 10000);
        assert_eq!(err, Err(GuildBankError::NotEnoughGold));
    }

    #[test]
    fn repair_spending_reset() {
        let mut spending = DailyRepairSpending::default();
        spending.add("Alice", 5000);
        spending.add("Bob", 3000);
        spending.reset();
        assert_eq!(spending.spent_by("Alice"), 0);
        assert_eq!(spending.spent_by("Bob"), 0);
    }
}
