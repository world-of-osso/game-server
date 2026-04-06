//! Player-to-player trading system.
//!
//! Handles trade initiation, validation, and session management.
//! Ref: AzerothCore `TradeHandler.cpp`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Maximum distance (in yards) between players to initiate or maintain a trade.
pub const TRADE_RANGE: f32 = 11.11;

/// Why a trade operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeError {
    /// Target player is too far away.
    OutOfRange,
    /// Cannot trade with yourself.
    CannotTradeWithSelf,
    /// One of the players is already in a trade.
    AlreadyTrading,
    /// No pending trade request from this player.
    NoPendingRequest,
    /// No active trade session found.
    NoActiveSession,
    /// Target player is not online or doesn't exist.
    InvalidTarget,
    /// Player is dead and cannot trade.
    PlayerDead,
    /// Player is in combat and cannot trade.
    PlayerInCombat,
}

/// State of a trade between two players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeState {
    /// Initiator has sent a request, waiting for target to accept.
    Pending,
    /// Both players have the trade window open.
    Open,
}

/// A pending or active trade session between two players.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeSession {
    /// Player who initiated the trade request.
    pub initiator: u64,
    /// Player who received the trade request.
    pub target: u64,
    /// Current state of the trade.
    pub state: TradeState,
    /// Server timestamp when the session was created.
    pub created_at: u64,
}

impl TradeSession {
    /// Whether a player is part of this session.
    pub fn involves(&self, player: u64) -> bool {
        self.initiator == player || self.target == player
    }

    /// Get the other player in the session.
    pub fn other_player(&self, player: u64) -> Option<u64> {
        if player == self.initiator {
            Some(self.target)
        } else if player == self.target {
            Some(self.initiator)
        } else {
            None
        }
    }
}

/// Calculate squared distance between two positions.
fn distance_squared(ax: f32, ay: f32, az: f32, bx: f32, by: f32, bz: f32) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;
    let dz = az - bz;
    dx * dx + dy * dy + dz * dz
}

/// Check if two positions are within trade range.
pub fn in_trade_range(ax: f32, ay: f32, az: f32, bx: f32, by: f32, bz: f32) -> bool {
    distance_squared(ax, ay, az, bx, by, bz) <= TRADE_RANGE * TRADE_RANGE
}

/// Manages all pending and active trade sessions.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TradeManager {
    /// Active sessions indexed by a session ID.
    sessions: HashMap<u32, TradeSession>,
    /// Maps player entity → session ID for fast lookup.
    player_index: HashMap<u64, u32>,
    /// Next session ID.
    next_id: u32,
}

impl TradeManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            player_index: HashMap::new(),
            next_id: 1,
        }
    }

    /// Initiate a trade request from one player to another.
    ///
    /// Creates a pending session. The target must call `accept_trade`
    /// to open the trade window.
    pub fn initiate_trade(
        &mut self,
        initiator: u64,
        target: u64,
        pos_a: (f32, f32, f32),
        pos_b: (f32, f32, f32),
        now: u64,
    ) -> Result<u32, TradeError> {
        self.validate_initiation(initiator, target, pos_a, pos_b)?;
        let session_id = self.insert_session(TradeSession {
            initiator, target, state: TradeState::Pending, created_at: now,
        });
        Ok(session_id)
    }

    fn validate_initiation(
        &self, initiator: u64, target: u64,
        pos_a: (f32, f32, f32), pos_b: (f32, f32, f32),
    ) -> Result<(), TradeError> {
        if initiator == target {
            return Err(TradeError::CannotTradeWithSelf);
        }
        if self.player_index.contains_key(&initiator)
            || self.player_index.contains_key(&target) {
            return Err(TradeError::AlreadyTrading);
        }
        if !in_trade_range(pos_a.0, pos_a.1, pos_a.2, pos_b.0, pos_b.1, pos_b.2) {
            return Err(TradeError::OutOfRange);
        }
        Ok(())
    }

    fn insert_session(&mut self, session: TradeSession) -> u32 {
        let session_id = self.next_id;
        self.next_id += 1;
        self.player_index.insert(session.initiator, session_id);
        self.player_index.insert(session.target, session_id);
        self.sessions.insert(session_id, session);
        session_id
    }

    /// Accept a pending trade request, opening the trade window.
    pub fn accept_trade(&mut self, target: u64) -> Result<u32, TradeError> {
        let &session_id = self.player_index.get(&target)
            .ok_or(TradeError::NoPendingRequest)?;
        let session = self.sessions.get_mut(&session_id)
            .ok_or(TradeError::NoPendingRequest)?;
        if session.target != target || session.state != TradeState::Pending {
            return Err(TradeError::NoPendingRequest);
        }
        session.state = TradeState::Open;
        Ok(session_id)
    }

    /// Decline a pending trade request.
    pub fn decline_trade(&mut self, target: u64) -> Result<(), TradeError> {
        let &session_id = self.player_index.get(&target)
            .ok_or(TradeError::NoPendingRequest)?;
        let session = self.sessions.get(&session_id)
            .ok_or(TradeError::NoPendingRequest)?;
        if session.target != target || session.state != TradeState::Pending {
            return Err(TradeError::NoPendingRequest);
        }
        self.remove_session(session_id);
        Ok(())
    }

    /// Cancel an active or pending trade (either player can cancel).
    pub fn cancel_trade(&mut self, player: u64) -> Result<u64, TradeError> {
        let &session_id = self.player_index.get(&player)
            .ok_or(TradeError::NoActiveSession)?;
        let session = self.sessions.get(&session_id)
            .ok_or(TradeError::NoActiveSession)?;
        let other = session.other_player(player)
            .ok_or(TradeError::NoActiveSession)?;
        self.remove_session(session_id);
        Ok(other)
    }

    /// Get the active trade session for a player.
    pub fn get_session(&self, player: u64) -> Option<&TradeSession> {
        let &session_id = self.player_index.get(&player)?;
        self.sessions.get(&session_id)
    }

    /// Get the session ID for a player, if any.
    pub fn get_session_id(&self, player: u64) -> Option<u32> {
        self.player_index.get(&player).copied()
    }

    /// Whether a player is currently in a trade (pending or open).
    pub fn is_trading(&self, player: u64) -> bool {
        self.player_index.contains_key(&player)
    }

    /// Remove expired pending requests (e.g. after a timeout).
    pub fn cleanup_expired(&mut self, now: u64, timeout_secs: u64) {
        let expired: Vec<u32> = self.sessions
            .iter()
            .filter(|(_, s)| s.state == TradeState::Pending && now.saturating_sub(s.created_at) > timeout_secs)
            .map(|(&id, _)| id)
            .collect();
        for id in expired {
            self.remove_session(id);
        }
    }

    /// Number of active sessions.
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Whether there are no active sessions.
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    fn remove_session(&mut self, session_id: u32) {
        if let Some(session) = self.sessions.remove(&session_id) {
            self.player_index.remove(&session.initiator);
            self.player_index.remove(&session.target);
        }
    }
}

#[cfg(test)]
#[path = "trade_tests.rs"]
mod tests;
