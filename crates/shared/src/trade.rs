//! Player-to-player trading system.
//!
//! Handles trade initiation, validation, and session management.
//! Ref: AzerothCore `TradeHandler.cpp`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::item_data::BoundState;

/// Maximum distance (in yards) between players to initiate or maintain a trade.
pub const TRADE_RANGE: f32 = 11.11;

/// Maximum number of item slots per player in a trade window.
pub const TRADE_SLOT_COUNT: usize = 6;

/// Duration (seconds) after group loot pickup during which a BoP item
/// can be traded to eligible group members who were present for the kill.
pub const BOP_TRADE_WINDOW_SECS: u64 = 7200; // 2 hours

/// Tracks the group-loot trade window for a BoP item.
///
/// When a BoP item is looted via group loot, the winner can trade it
/// to other eligible group members within 2 hours.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BopTradeWindow {
    /// Timestamp when the item was looted.
    pub looted_at: u64,
    /// Entity IDs of group members eligible to receive this item.
    pub eligible_players: Vec<u64>,
}

impl BopTradeWindow {
    /// Whether the trade window has expired.
    pub fn is_expired(&self, now: u64) -> bool {
        now.saturating_sub(self.looted_at) >= BOP_TRADE_WINDOW_SECS
    }

    /// Whether a player is eligible to receive this item.
    pub fn is_eligible(&self, player: u64, now: u64) -> bool {
        !self.is_expired(now) && self.eligible_players.contains(&player)
    }

    /// Seconds remaining in the trade window (0 if expired).
    pub fn remaining(&self, now: u64) -> u64 {
        (self.looted_at + BOP_TRADE_WINDOW_SECS).saturating_sub(now)
    }
}

/// Check whether an item can be placed in a trade offer.
///
/// - Unbound items: always tradeable.
/// - Bound items: only if a `BopTradeWindow` is active and the trade
///   target is in the eligible list.
pub fn validate_trade_binding(
    bound_state: &BoundState,
    bop_window: Option<&BopTradeWindow>,
    trade_target: u64,
    now: u64,
) -> Result<(), TradeError> {
    if bound_state.is_tradeable() {
        return Ok(());
    }
    // Item is bound — check for group loot trade window
    let window = bop_window.ok_or(TradeError::ItemSoulbound)?;
    if window.is_eligible(trade_target, now) {
        Ok(())
    } else {
        Err(TradeError::ItemSoulbound)
    }
}

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
    /// Trade is not in Open state (still pending or already completed).
    TradeNotOpen,
    /// Slot index is out of range (must be 0–5).
    InvalidSlot,
    /// Item is already in another slot of the same offer.
    ItemAlreadyOffered,
    /// Player has not confirmed / already unconfirmed.
    NotConfirmed,
    /// Item is soulbound and cannot be traded.
    ItemSoulbound,
}

/// State of a trade between two players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeState {
    /// Initiator has sent a request, waiting for target to accept.
    Pending,
    /// Both players have the trade window open.
    Open,
}

/// An item placed in a trade slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeItem {
    /// Unique inventory item GUID.
    pub item_guid: u64,
    /// Stack count being offered.
    pub count: u16,
}

/// One player's offer in a trade: up to 6 items + gold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeOffer {
    /// Item slots (6 total). `None` = empty slot.
    pub slots: [Option<TradeItem>; TRADE_SLOT_COUNT],
    /// Gold offered (in copper).
    pub gold: u32,
}

impl Default for TradeOffer {
    fn default() -> Self {
        Self {
            slots: [None; TRADE_SLOT_COUNT],
            gold: 0,
        }
    }
}

impl TradeOffer {
    /// Set an item in a slot. Returns error if slot index is invalid
    /// or the item is already in another slot.
    pub fn set_item(&mut self, slot: usize, item: TradeItem) -> Result<(), TradeError> {
        if slot >= TRADE_SLOT_COUNT {
            return Err(TradeError::InvalidSlot);
        }
        let already_offered = self
            .slots
            .iter()
            .enumerate()
            .any(|(i, s)| i != slot && s.is_some_and(|s| s.item_guid == item.item_guid));
        if already_offered {
            return Err(TradeError::ItemAlreadyOffered);
        }
        self.slots[slot] = Some(item);
        Ok(())
    }

    /// Clear a slot.
    pub fn clear_slot(&mut self, slot: usize) -> Result<(), TradeError> {
        if slot >= TRADE_SLOT_COUNT {
            return Err(TradeError::InvalidSlot);
        }
        self.slots[slot] = None;
        Ok(())
    }

    /// Set the gold amount offered.
    pub fn set_gold(&mut self, copper: u32) {
        self.gold = copper;
    }

    /// Number of items offered (non-empty slots).
    pub fn item_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// Whether the offer is empty (no items, no gold).
    pub fn is_empty(&self) -> bool {
        self.gold == 0 && self.slots.iter().all(|s| s.is_none())
    }
}

/// The result of a completed trade — both offers ready for the server to execute.
#[derive(Debug, Clone, PartialEq)]
pub struct CompletedTrade {
    pub initiator: u64,
    pub target: u64,
    pub initiator_offer: TradeOffer,
    pub target_offer: TradeOffer,
}

// --- Trade logging (dispute resolution) ---

/// A permanent record of a completed trade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeLogEntry {
    /// Sequential log ID.
    pub id: u64,
    /// Player who initiated the trade.
    pub initiator: u64,
    /// Player who accepted the trade request.
    pub target: u64,
    /// What the initiator gave.
    pub initiator_offer: TradeOffer,
    /// What the target gave.
    pub target_offer: TradeOffer,
    /// Server timestamp when the trade completed.
    pub completed_at: u64,
}

/// Append-only trade log for server-side dispute resolution.
///
/// Records every completed trade with full item/gold details.
/// The server persists this to disk; this struct is the in-memory buffer.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TradeLog {
    entries: Vec<TradeLogEntry>,
    next_id: u64,
}

impl TradeLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }

    /// Record a completed trade. Returns the log entry ID.
    pub fn record(&mut self, trade: &CompletedTrade, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(TradeLogEntry {
            id,
            initiator: trade.initiator,
            target: trade.target,
            initiator_offer: trade.initiator_offer.clone(),
            target_offer: trade.target_offer.clone(),
            completed_at: now,
        });
        id
    }

    /// Find all trades involving a specific player.
    pub fn trades_for_player(&self, player: u64) -> Vec<&TradeLogEntry> {
        self.entries
            .iter()
            .filter(|e| e.initiator == player || e.target == player)
            .collect()
    }

    /// Find all trades within a time range.
    pub fn trades_in_range(&self, from: u64, to: u64) -> Vec<&TradeLogEntry> {
        self.entries
            .iter()
            .filter(|e| e.completed_at >= from && e.completed_at <= to)
            .collect()
    }

    /// Look up a specific trade by log ID.
    pub fn get(&self, id: u64) -> Option<&TradeLogEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Total number of recorded trades.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drain entries older than `before` timestamp for archival.
    /// Returns the removed entries so the server can persist them.
    pub fn drain_before(&mut self, before: u64) -> Vec<TradeLogEntry> {
        let (old, recent): (Vec<_>, Vec<_>) = self
            .entries
            .drain(..)
            .partition(|e| e.completed_at < before);
        self.entries = recent;
        old
    }
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
    /// Initiator's offered items and gold.
    pub initiator_offer: TradeOffer,
    /// Target's offered items and gold.
    pub target_offer: TradeOffer,
    /// Whether the initiator has clicked "Accept".
    pub initiator_accepted: bool,
    /// Whether the target has clicked "Accept".
    pub target_accepted: bool,
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

    /// Get a player's offer (read-only).
    pub fn offer(&self, player: u64) -> Option<&TradeOffer> {
        if player == self.initiator {
            Some(&self.initiator_offer)
        } else if player == self.target {
            Some(&self.target_offer)
        } else {
            None
        }
    }

    /// Get a mutable reference to a player's offer.
    pub fn offer_mut(&mut self, player: u64) -> Option<&mut TradeOffer> {
        if player == self.initiator {
            Some(&mut self.initiator_offer)
        } else if player == self.target {
            Some(&mut self.target_offer)
        } else {
            None
        }
    }

    /// Whether a player has accepted the current offer.
    pub fn has_accepted(&self, player: u64) -> bool {
        if player == self.initiator {
            self.initiator_accepted
        } else if player == self.target {
            self.target_accepted
        } else {
            false
        }
    }

    /// Whether both players have accepted.
    pub fn both_accepted(&self) -> bool {
        self.initiator_accepted && self.target_accepted
    }

    /// Reset both accept flags (called when either offer changes).
    pub fn reset_accepts(&mut self) {
        self.initiator_accepted = false;
        self.target_accepted = false;
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
            initiator,
            target,
            state: TradeState::Pending,
            created_at: now,
            initiator_offer: TradeOffer::default(),
            target_offer: TradeOffer::default(),
            initiator_accepted: false,
            target_accepted: false,
        });
        Ok(session_id)
    }

    fn validate_initiation(
        &self,
        initiator: u64,
        target: u64,
        pos_a: (f32, f32, f32),
        pos_b: (f32, f32, f32),
    ) -> Result<(), TradeError> {
        if initiator == target {
            return Err(TradeError::CannotTradeWithSelf);
        }
        if self.player_index.contains_key(&initiator) || self.player_index.contains_key(&target) {
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
        let &session_id = self
            .player_index
            .get(&target)
            .ok_or(TradeError::NoPendingRequest)?;
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or(TradeError::NoPendingRequest)?;
        if session.target != target || session.state != TradeState::Pending {
            return Err(TradeError::NoPendingRequest);
        }
        session.state = TradeState::Open;
        Ok(session_id)
    }

    /// Decline a pending trade request.
    pub fn decline_trade(&mut self, target: u64) -> Result<(), TradeError> {
        let &session_id = self
            .player_index
            .get(&target)
            .ok_or(TradeError::NoPendingRequest)?;
        let session = self
            .sessions
            .get(&session_id)
            .ok_or(TradeError::NoPendingRequest)?;
        if session.target != target || session.state != TradeState::Pending {
            return Err(TradeError::NoPendingRequest);
        }
        self.remove_session(session_id);
        Ok(())
    }

    /// Cancel an active or pending trade (either player can cancel).
    pub fn cancel_trade(&mut self, player: u64) -> Result<u64, TradeError> {
        let &session_id = self
            .player_index
            .get(&player)
            .ok_or(TradeError::NoActiveSession)?;
        let session = self
            .sessions
            .get(&session_id)
            .ok_or(TradeError::NoActiveSession)?;
        let other = session
            .other_player(player)
            .ok_or(TradeError::NoActiveSession)?;
        self.remove_session(session_id);
        Ok(other)
    }

    /// Set an item in a player's trade slot. Trade must be Open.
    /// Resets both players' accept flags (offer changed).
    pub fn set_item(
        &mut self,
        player: u64,
        slot: usize,
        item: TradeItem,
    ) -> Result<(), TradeError> {
        let session = self.require_open_session(player)?;
        session.reset_accepts();
        let offer = session
            .offer_mut(player)
            .ok_or(TradeError::NoActiveSession)?;
        offer.set_item(slot, item)
    }

    /// Clear an item slot in a player's trade offer.
    /// Resets both players' accept flags.
    pub fn clear_item(&mut self, player: u64, slot: usize) -> Result<(), TradeError> {
        let session = self.require_open_session(player)?;
        session.reset_accepts();
        let offer = session
            .offer_mut(player)
            .ok_or(TradeError::NoActiveSession)?;
        offer.clear_slot(slot)
    }

    /// Set the gold amount in a player's trade offer.
    /// Resets both players' accept flags.
    pub fn set_gold(&mut self, player: u64, copper: u32) -> Result<(), TradeError> {
        let session = self.require_open_session(player)?;
        session.reset_accepts();
        let offer = session
            .offer_mut(player)
            .ok_or(TradeError::NoActiveSession)?;
        offer.set_gold(copper);
        Ok(())
    }

    /// Confirm (accept) the trade. Both players must confirm for it to complete.
    ///
    /// Returns `Some(CompletedTrade)` if both players have now accepted,
    /// which means the server should execute the item/gold exchange.
    /// The session is removed on completion.
    pub fn confirm_trade(&mut self, player: u64) -> Result<Option<CompletedTrade>, TradeError> {
        let session = self.require_open_session(player)?;
        if player == session.initiator {
            session.initiator_accepted = true;
        } else {
            session.target_accepted = true;
        }

        if !session.both_accepted() {
            return Ok(None);
        }

        // Both accepted — complete the trade
        let session_id = self.player_index[&player];
        let session = self.sessions.remove(&session_id).unwrap();
        self.player_index.remove(&session.initiator);
        self.player_index.remove(&session.target);
        Ok(Some(CompletedTrade {
            initiator: session.initiator,
            target: session.target,
            initiator_offer: session.initiator_offer,
            target_offer: session.target_offer,
        }))
    }

    /// Withdraw acceptance (unconfirm). Only valid if the player has accepted.
    pub fn unconfirm_trade(&mut self, player: u64) -> Result<(), TradeError> {
        let session = self.require_open_session(player)?;
        let accepted = session.has_accepted(player);
        if !accepted {
            return Err(TradeError::NotConfirmed);
        }
        if player == session.initiator {
            session.initiator_accepted = false;
        } else {
            session.target_accepted = false;
        }
        Ok(())
    }

    fn require_open_session(&mut self, player: u64) -> Result<&mut TradeSession, TradeError> {
        let session = self
            .get_session_mut(player)
            .ok_or(TradeError::NoActiveSession)?;
        if session.state != TradeState::Open {
            return Err(TradeError::TradeNotOpen);
        }
        Ok(session)
    }

    /// Get the active trade session for a player.
    pub fn get_session(&self, player: u64) -> Option<&TradeSession> {
        let &session_id = self.player_index.get(&player)?;
        self.sessions.get(&session_id)
    }

    /// Get a mutable reference to a session by player.
    fn get_session_mut(&mut self, player: u64) -> Option<&mut TradeSession> {
        let &session_id = self.player_index.get(&player)?;
        self.sessions.get_mut(&session_id)
    }

    /// Get the session ID for a player, if any.
    pub fn get_session_id(&self, player: u64) -> Option<u32> {
        self.player_index.get(&player).copied()
    }

    /// Whether a player is currently in a trade (pending or open).
    pub fn is_trading(&self, player: u64) -> bool {
        self.player_index.contains_key(&player)
    }

    /// Check if a specific trade session's players are still in range.
    ///
    /// Returns `Err(OutOfRange)` if they've moved apart. The server
    /// should cancel the trade when this fails.
    pub fn check_range(
        &self,
        player: u64,
        pos_a: (f32, f32, f32),
        pos_b: (f32, f32, f32),
    ) -> Result<(), TradeError> {
        if !self.is_trading(player) {
            return Err(TradeError::NoActiveSession);
        }
        if in_trade_range(pos_a.0, pos_a.1, pos_a.2, pos_b.0, pos_b.1, pos_b.2) {
            Ok(())
        } else {
            Err(TradeError::OutOfRange)
        }
    }

    /// Enforce range on all active sessions. Cancels any where players
    /// have moved out of range.
    ///
    /// `get_pos` resolves a player entity to their current position.
    /// Returns cancelled session pairs `(initiator, target)` so the
    /// server can notify both players.
    pub fn enforce_range(
        &mut self,
        get_pos: impl Fn(u64) -> Option<(f32, f32, f32)>,
    ) -> Vec<(u64, u64)> {
        let out_of_range: Vec<u32> = self
            .sessions
            .iter()
            .filter(|(_, s)| {
                let Some(a) = get_pos(s.initiator) else {
                    return true;
                };
                let Some(b) = get_pos(s.target) else {
                    return true;
                };
                !in_trade_range(a.0, a.1, a.2, b.0, b.1, b.2)
            })
            .map(|(&id, _)| id)
            .collect();

        out_of_range
            .iter()
            .filter_map(|&id| {
                let s = self.sessions.get(&id)?;
                let pair = (s.initiator, s.target);
                self.remove_session(id);
                Some(pair)
            })
            .collect()
    }

    /// Remove expired pending requests (e.g. after a timeout).
    pub fn cleanup_expired(&mut self, now: u64, timeout_secs: u64) {
        let expired: Vec<u32> = self
            .sessions
            .iter()
            .filter(|(_, s)| {
                s.state == TradeState::Pending && now.saturating_sub(s.created_at) > timeout_secs
            })
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
