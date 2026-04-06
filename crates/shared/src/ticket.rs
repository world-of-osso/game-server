//! GM ticket system: player support requests.
//!
//! Players submit tickets with a category and description.
//! GMs claim, respond, and resolve tickets via admin IPC.
//! Ref: AzerothCore `TicketMgr.cpp`.

use serde::{Deserialize, Serialize};

/// Ticket category for routing and prioritization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TicketCategory {
    /// Stuck character (can't move, fell through world).
    Stuck,
    /// Bug report.
    Bug,
    /// Harassment or player behavior.
    Harassment,
    /// Item or loot issue.
    Item,
    /// Quest issue (broken objective, missing NPC).
    Quest,
    /// Account or billing question.
    Account,
    /// Other / general.
    Other,
}

/// Current state of a ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TicketState {
    /// Waiting in queue for a GM.
    Open,
    /// A GM has claimed the ticket.
    Assigned,
    /// GM has resolved the issue.
    Resolved,
    /// Escalated to a senior GM.
    Escalated,
}

/// A player-submitted support ticket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ticket {
    /// Unique ticket ID.
    pub id: u64,
    /// Player who submitted the ticket.
    pub player: u64,
    /// Category for routing.
    pub category: TicketCategory,
    /// Player's description of the issue.
    pub description: String,
    /// Current ticket state.
    pub state: TicketState,
    /// Server timestamp when created.
    pub created_at: u64,
}

/// Why a ticket operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketError {
    /// Player already has an open ticket.
    AlreadyHasTicket,
    /// Description is empty.
    EmptyDescription,
    /// Description exceeds maximum length.
    DescriptionTooLong,
    /// No ticket found with this ID.
    NotFound,
}

/// Maximum description length in characters.
pub const MAX_DESCRIPTION_LEN: usize = 500;

/// Manages all active tickets.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TicketManager {
    tickets: Vec<Ticket>,
    next_id: u64,
}

impl TicketManager {
    pub fn new() -> Self {
        Self { tickets: Vec::new(), next_id: 1 }
    }

    /// Create a new ticket. One open ticket per player.
    pub fn create(
        &mut self,
        player: u64,
        category: TicketCategory,
        description: &str,
        now: u64,
    ) -> Result<u64, TicketError> {
        if description.is_empty() {
            return Err(TicketError::EmptyDescription);
        }
        if description.len() > MAX_DESCRIPTION_LEN {
            return Err(TicketError::DescriptionTooLong);
        }
        let has_open = self.tickets.iter().any(|t| {
            t.player == player && matches!(t.state, TicketState::Open | TicketState::Assigned | TicketState::Escalated)
        });
        if has_open {
            return Err(TicketError::AlreadyHasTicket);
        }
        let id = self.next_id;
        self.next_id += 1;
        self.tickets.push(Ticket {
            id, player, category,
            description: description.to_string(),
            state: TicketState::Open,
            created_at: now,
        });
        Ok(id)
    }

    /// Look up a ticket by ID.
    pub fn get(&self, id: u64) -> Option<&Ticket> {
        self.tickets.iter().find(|t| t.id == id)
    }

    /// Find a player's active (non-resolved) ticket.
    pub fn find_player_ticket(&self, player: u64) -> Option<&Ticket> {
        self.tickets.iter().find(|t| {
            t.player == player && !matches!(t.state, TicketState::Resolved)
        })
    }

    /// Number of tickets in the system.
    pub fn len(&self) -> usize {
        self.tickets.len()
    }

    /// Whether there are no tickets.
    pub fn is_empty(&self) -> bool {
        self.tickets.is_empty()
    }

    /// All open tickets (not yet assigned).
    pub fn open_tickets(&self) -> Vec<&Ticket> {
        self.tickets.iter().filter(|t| t.state == TicketState::Open).collect()
    }
}

#[cfg(test)]
#[path = "ticket_tests.rs"]
mod tests;
