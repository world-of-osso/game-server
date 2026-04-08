use serde::{Deserialize, Serialize};

use super::InstanceError;

/// Maximum instances an account can enter per hour.
pub const INSTANCE_LIMIT_PER_HOUR: usize = 5;
/// Rate limit window in seconds (1 hour).
pub const RATE_LIMIT_WINDOW: u64 = 3600;

/// Per-account instance entry rate limiter.
///
/// Tracks timestamps of recent instance entries. Rejects new entries
/// if the account has entered 5+ instances within the last hour.
/// Ref: WoW's "You have entered too many instances recently" message.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstanceRateLimiter {
    /// Per-account entry timestamps: (account_id, Vec<entry_timestamp>).
    entries: std::collections::HashMap<u64, Vec<u64>>,
}

impl InstanceRateLimiter {
    pub fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
        }
    }

    /// Check if an account can enter a new instance.
    pub fn can_enter(&self, account_id: u64, now: u64) -> bool {
        self.recent_count(account_id, now) < INSTANCE_LIMIT_PER_HOUR
    }

    /// Record an instance entry for an account. Returns error if limit reached.
    pub fn record_entry(&mut self, account_id: u64, now: u64) -> Result<(), InstanceError> {
        if !self.can_enter(account_id, now) {
            return Err(InstanceError::InstanceLimitReached);
        }
        self.entries.entry(account_id).or_default().push(now);
        Ok(())
    }

    /// Number of instances entered in the last hour.
    pub fn recent_count(&self, account_id: u64, now: u64) -> usize {
        let cutoff = now.saturating_sub(RATE_LIMIT_WINDOW);
        self.entries
            .get(&account_id)
            .map_or(0, |ts| ts.iter().filter(|&&t| t > cutoff).count())
    }

    /// Seconds until the oldest entry expires and a slot opens up.
    /// Returns 0 if the account is not at the limit.
    pub fn cooldown_remaining(&self, account_id: u64, now: u64) -> u64 {
        if self.can_enter(account_id, now) {
            return 0;
        }
        let cutoff = now.saturating_sub(RATE_LIMIT_WINDOW);
        self.entries
            .get(&account_id)
            .and_then(|ts| ts.iter().filter(|&&t| t > cutoff).min())
            .map(|&oldest| (oldest + RATE_LIMIT_WINDOW).saturating_sub(now))
            .unwrap_or(0)
    }

    /// Remove expired entries for all accounts to reclaim memory.
    pub fn cleanup(&mut self, now: u64) {
        let cutoff = now.saturating_sub(RATE_LIMIT_WINDOW);
        self.entries.retain(|_, ts| {
            ts.retain(|&t| t > cutoff);
            !ts.is_empty()
        });
    }
}
