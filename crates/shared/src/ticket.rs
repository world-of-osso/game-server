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

/// Priority level for queue ordering. Higher = handled first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TicketPriority {
    /// Standard issues (bugs, quests, items, account, other).
    Normal,
    /// Time-sensitive issues (stuck characters).
    High,
    /// Safety issues (harassment, exploits).
    Critical,
}

/// Default priority based on category.
pub fn default_priority(category: TicketCategory) -> TicketPriority {
    match category {
        TicketCategory::Harassment => TicketPriority::Critical,
        TicketCategory::Stuck => TicketPriority::High,
        _ => TicketPriority::Normal,
    }
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
    /// Queue priority (derived from category, can be overridden by GM).
    pub priority: TicketPriority,
    /// GM who claimed this ticket (None if unassigned).
    pub assigned_to: Option<String>,
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
    /// Ticket is already assigned to a GM.
    AlreadyAssigned,
    /// Ticket is not in a claimable state (not Open).
    NotOpen,
    /// Ticket is not assigned (can't unassign).
    NotAssigned,
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
            priority: default_priority(category),
            assigned_to: None,
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

    /// Open tickets sorted by priority (highest first), then FIFO (oldest first).
    pub fn queue(&self) -> Vec<&Ticket> {
        let mut queue: Vec<&Ticket> = self.open_tickets();
        queue.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.created_at.cmp(&b.created_at)));
        queue
    }

    /// The next ticket a GM should handle (highest priority, oldest).
    pub fn next_in_queue(&self) -> Option<&Ticket> {
        self.queue().into_iter().next()
    }

    /// Override a ticket's priority (GM escalation/de-escalation).
    pub fn set_priority(&mut self, id: u64, priority: TicketPriority) -> Result<(), TicketError> {
        let ticket = self.tickets.iter_mut().find(|t| t.id == id)
            .ok_or(TicketError::NotFound)?;
        ticket.priority = priority;
        Ok(())
    }

    /// Count of open tickets per priority level.
    pub fn queue_counts(&self) -> (usize, usize, usize) {
        let open = self.open_tickets();
        let normal = open.iter().filter(|t| t.priority == TicketPriority::Normal).count();
        let high = open.iter().filter(|t| t.priority == TicketPriority::High).count();
        let critical = open.iter().filter(|t| t.priority == TicketPriority::Critical).count();
        (critical, high, normal)
    }

    /// GM claims a ticket. Moves it from Open → Assigned.
    pub fn assign(&mut self, id: u64, gm_name: &str) -> Result<(), TicketError> {
        let ticket = self.tickets.iter_mut().find(|t| t.id == id)
            .ok_or(TicketError::NotFound)?;
        if ticket.state != TicketState::Open {
            return Err(TicketError::NotOpen);
        }
        if ticket.assigned_to.is_some() {
            return Err(TicketError::AlreadyAssigned);
        }
        ticket.state = TicketState::Assigned;
        ticket.assigned_to = Some(gm_name.to_string());
        Ok(())
    }

    /// Release a ticket back to the queue. Moves Assigned → Open.
    pub fn unassign(&mut self, id: u64) -> Result<(), TicketError> {
        let ticket = self.tickets.iter_mut().find(|t| t.id == id)
            .ok_or(TicketError::NotFound)?;
        if ticket.assigned_to.is_none() {
            return Err(TicketError::NotAssigned);
        }
        ticket.state = TicketState::Open;
        ticket.assigned_to = None;
        Ok(())
    }

    /// Claim the next ticket from the queue for a GM.
    pub fn claim_next(&mut self, gm_name: &str) -> Result<u64, TicketError> {
        let id = self.next_in_queue().ok_or(TicketError::NotFound)?.id;
        self.assign(id, gm_name)?;
        Ok(id)
    }

    /// All tickets assigned to a specific GM.
    pub fn assigned_to(&self, gm_name: &str) -> Vec<&Ticket> {
        self.tickets.iter()
            .filter(|t| t.assigned_to.as_deref() == Some(gm_name))
            .collect()
    }
}

#[cfg(test)]
#[path = "ticket_tests.rs"]
mod tests;
