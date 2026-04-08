//! Group and raid system: party management, roles, loot rules.
//!
//! Ref: AzerothCore `Group.cpp`.

use serde::{Deserialize, Serialize};

use crate::loot::LootMode;

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
    /// Group loot distribution mode.
    pub loot_mode: LootMode,
    /// Round-robin loot rotation index.
    pub loot_round_robin: usize,
}

impl Party {
    /// Create a new party with a single member (the leader).
    pub fn new(leader: u64) -> Self {
        Self {
            leader,
            members: vec![leader],
            loot_mode: LootMode::PersonalLoot,
            loot_round_robin: 0,
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
            // bounded: max 5 members
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

    /// Set the loot mode (only leader should call this).
    pub fn set_loot_mode(&mut self, requester: u64, mode: LootMode) -> Result<(), PartyError> {
        if requester != self.leader {
            return Err(PartyError::NotLeader);
        }
        self.loot_mode = mode;
        Ok(())
    }

    /// Get the next round-robin loot recipient and advance the rotation.
    pub fn next_round_robin(&mut self) -> u64 {
        if self.members.is_empty() {
            return 0;
        }
        let member = self.members[self.loot_round_robin % self.members.len()];
        self.loot_round_robin = (self.loot_round_robin + 1) % self.members.len();
        member
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
    /// Group loot distribution mode.
    pub loot_mode: LootMode,
}

impl Raid {
    /// Convert a party into a raid. All party members go into subgroup 0.
    pub fn from_party(party: &Party) -> Self {
        let mut subgroups: [Vec<u64>; RAID_SUBGROUPS] = Default::default();
        subgroups[0] = party.members.clone();
        Self {
            leader: party.leader,
            subgroups,
            loot_mode: party.loot_mode,
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

// --- Role assignment ---

/// Group role for dungeon/raid finder and UI display.
///
/// Separate from `components::Role` (which is per-spec). This is the
/// player's chosen role for the current group activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupRole {
    Tank,
    Healer,
    Dps,
}

/// Per-member role assignments for a group.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RoleAssignments {
    entries: Vec<(u64, GroupRole)>,
}

impl RoleAssignments {
    /// Assign a role to a member. Overwrites any previous role.
    pub fn assign(&mut self, player: u64, role: GroupRole) {
        if let Some(entry) = self.entries.iter_mut().find(|(m, _)| *m == player) {
            entry.1 = role;
        } else {
            self.entries.push((player, role));
        }
    }

    /// Get a member's assigned role.
    pub fn role_of(&self, player: u64) -> Option<GroupRole> {
        self.entries
            .iter()
            .find(|(m, _)| *m == player)
            .map(|(_, r)| *r)
    }

    /// Remove a member's role (e.g. when they leave the group).
    pub fn remove(&mut self, player: u64) {
        self.entries.retain(|(m, _)| *m != player);
    }

    /// Count of each role.
    pub fn counts(&self) -> (usize, usize, usize) {
        let tanks = self
            .entries
            .iter()
            .filter(|(_, r)| *r == GroupRole::Tank)
            .count();
        let healers = self
            .entries
            .iter()
            .filter(|(_, r)| *r == GroupRole::Healer)
            .count();
        let dps = self
            .entries
            .iter()
            .filter(|(_, r)| *r == GroupRole::Dps)
            .count();
        (tanks, healers, dps)
    }

    /// Whether the group has the standard 1T/1H/3D composition.
    pub fn has_standard_composition(&self) -> bool {
        let (tanks, healers, _) = self.counts();
        tanks >= 1 && healers >= 1
    }
}

// --- Ready check ---

/// Timeout for a ready check (seconds).
const READY_CHECK_TIMEOUT: f32 = 30.0;

/// A player's response to a ready check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadyResponse {
    Ready,
    NotReady,
    /// No response yet.
    Pending,
}

/// An active ready check initiated by the group leader.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadyCheck {
    pub responses: Vec<(u64, ReadyResponse)>,
    pub time_remaining: f32,
}

impl ReadyCheck {
    /// Start a ready check for the given members.
    pub fn new(members: &[u64]) -> Self {
        Self {
            responses: members
                .iter()
                .map(|&m| (m, ReadyResponse::Pending))
                .collect(),
            time_remaining: READY_CHECK_TIMEOUT,
        }
    }

    /// Record a player's response. Returns `false` if player not in the check.
    pub fn respond(&mut self, player: u64, response: ReadyResponse) -> bool {
        if let Some(entry) = self.responses.iter_mut().find(|(m, _)| *m == player) {
            entry.1 = response;
            true
        } else {
            false
        }
    }

    /// Tick the timer. Returns `true` if timed out.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.time_remaining -= dt;
        self.time_remaining <= 0.0
    }

    /// Whether all members have responded (no Pending left).
    pub fn all_responded(&self) -> bool {
        self.responses
            .iter()
            .all(|(_, r)| *r != ReadyResponse::Pending)
    }

    /// Whether everyone responded Ready.
    pub fn all_ready(&self) -> bool {
        self.responses
            .iter()
            .all(|(_, r)| *r == ReadyResponse::Ready)
    }

    /// Count of pending / ready / not-ready.
    pub fn counts(&self) -> (usize, usize, usize) {
        let pending = self
            .responses
            .iter()
            .filter(|(_, r)| *r == ReadyResponse::Pending)
            .count();
        let ready = self
            .responses
            .iter()
            .filter(|(_, r)| *r == ReadyResponse::Ready)
            .count();
        let not_ready = self
            .responses
            .iter()
            .filter(|(_, r)| *r == ReadyResponse::NotReady)
            .count();
        (pending, ready, not_ready)
    }
}

// --- Group XP sharing ---

use crate::xp::{self, GroupMemberXp};

/// Info about a party/raid member for XP distribution.
#[derive(Debug, Clone, Copy)]
pub struct MemberInfo {
    pub entity: u64,
    pub level: u8,
    pub distance_from_kill: f32,
}

/// Distribute kill XP among party members based on proximity and level.
///
/// Wraps `xp::group_kill_xp()` with party member lookups.
/// Members beyond `GROUP_XP_RANGE` (100y) get 0 XP.
pub fn party_kill_xp(
    party: &Party,
    creature_level: u8,
    member_info: &[MemberInfo],
) -> Vec<(u64, u32)> {
    let members: Vec<GroupMemberXp> = party
        .members
        .iter()
        .filter_map(|&entity| {
            member_info
                .iter()
                .find(|m| m.entity == entity)
                .map(|m| GroupMemberXp {
                    level: m.level,
                    distance: m.distance_from_kill,
                })
        })
        .collect();

    let shares = xp::group_kill_xp(creature_level, &members);

    party
        .members
        .iter()
        .zip(shares.iter())
        .map(|(&entity, share)| (entity, share.xp))
        .collect()
}

/// Distribute kill XP among raid members.
///
/// Same as party XP but uses all raid members across subgroups.
pub fn raid_kill_xp(
    raid: &Raid,
    creature_level: u8,
    member_info: &[MemberInfo],
) -> Vec<(u64, u32)> {
    let all_members: Vec<u64> = raid.all_members().collect();
    let members: Vec<GroupMemberXp> = all_members
        .iter()
        .filter_map(|&entity| {
            member_info
                .iter()
                .find(|m| m.entity == entity)
                .map(|m| GroupMemberXp {
                    level: m.level,
                    distance: m.distance_from_kill,
                })
        })
        .collect();

    let shares = xp::group_kill_xp(creature_level, &members);

    all_members
        .iter()
        .zip(shares.iter())
        .map(|(&entity, share)| (entity, share.xp))
        .collect()
}

// --- Shared threat ---

use crate::components::ThreatTable;

/// Apply damage threat from a group member to a mob's threat table.
///
/// The group member's threat is attributed directly (1:1 ratio × modifier).
/// All group members share the same threat table on the mob.
pub fn apply_group_damage_threat(table: &mut ThreatTable, source: u64, damage: f32, modifier: f32) {
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
pub fn engaged_mobs(mob_threats: &[(u64, &ThreatTable)], group_members: &[u64]) -> Vec<u64> {
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
#[path = "group_tests.rs"]
mod tests;
