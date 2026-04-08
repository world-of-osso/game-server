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
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Default,
    Resource,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
)]
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

// --- Autobroadcast ---

/// Default autobroadcast interval in seconds.
pub const DEFAULT_BROADCAST_INTERVAL: u64 = 900; // 15 minutes

/// Periodic server-wide message broadcaster.
///
/// Cycles through a list of messages at a configurable interval.
/// The server checks `should_broadcast()` each tick and sends the
/// returned message to all players.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Default,
    Resource,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
)]
pub struct Autobroadcast {
    /// Messages to cycle through.
    pub messages: Vec<String>,
    /// Seconds between broadcasts.
    pub interval: u64,
    /// Whether broadcasting is enabled.
    pub enabled: bool,
    /// Index of the next message to send.
    next_index: usize,
    /// Timestamp of the last broadcast.
    last_broadcast: u64,
}

impl Autobroadcast {
    /// Create a new autobroadcast with default interval, disabled.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            interval: DEFAULT_BROADCAST_INTERVAL,
            enabled: false,
            next_index: 0,
            last_broadcast: 0,
        }
    }

    /// Add a message to the rotation.
    pub fn add_message(&mut self, message: String) {
        self.messages.push(message);
    }

    /// Remove a message by index. Returns the removed message.
    pub fn remove_message(&mut self, index: usize) -> Option<String> {
        if index >= self.messages.len() {
            return None;
        }
        let removed = self.messages.remove(index);
        if self.next_index >= self.messages.len() && !self.messages.is_empty() {
            self.next_index = 0;
        }
        Some(removed)
    }

    /// Check if it's time to broadcast. Returns the message if so.
    ///
    /// Call this each server tick with the current timestamp.
    pub fn should_broadcast(&mut self, now: u64) -> Option<&str> {
        if !self.enabled || self.messages.is_empty() {
            return None;
        }
        if now.saturating_sub(self.last_broadcast) < self.interval {
            return None;
        }
        self.last_broadcast = now;
        let msg = &self.messages[self.next_index];
        self.next_index = (self.next_index + 1) % self.messages.len();
        Some(msg)
    }

    /// Number of messages in the rotation.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

// --- Chat integration ---

use crate::protocol::{ChatMessage, ChatType};

/// Sender name used for server system messages.
pub const SYSTEM_SENDER: &str = "[Server]";

impl Motd {
    /// Build a system chat message for login display.
    /// Returns `None` if no MOTD is set.
    pub fn as_chat_message(&self) -> Option<ChatMessage> {
        if !self.is_set() {
            return None;
        }
        Some(ChatMessage {
            sender: SYSTEM_SENDER.to_string(),
            content: self.message.clone(),
            channel: ChatType::System,
        })
    }
}

impl Autobroadcast {
    /// Check the timer and build a broadcast chat message if due.
    pub fn tick_chat_message(&mut self, now: u64) -> Option<ChatMessage> {
        let msg = self.should_broadcast(now)?;
        Some(ChatMessage {
            sender: SYSTEM_SENDER.to_string(),
            content: msg.to_string(),
            channel: ChatType::ServerBroadcast,
        })
    }
}

#[cfg(test)]
#[path = "motd_tests.rs"]
mod tests;
