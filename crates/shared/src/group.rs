//! Group and raid system: party management, roles, loot rules.
//!
//! Ref: AzerothCore `Group.cpp`.

use serde::{Deserialize, Serialize};

/// Maximum party size.
pub const MAX_PARTY_SIZE: usize = 5;

/// A pending party invite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartyInvite {
    pub inviter: u64,
    pub invitee: u64,
}

/// Why a party operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyError {
    /// Party is full (5 members).
    Full,
    /// Player is already in this party.
    AlreadyMember,
    /// Player is already in another group.
    AlreadyInGroup,
    /// No pending invite to accept.
    NoInvite,
    /// Only the leader can do this.
    NotLeader,
    /// Target player not found.
    PlayerNotFound,
}

/// A party of up to 5 players.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Party {
    /// Entity bits of the party leader.
    pub leader: u64,
    /// All members including the leader.
    pub members: Vec<u64>,
}

impl Party {
    /// Create a new party with a single member (the leader).
    pub fn new(leader: u64) -> Self {
        Self {
            leader,
            members: vec![leader],
        }
    }

    /// Invite a player. Returns `Err` if party is full or player already in it.
    pub fn invite(&self, invitee: u64) -> Result<PartyInvite, PartyError> {
        if self.members.len() >= MAX_PARTY_SIZE {
            return Err(PartyError::Full);
        }
        if self.members.contains(&invitee) {
            return Err(PartyError::AlreadyMember);
        }
        Ok(PartyInvite {
            inviter: self.leader,
            invitee,
        })
    }

    /// Accept an invite, adding the player to the party.
    pub fn accept(&mut self, player: u64) -> Result<(), PartyError> {
        if self.members.len() >= MAX_PARTY_SIZE {
            return Err(PartyError::Full);
        }
        if self.members.contains(&player) {
            return Err(PartyError::AlreadyMember);
        }
        self.members.push(player);
        Ok(())
    }

    /// Remove a player from the party.
    ///
    /// If the leader leaves, the next member becomes leader.
    /// Returns `true` if the party should be disbanded (0-1 members left).
    pub fn leave(&mut self, player: u64) -> bool {
        self.members.retain(|&m| m != player);
        if player == self.leader {
            self.leader = self.members.first().copied().unwrap_or(0);
        }
        self.members.len() <= 1
    }

    /// Disband the party (only leader can).
    pub fn disband(&mut self, requester: u64) -> Result<(), PartyError> {
        if requester != self.leader {
            return Err(PartyError::NotLeader);
        }
        self.members.clear();
        Ok(())
    }

    /// Number of members.
    pub fn size(&self) -> usize {
        self.members.len()
    }

    /// Whether a player is in this party.
    pub fn contains(&self, player: u64) -> bool {
        self.members.contains(&player)
    }

    /// Whether the party is full.
    pub fn is_full(&self) -> bool {
        self.members.len() >= MAX_PARTY_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn party_create() {
        let party = Party::new(1);
        assert_eq!(party.leader, 1);
        assert_eq!(party.size(), 1);
        assert!(party.contains(1));
    }

    #[test]
    fn party_invite_and_accept() {
        let mut party = Party::new(1);
        let invite = party.invite(2).unwrap();
        assert_eq!(invite.invitee, 2);

        party.accept(2).unwrap();
        assert_eq!(party.size(), 2);
        assert!(party.contains(2));
    }

    #[test]
    fn party_reject_duplicate() {
        let party = Party::new(1);
        assert_eq!(party.invite(1), Err(PartyError::AlreadyMember));
    }

    #[test]
    fn party_full_at_5() {
        let mut party = Party::new(1);
        for i in 2..=5 {
            party.accept(i).unwrap();
        }
        assert!(party.is_full());
        assert_eq!(party.invite(6), Err(PartyError::Full));
        assert_eq!(party.accept(6), Err(PartyError::Full));
    }

    #[test]
    fn party_leave_promotes_leader() {
        let mut party = Party::new(1);
        party.accept(2).unwrap();
        party.accept(3).unwrap();

        let disband = party.leave(1); // leader leaves
        assert!(!disband);
        assert_eq!(party.leader, 2); // next member promoted
        assert!(!party.contains(1));
    }

    #[test]
    fn party_leave_last_disbands() {
        let mut party = Party::new(1);
        party.accept(2).unwrap();
        party.leave(1);
        let disband = party.leave(2);
        assert!(disband); // 0 members
    }

    #[test]
    fn party_disband_only_leader() {
        let mut party = Party::new(1);
        party.accept(2).unwrap();
        assert_eq!(party.disband(2), Err(PartyError::NotLeader));
        assert!(party.disband(1).is_ok());
        assert_eq!(party.size(), 0);
    }

    #[test]
    fn party_leave_non_leader_keeps_leader() {
        let mut party = Party::new(1);
        party.accept(2).unwrap();
        party.accept(3).unwrap();
        party.leave(2);
        assert_eq!(party.leader, 1);
        assert_eq!(party.size(), 2);
    }
}
