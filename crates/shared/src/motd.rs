//! Message of the Day and autobroadcast system.
//!
//! MOTD is displayed to players on login. Configurable via admin IPC.
//! Autobroadcast sends periodic server-wide messages.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Maximum MOTD length in characters.
pub const MAX_MOTD_LEN: usize = 500;

/// Server Message of the Day.
///
/// Displayed to every player on login. Stored as a Bevy resource.
#[derive(Debug, Clone, PartialEq, Eq, Default, Resource, Serialize, Deserialize)]
pub struct Motd {
    /// The message text. Empty string means no MOTD.
    pub message: String,
    /// Server timestamp when the MOTD was last updated.
    pub updated_at: u64,
}

/// Why a MOTD operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotdError {
    /// Message exceeds maximum length.
    TooLong,
}

impl Motd {
    /// Set the MOTD message. Returns error if too long.
    pub fn set(&mut self, message: &str, now: u64) -> Result<(), MotdError> {
        if message.len() > MAX_MOTD_LEN {
            return Err(MotdError::TooLong);
        }
        self.message = message.to_string();
        self.updated_at = now;
        Ok(())
    }

    /// Clear the MOTD.
    pub fn clear(&mut self, now: u64) {
        self.message.clear();
        self.updated_at = now;
    }

    /// Whether there is an active MOTD.
    pub fn is_set(&self) -> bool {
        !self.message.is_empty()
    }
}

#[cfg(test)]
#[path = "motd_tests.rs"]
mod tests;
