//! GM ticket system: player support requests.
//!
//! Players submit tickets with a category and description.
//! GMs claim, respond, and resolve tickets via admin IPC.
//! Ref: AzerothCore `TicketMgr.cpp`.

use bevy::prelude::*;
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

/// A GM response message on a ticket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TicketResponse {
    /// GM who wrote the response.
    pub gm_name: String,
    /// Response text.
    pub message: String,
    /// Server timestamp.
    pub timestamp: u64,
}

/// A notification for the player about their ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TicketNotification {
    /// A GM has responded to the ticket.
    GmResponse {
        ticket_id: u64,
        player: u64,
        gm_name: String,
        message: String,
    },
    /// Ticket has been assigned to a GM.
    Assigned {
        ticket_id: u64,
        player: u64,
        gm_name: String,
    },
    /// Ticket has been resolved.
    Resolved { ticket_id: u64, player: u64 },
    /// Ticket has been escalated.
    Escalated { ticket_id: u64, player: u64 },
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
    /// GM responses on this ticket.
    pub responses: Vec<TicketResponse>,
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
    /// Ticket is already resolved.
    AlreadyResolved,
    /// Invalid state transition (e.g. resolving an Open ticket).
    InvalidTransition,
}

/// Maximum description length in characters.
pub const MAX_DESCRIPTION_LEN: usize = 500;

/// Manages all active tickets.
#[derive(Debug, Clone, PartialEq, Default, Resource)]
pub struct TicketManager {
    tickets: Vec<Ticket>,
    next_id: u64,
    /// Pending notifications for players. The server drains these each tick.
    pub notifications: Vec<TicketNotification>,
}

impl TicketManager {
    pub fn new() -> Self {
        Self {
            tickets: Vec::new(),
            next_id: 1,
            notifications: Vec::new(),
        }
    }

    /// Drain all pending notifications (server sends these to players).
    pub fn drain_notifications(&mut self) -> Vec<TicketNotification> {
        std::mem::take(&mut self.notifications)
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
            t.player == player
                && matches!(
                    t.state,
                    TicketState::Open | TicketState::Assigned | TicketState::Escalated
                )
        });
        if has_open {
            return Err(TicketError::AlreadyHasTicket);
        }
        let id = self.next_id;
        self.next_id += 1;
        self.tickets.push(Ticket {
            id,
            player,
            category,
            description: description.to_string(),
            state: TicketState::Open,
            priority: default_priority(category),
            assigned_to: None,
            responses: Vec::new(),
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
        self.tickets
            .iter()
            .find(|t| t.player == player && !matches!(t.state, TicketState::Resolved))
    }

    /// Full ticket history for a player (all states), newest first.
    pub fn player_history(&self, player: u64) -> Vec<&Ticket> {
        let mut history: Vec<&Ticket> =
            self.tickets.iter().filter(|t| t.player == player).collect();
        history.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        history
    }

    /// Resolved tickets for a player, newest first.
    pub fn player_resolved_history(&self, player: u64) -> Vec<&Ticket> {
        let mut history: Vec<&Ticket> = self
            .tickets
            .iter()
            .filter(|t| t.player == player && t.state == TicketState::Resolved)
            .collect();
        history.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        history
    }

    /// Total ticket count for a player.
    pub fn player_ticket_count(&self, player: u64) -> usize {
        self.tickets.iter().filter(|t| t.player == player).count()
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
        self.tickets
            .iter()
            .filter(|t| t.state == TicketState::Open)
            .collect()
    }

    /// Open tickets sorted by priority (highest first), then FIFO (oldest first).
    pub fn queue(&self) -> Vec<&Ticket> {
        let mut queue: Vec<&Ticket> = self.open_tickets();
        queue.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then(a.created_at.cmp(&b.created_at))
        });
        queue
    }

    /// The next ticket a GM should handle (highest priority, oldest).
    pub fn next_in_queue(&self) -> Option<&Ticket> {
        self.queue().into_iter().next()
    }

    /// Override a ticket's priority (GM escalation/de-escalation).
    pub fn set_priority(&mut self, id: u64, priority: TicketPriority) -> Result<(), TicketError> {
        let ticket = self
            .tickets
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or(TicketError::NotFound)?;
        ticket.priority = priority;
        Ok(())
    }

    /// Count of open tickets per priority level.
    pub fn queue_counts(&self) -> (usize, usize, usize) {
        let open = self.open_tickets();
        let normal = open
            .iter()
            .filter(|t| t.priority == TicketPriority::Normal)
            .count();
        let high = open
            .iter()
            .filter(|t| t.priority == TicketPriority::High)
            .count();
        let critical = open
            .iter()
            .filter(|t| t.priority == TicketPriority::Critical)
            .count();
        (critical, high, normal)
    }

    /// GM claims a ticket. Moves it from Open → Assigned.
    pub fn assign(&mut self, id: u64, gm_name: &str) -> Result<(), TicketError> {
        let ticket = self
            .tickets
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or(TicketError::NotFound)?;
        if ticket.state != TicketState::Open {
            return Err(TicketError::NotOpen);
        }
        if ticket.assigned_to.is_some() {
            return Err(TicketError::AlreadyAssigned);
        }
        ticket.state = TicketState::Assigned;
        ticket.assigned_to = Some(gm_name.to_string());
        self.notifications.push(TicketNotification::Assigned {
            ticket_id: id,
            player: ticket.player,
            gm_name: gm_name.to_string(),
        });
        Ok(())
    }

    /// Release a ticket back to the queue. Moves Assigned → Open.
    pub fn unassign(&mut self, id: u64) -> Result<(), TicketError> {
        let ticket = self
            .tickets
            .iter_mut()
            .find(|t| t.id == id)
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
        self.tickets
            .iter()
            .filter(|t| t.assigned_to.as_deref() == Some(gm_name))
            .collect()
    }

    /// Resolve a ticket. Only Assigned or Escalated tickets can be resolved.
    pub fn resolve(&mut self, id: u64) -> Result<(), TicketError> {
        let ticket = self
            .tickets
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or(TicketError::NotFound)?;
        match ticket.state {
            TicketState::Resolved => Err(TicketError::AlreadyResolved),
            TicketState::Assigned | TicketState::Escalated => {
                ticket.state = TicketState::Resolved;
                self.notifications.push(TicketNotification::Resolved {
                    ticket_id: id,
                    player: ticket.player,
                });
                Ok(())
            }
            TicketState::Open => Err(TicketError::InvalidTransition),
        }
    }

    /// Escalate a ticket. Only Assigned tickets can be escalated.
    pub fn escalate(&mut self, id: u64) -> Result<(), TicketError> {
        let ticket = self
            .tickets
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or(TicketError::NotFound)?;
        match ticket.state {
            TicketState::Assigned => {
                ticket.state = TicketState::Escalated;
                self.notifications.push(TicketNotification::Escalated {
                    ticket_id: id,
                    player: ticket.player,
                });
                Ok(())
            }
            TicketState::Resolved => Err(TicketError::AlreadyResolved),
            _ => Err(TicketError::InvalidTransition),
        }
    }

    /// GM responds to a ticket. Adds a response and notifies the player.
    pub fn respond(
        &mut self,
        id: u64,
        gm_name: &str,
        message: &str,
        now: u64,
    ) -> Result<(), TicketError> {
        let ticket = self
            .tickets
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or(TicketError::NotFound)?;
        if ticket.state == TicketState::Resolved {
            return Err(TicketError::AlreadyResolved);
        }
        let player = ticket.player;
        ticket.responses.push(TicketResponse {
            gm_name: gm_name.to_string(),
            message: message.to_string(),
            timestamp: now,
        });
        self.notifications.push(TicketNotification::GmResponse {
            ticket_id: id,
            player,
            gm_name: gm_name.to_string(),
            message: message.to_string(),
        });
        Ok(())
    }

    /// All resolved tickets.
    pub fn resolved_tickets(&self) -> Vec<&Ticket> {
        self.tickets
            .iter()
            .filter(|t| t.state == TicketState::Resolved)
            .collect()
    }

    /// All escalated tickets.
    pub fn escalated_tickets(&self) -> Vec<&Ticket> {
        self.tickets
            .iter()
            .filter(|t| t.state == TicketState::Escalated)
            .collect()
    }
}

#[cfg(test)]
#[path = "ticket_tests.rs"]
mod tests;
