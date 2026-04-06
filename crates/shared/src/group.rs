//! Group and raid system: party management, roles, loot rules.
//!
//! Ref: AzerothCore `Group.cpp`.

use serde::{Deserialize, Serialize};

/// Maximum party size.
pub const MAX_PARTY_SIZE: usize = 5;
/// Maximum raid size.
pub const MAX_RAID_SIZE: usize = 40;
/// Number of subgroups in a raid.
pub const RAID_SUBGROUPS: usize = 8;
/// Members per subgroup.
pub const SUBGROUP_SIZE: usize = 5;

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

// --- Raid ---

/// A raid of up to 40 players in 8 subgroups of 5.
///
/// Raids are formed by converting a party to a raid. Members are assigned
/// to subgroups (0–7). Subgroup 0 is the "main tank" group by convention.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Raid {
    pub leader: u64,
    /// 8 subgroups, each holding up to 5 member entity bits.
    pub subgroups: [Vec<u64>; RAID_SUBGROUPS],
}

impl Raid {
    /// Convert a party into a raid. All party members go into subgroup 0.
    pub fn from_party(party: &Party) -> Self {
        let mut subgroups: [Vec<u64>; RAID_SUBGROUPS] = Default::default();
        subgroups[0] = party.members.clone();
        Self {
            leader: party.leader,
            subgroups,
        }
    }

    /// Add a member to the first subgroup with space.
    pub fn add_member(&mut self, player: u64) -> Result<usize, PartyError> {
        if self.total_members() >= MAX_RAID_SIZE {
            return Err(PartyError::Full);
        }
        if self.contains(player) {
            return Err(PartyError::AlreadyMember);
        }
        let group = self.first_open_subgroup().ok_or(PartyError::Full)?;
        self.subgroups[group].push(player);
        Ok(group)
    }

    /// Move a member to a different subgroup.
    pub fn move_to_subgroup(&mut self, player: u64, target_group: usize) -> Result<(), PartyError> {
        if target_group >= RAID_SUBGROUPS {
            return Err(PartyError::Full);
        }
        if self.subgroups[target_group].len() >= SUBGROUP_SIZE {
            return Err(PartyError::Full);
        }
        // Remove from current subgroup
        for group in &mut self.subgroups {
            group.retain(|&m| m != player);
        }
        self.subgroups[target_group].push(player);
        Ok(())
    }

    /// Remove a member from the raid. Returns `true` if raid should disband.
    pub fn leave(&mut self, player: u64) -> bool {
        for group in &mut self.subgroups {
            group.retain(|&m| m != player);
        }
        if player == self.leader {
            let next = self.subgroups.iter().flat_map(|g| g.iter()).next().copied();
            self.leader = next.unwrap_or(0);
        }
        self.total_members() <= 1
    }

    /// Total number of members across all subgroups.
    pub fn total_members(&self) -> usize {
        self.subgroups.iter().map(|g| g.len()).sum()
    }

    /// Whether a player is in the raid.
    pub fn contains(&self, player: u64) -> bool {
        self.subgroups.iter().any(|g| g.contains(&player))
    }

    /// Which subgroup a player is in (0–7), or None.
    pub fn subgroup_of(&self, player: u64) -> Option<usize> {
        self.subgroups.iter().position(|g| g.contains(&player))
    }

    /// Iterator over all members.
    pub fn all_members(&self) -> impl Iterator<Item = u64> + '_ {
        self.subgroups.iter().flat_map(|g| g.iter().copied())
    }

    fn first_open_subgroup(&self) -> Option<usize> {
        self.subgroups.iter().position(|g| g.len() < SUBGROUP_SIZE)
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

    // --- Raid tests ---

    #[test]
    fn raid_from_party() {
        let mut party = Party::new(1);
        party.accept(2).unwrap();
        party.accept(3).unwrap();
        let raid = Raid::from_party(&party);
        assert_eq!(raid.leader, 1);
        assert_eq!(raid.total_members(), 3);
        assert_eq!(raid.subgroup_of(1), Some(0));
        assert_eq!(raid.subgroup_of(2), Some(0));
    }

    #[test]
    fn raid_add_fills_subgroups() {
        let party = Party::new(1);
        let mut raid = Raid::from_party(&party);
        // Fill subgroup 0 (already has 1 member)
        for i in 2..=5 {
            raid.add_member(i).unwrap();
        }
        assert_eq!(raid.subgroups[0].len(), 5);
        // Next member goes to subgroup 1
        let group = raid.add_member(6).unwrap();
        assert_eq!(group, 1);
        assert_eq!(raid.subgroup_of(6), Some(1));
    }

    #[test]
    fn raid_full_at_40() {
        let party = Party::new(1);
        let mut raid = Raid::from_party(&party);
        for i in 2..=40 {
            raid.add_member(i).unwrap();
        }
        assert_eq!(raid.total_members(), 40);
        assert_eq!(raid.add_member(41), Err(PartyError::Full));
    }

    #[test]
    fn raid_move_subgroup() {
        let party = Party::new(1);
        let mut raid = Raid::from_party(&party);
        raid.add_member(2).unwrap();
        raid.move_to_subgroup(2, 3).unwrap();
        assert_eq!(raid.subgroup_of(2), Some(3));
        assert_eq!(raid.subgroups[0].len(), 1);
    }

    #[test]
    fn raid_move_to_full_subgroup_fails() {
        let party = Party::new(1);
        let mut raid = Raid::from_party(&party);
        for i in 2..=5 {
            raid.add_member(i).unwrap();
        }
        raid.add_member(6).unwrap(); // goes to group 1
        assert_eq!(raid.move_to_subgroup(6, 0), Err(PartyError::Full));
    }

    #[test]
    fn raid_leave_promotes_leader() {
        let mut party = Party::new(1);
        party.accept(2).unwrap();
        let mut raid = Raid::from_party(&party);
        raid.leave(1);
        assert_eq!(raid.leader, 2);
        assert!(!raid.contains(1));
    }

    #[test]
    fn raid_reject_duplicate() {
        let party = Party::new(1);
        let mut raid = Raid::from_party(&party);
        assert_eq!(raid.add_member(1), Err(PartyError::AlreadyMember));
    }
}
