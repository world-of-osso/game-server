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

// --- Shared threat ---

use crate::components::ThreatTable;

/// Apply damage threat from a group member to a mob's threat table.
///
/// The group member's threat is attributed directly (1:1 ratio × modifier).
/// All group members share the same threat table on the mob.
pub fn apply_group_damage_threat(
    table: &mut ThreatTable,
    source: u64,
    damage: f32,
    modifier: f32,
) {
    table.add_damage_threat(source, damage, modifier);
}

/// Apply healing threat from a group member, split across all engaged mobs.
///
/// Healing generates 0.5 threat per point healed, split equally among
/// all mobs the group is fighting. Each mob's threat table gets
/// `heal * 0.5 * modifier / num_mobs`.
///
/// Ref: AzerothCore ThreatMgr — healing threat is divided by engaged mob count.
pub fn apply_group_heal_threat(
    tables: &mut [&mut ThreatTable],
    source: u64,
    heal_amount: f32,
    modifier: f32,
) {
    if tables.is_empty() {
        return;
    }
    let per_mob = heal_amount / tables.len() as f32;
    for table in tables {
        table.add_heal_threat(source, per_mob, modifier);
    }
}

/// Collect all unique mob entity IDs that any group member has threat on.
///
/// Used to determine which mobs to split healing threat across.
/// `mob_threats` is a list of (mob_entity, threat_table) pairs.
/// `group_members` is the list of player entity bits in the group.
pub fn engaged_mobs(
    mob_threats: &[(u64, &ThreatTable)],
    group_members: &[u64],
) -> Vec<u64> {
    mob_threats
        .iter()
        .filter(|(_, table)| {
            group_members
                .iter()
                .any(|&member| table.threat_for(member) > 0.0)
        })
        .map(|(entity, _)| *entity)
        .collect()
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

    // --- Shared threat tests ---

    #[test]
    fn group_damage_threat_shared_table() {
        let mut table = ThreatTable::default();
        // Two party members both damage the same mob
        apply_group_damage_threat(&mut table, 1, 500.0, 1.0);
        apply_group_damage_threat(&mut table, 2, 300.0, 1.0);
        assert_eq!(table.threat_for(1), 500.0);
        assert_eq!(table.threat_for(2), 300.0);
        assert_eq!(table.top_threat().unwrap().entity, 1);
    }

    #[test]
    fn group_heal_threat_split_across_mobs() {
        let mut table1 = ThreatTable::default();
        let mut table2 = ThreatTable::default();
        // Healer heals 1000, split across 2 mobs
        apply_group_heal_threat(
            &mut [&mut table1, &mut table2],
            3, 1000.0, 1.0,
        );
        // Each mob gets 500 heal → 500 * 0.5 = 250 threat
        assert!((table1.threat_for(3) - 250.0).abs() < 0.01);
        assert!((table2.threat_for(3) - 250.0).abs() < 0.01);
    }

    #[test]
    fn group_heal_no_mobs_no_crash() {
        apply_group_heal_threat(&mut [], 1, 1000.0, 1.0);
    }

    #[test]
    fn engaged_mobs_filters_by_group() {
        let mut t1 = ThreatTable::default();
        t1.add_damage_threat(1, 100.0, 1.0); // player 1 in group

        let mut t2 = ThreatTable::default();
        t2.add_damage_threat(99, 100.0, 1.0); // player 99 NOT in group

        let mobs: Vec<(u64, &ThreatTable)> = vec![(10, &t1), (20, &t2)];
        let group_members = vec![1, 2, 3];
        let engaged = engaged_mobs(&mobs, &group_members);
        assert_eq!(engaged, vec![10]); // only mob 10 is engaged by the group
    }

    #[test]
    fn heal_threat_with_tank_modifier() {
        let mut table = ThreatTable::default();
        // Tank heals with 1.43x threat modifier
        apply_group_heal_threat(&mut [&mut table], 1, 1000.0, 1.43);
        // 1000 * 0.5 * 1.43 = 715
        assert!((table.threat_for(1) - 715.0).abs() < 0.01);
    }
}
