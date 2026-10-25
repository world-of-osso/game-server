//! Pure party/raid state keyed by character id. Systems in `group/mod.rs` feed it the
//! online player view and turn the returned [`GroupEffect`]s into network messages.
//!
//! Groups live in memory only: a group survives a member's relog because members are
//! character ids, not entities. A group disbands once no member is online.

use std::collections::{BTreeSet, HashMap};

use shared::group::{
    GroupRole, MAX_PARTY_SIZE, PartyError, Raid, ReadyCheck, ReadyResponse, RoleAssignments,
};
use shared::loot::LootMode;
use shared::protocol::{
    GROUP_INVITE_TIMEOUT_SECS, GroupAssistants, GroupCountdownUpdate, GroupMemberSnapshot,
    GroupMessageCode, GroupPortraitAppearance, GroupRoleSnapshot, GroupRosterSnapshot, MinimapPing,
    MinimapPingRequest, RAID_TARGET_ICON_COUNT, READY_CHECK_DURATION_SECS, RaidTargetIcons,
    ReadyCheckAnswer, ReadyCheckMemberSnapshot, ReadyCheckUpdate,
};

pub type CharacterId = u64;
pub use server_map_rules::maps::GroupId;

/// A player currently in the world.
#[derive(Clone, Debug, PartialEq)]
pub struct OnlineMember {
    pub entity: u64,
    pub name: String,
    pub class: u8,
    pub level: u8,
    pub portrait: GroupPortraitAppearance,
    /// `Player::GetDungeonDifficultyID`: a group it creates starts at it.
    pub dungeon_difficulty: u32,
    pub raid_difficulty: crate::maps::RaidDifficulty,
}

pub type OnlineView = HashMap<CharacterId, OnlineMember>;

/// Something the network layer must send.
#[derive(Clone, Debug, PartialEq)]
pub enum GroupEffect {
    Notice {
        to: CharacterId,
        code: GroupMessageCode,
        name: String,
    },
    /// Send the current roster to every online member.
    Roster(GroupId),
    Assistants(GroupId),
    /// Tell a former member it is no longer in a group.
    ClearRoster(CharacterId),
    InvitePrompt {
        to: CharacterId,
        inviter_name: String,
    },
    /// Close the invitee's invite popup: the invite is no longer valid.
    InviteCancelled {
        to: CharacterId,
        inviter_name: String,
    },
    /// Send the ready check state to every participant.
    ReadyCheck(GroupId),
    /// Relay an authoritative player countdown snapshot to one group member.
    Countdown {
        to: CharacterId,
        update: GroupCountdownUpdate,
    },
    /// `Player::SetDungeonDifficultyID` + `SendDungeonDifficulty`: the member now enters
    /// dungeons at the group's difficulty.
    DungeonDifficulty {
        to: CharacterId,
        difficulty_id: u32,
    },
    RaidDifficulty {
        to: CharacterId,
        selection: crate::maps::RaidDifficulty,
    },
    /// Send the group's raid target icons to every online member.
    TargetIcons(GroupId),
    /// Send a solo player their own raid target icons.
    SoloTargetIcons(CharacterId),
    /// Relay a group member's minimap ping.
    MinimapPing {
        to: CharacterId,
        ping: MinimapPing,
    },
}

#[derive(Clone, Debug)]
struct PendingInvite {
    inviter: CharacterId,
    invitee: CharacterId,
    remaining_secs: f32,
}

#[derive(Clone, Debug)]
struct ActiveReadyCheck {
    initiator_name: String,
    check: ReadyCheck,
    finished: bool,
}

#[derive(Clone, Debug)]
struct Group {
    /// Party members also live in `raid.subgroups[0]`; `is_raid` gates raid rules.
    raid: Raid,
    is_raid: bool,
    roles: RoleAssignments,
    ready_check: Option<ActiveReadyCheck>,
    countdown: Option<GroupCountdownUpdate>,
    assistants: BTreeSet<CharacterId>,
    /// Online entity, level and portrait last reported in a roster.
    reported_online: HashMap<CharacterId, Option<(u64, u8, GroupPortraitAppearance)>>,
    /// `Group::m_dungeonDifficulty`, the leader's when the group formed.
    dungeon_difficulty: u32,
    raid_difficulty: crate::maps::RaidDifficulty,
    /// `Group::m_targetIcons`.
    target_icons: RaidTargetIcons,
    loot_threshold: u8,
    loot_cursor: usize,
}

impl Group {
    fn new(leader: CharacterId, dungeon_difficulty: u32) -> Self {
        Self {
            dungeon_difficulty,
            raid_difficulty: crate::maps::RaidDifficulty::default(),
            raid: Raid {
                loot_mode: LootMode::GroupLoot,
                ..Raid::new(leader)
            },
            is_raid: false,
            roles: RoleAssignments::default(),
            ready_check: None,
            countdown: None,
            assistants: BTreeSet::new(),
            reported_online: HashMap::new(),
            target_icons: RaidTargetIcons::default(),
            loot_threshold: 2,
            loot_cursor: 0,
        }
    }

    fn members(&self) -> Vec<CharacterId> {
        self.raid.all_members().collect()
    }

    fn add(&mut self, member: CharacterId) -> Result<(), PartyError> {
        if !self.is_raid && self.raid.total_members() >= MAX_PARTY_SIZE {
            return Err(PartyError::Full);
        }
        self.raid.add_member(member).map(|_| ())
    }
}

/// Cached identity of a character that has been seen online.
#[derive(Clone, Debug, Default)]
struct Profile {
    name: String,
    class: u8,
    level: u8,
    portrait: GroupPortraitAppearance,
    dungeon_difficulty: u32,
    raid_difficulty: crate::maps::RaidDifficulty,
}

#[derive(bevy::prelude::Resource, Default)]
pub struct GroupRegistry {
    groups: HashMap<GroupId, Group>,
    membership: HashMap<CharacterId, GroupId>,
    invites: Vec<PendingInvite>,
    profiles: HashMap<CharacterId, Profile>,
    next_group_id: GroupId,
    /// Raid target icons of online players outside a group, for themselves only.
    solo_target_icons: HashMap<CharacterId, RaidTargetIcons>,
}

type Effects = Vec<GroupEffect>;

fn notice(to: CharacterId, code: GroupMessageCode, name: &str) -> GroupEffect {
    GroupEffect::Notice {
        to,
        code,
        name: name.into(),
    }
}

impl GroupRegistry {
    // --- Chat-routing hooks ---

    /// Stable group identity; private summons retain this ID across roster changes.
    pub(crate) fn group_id(&self, member: CharacterId) -> Option<GroupId> {
        self.membership.get(&member).copied()
    }

    /// Whether two characters are in the same party or raid.
    pub fn is_in_same_group(&self, a: CharacterId, b: CharacterId) -> bool {
        match (self.membership.get(&a), self.membership.get(&b)) {
            (Some(ga), Some(gb)) => ga == gb,
            _ => false,
        }
    }

    /// Whether the character belongs to a raid, rather than a party or no group.
    pub fn is_in_raid(&self, member: CharacterId) -> bool {
        self.group_of(member)
            .is_some_and(|(_, group)| group.is_raid)
    }

    /// Roster identity, including runtime companions that have no persisted character.
    pub(crate) fn member_identity(&self, member: CharacterId) -> Option<(&str, u8)> {
        self.profiles
            .get(&member)
            .map(|profile| (profile.name.as_str(), profile.class))
    }

    /// Every member of the character's group (including itself); empty when ungrouped.
    pub fn group_members(&self, member: CharacterId) -> Vec<CharacterId> {
        self.group_of(member)
            .map(|(_, group)| group.members())
            .unwrap_or_default()
    }

    /// The members of the character's subgroup (`Group::SameSubGroup`; a party is one
    /// subgroup), including itself; empty when ungrouped.
    pub fn subgroup_members(&self, member: CharacterId) -> Vec<CharacterId> {
        self.group_of(member)
            .and_then(|(_, group)| {
                let subgroup = group.raid.subgroup_of(member)?;
                Some(group.raid.subgroups[subgroup].clone())
            })
            .unwrap_or_default()
    }

    // --- Commands ---

    pub fn invite(
        &mut self,
        inviter: CharacterId,
        target_name: &str,
        online: &OnlineView,
    ) -> Effects {
        match self.try_invite(inviter, target_name, online) {
            Ok(effects) => effects,
            Err((code, name)) => vec![notice(inviter, code, &name)],
        }
    }

    fn try_invite(
        &mut self,
        inviter: CharacterId,
        target_name: &str,
        online: &OnlineView,
    ) -> Result<Effects, (GroupMessageCode, String)> {
        let Some((invitee, target)) = find_online_by_name(online, target_name) else {
            return Err((GroupMessageCode::BadPlayerName, target_name.into()));
        };
        if invitee == inviter {
            return Err((GroupMessageCode::InviteSelf, String::new()));
        }
        if self.membership.contains_key(&invitee) || self.has_pending_invite(invitee) {
            return Err((GroupMessageCode::AlreadyInGroup, target.name.clone()));
        }
        self.check_can_invite(inviter)?;
        self.invites.push(PendingInvite {
            inviter,
            invitee,
            remaining_secs: GROUP_INVITE_TIMEOUT_SECS,
        });
        let inviter_name = online
            .get(&inviter)
            .map(|m| m.name.clone())
            .unwrap_or_default();
        Ok(vec![
            notice(inviter, GroupMessageCode::InvitePlayer, &target.name),
            GroupEffect::InvitePrompt {
                to: invitee,
                inviter_name,
            },
        ])
    }

    fn check_can_invite(&self, inviter: CharacterId) -> Result<(), (GroupMessageCode, String)> {
        let Some((_, group)) = self.group_of(inviter) else {
            // Retail: an ungrouped player has one outstanding invite until the party forms.
            if self.invites.iter().any(|i| i.inviter == inviter) {
                return Err((GroupMessageCode::InvitePartyBusy, String::new()));
            }
            return Ok(());
        };
        if group.raid.leader != inviter {
            return Err((GroupMessageCode::NotLeader, String::new()));
        }
        let capacity = if group.is_raid {
            shared::group::MAX_RAID_SIZE
        } else {
            MAX_PARTY_SIZE
        };
        if group.raid.total_members() >= capacity {
            return Err((GroupMessageCode::GroupFull, String::new()));
        }
        Ok(())
    }

    pub fn respond_invite(&mut self, invitee: CharacterId, accept: bool) -> Effects {
        let Some(index) = self.invites.iter().position(|i| i.invitee == invitee) else {
            return Vec::new();
        };
        let invite = self.invites.remove(index);
        let invitee_name = self.name_of(invitee);
        if !accept {
            return vec![notice(
                invite.inviter,
                GroupMessageCode::DeclineGroup,
                &invitee_name,
            )];
        }
        self.join(invite.inviter, invitee, &invitee_name)
    }

    fn join(&mut self, inviter: CharacterId, invitee: CharacterId, invitee_name: &str) -> Effects {
        let group_id = match self.membership.get(&inviter) {
            Some(&id) => id,
            None => self.create_group(inviter),
        };
        let group = self
            .groups
            .get_mut(&group_id)
            .expect("membership points at a group");
        if group.add(invitee).is_err() {
            if group.raid.total_members() <= 1 {
                self.disband(group_id);
            }
            return vec![notice(invitee, GroupMessageCode::GroupFull, "")];
        }
        self.membership.insert(invitee, group_id);
        // Group::AddMember: a party's target icons reset when a member joins.
        if !group.is_raid {
            group.target_icons = RaidTargetIcons::default();
        }
        let is_raid = group.is_raid;
        let others: Vec<_> = group
            .members()
            .into_iter()
            .filter(|&m| m != invitee)
            .collect();
        // Invites the joiner sent while ungrouped would pull players into this group.
        let mut effects = self.cancel_invites_from(invitee);
        self.solo_target_icons.remove(&invitee);
        self.solo_target_icons.remove(&inviter);
        let joined_code = if is_raid {
            GroupMessageCode::RaidMemberAdded
        } else {
            GroupMessageCode::JoinedGroup
        };
        effects.extend(
            others
                .into_iter()
                .map(|m| notice(m, joined_code, invitee_name)),
        );
        if is_raid {
            effects.push(notice(invitee, GroupMessageCode::RaidYouJoined, ""));
        }
        // Group::AddMember: the joiner takes the group's dungeon difficulty.
        effects.push(GroupEffect::DungeonDifficulty {
            to: invitee,
            difficulty_id: self.groups[&group_id].dungeon_difficulty,
        });
        effects.push(GroupEffect::RaidDifficulty {
            to: invitee,
            selection: self.groups[&group_id].raid_difficulty,
        });
        effects.push(GroupEffect::Roster(group_id));
        effects
    }

    /// `Group::Create`: the group starts at its leader's dungeon difficulty.
    fn create_group(&mut self, leader: CharacterId) -> GroupId {
        self.next_group_id += 1;
        let id = self.next_group_id;
        let difficulty = self
            .profiles
            .get(&leader)
            .map_or(crate::maps::store::DIFFICULTY_NORMAL, |profile| {
                profile.dungeon_difficulty
            });
        let mut group = Group::new(leader, difficulty);
        group.raid_difficulty = self
            .profiles
            .get(&leader)
            .map_or_default(|p| p.raid_difficulty);
        self.groups.insert(id, group);
        self.membership.insert(leader, id);
        id
    }

    pub fn leave(&mut self, member: CharacterId) -> Effects {
        let Some(&group_id) = self.membership.get(&member) else {
            return vec![notice(member, GroupMessageCode::NotInGroup, "")];
        };
        let is_raid = self.groups[&group_id].is_raid;
        let you_code = if is_raid {
            GroupMessageCode::RaidYouLeft
        } else {
            GroupMessageCode::LeftGroupYou
        };
        let others_code = if is_raid {
            GroupMessageCode::RaidMemberRemoved
        } else {
            GroupMessageCode::LeftGroup
        };
        let mut effects = vec![notice(member, you_code, "")];
        effects.extend(self.remove_member(group_id, member, others_code));
        effects
    }

    pub fn uninvite(&mut self, leader: CharacterId, target_name: &str) -> Effects {
        let (group_id, target) = match self.leader_target(leader, target_name) {
            Ok(found) => found,
            Err(effect) => return vec![effect],
        };
        let others_code = if self.groups[&group_id].is_raid {
            GroupMessageCode::RaidMemberRemoved
        } else {
            GroupMessageCode::LeftGroup
        };
        let mut effects = vec![notice(target, GroupMessageCode::UninviteYou, "")];
        effects.extend(self.remove_member(group_id, target, others_code));
        effects
    }

    /// Removes `member`, passing leadership and disbanding a group left with one member.
    fn remove_member(
        &mut self,
        group_id: GroupId,
        member: CharacterId,
        others_code: GroupMessageCode,
    ) -> Effects {
        let name = self.name_of(member);
        let mut effects = self.cancel_invites_from(member);
        self.membership.remove(&member);
        let group = self.groups.get_mut(&group_id).expect("group exists");
        let old_leader = group.raid.leader;
        let disband = group.raid.leave(member);
        group.roles.remove(member);
        group.assistants.remove(&member);
        group.reported_online.remove(&member);
        if let Some(active) = &mut group.ready_check {
            active.check.responses.retain(|(m, _)| *m != member);
        }
        effects.push(GroupEffect::ClearRoster(member));
        if disband {
            effects.extend(self.disband(group_id));
            return effects;
        }
        let remaining = group.members();
        effects.extend(remaining.iter().map(|&m| notice(m, others_code, &name)));
        if group.raid.leader != old_leader {
            let new_leader = group.raid.leader;
            effects.extend(self.leader_changed_notices(group_id, new_leader));
        }
        effects.push(GroupEffect::Roster(group_id));
        effects
    }

    fn disband(&mut self, group_id: GroupId) -> Effects {
        let Some(group) = self.groups.remove(&group_id) else {
            return Vec::new();
        };
        let mut effects = Vec::new();
        for member in group.members() {
            self.membership.remove(&member);
            effects.push(notice(member, GroupMessageCode::GroupDisbanded, ""));
            effects.push(GroupEffect::ClearRoster(member));
        }
        effects
    }

    pub fn promote(&mut self, leader: CharacterId, target_name: &str) -> Effects {
        let (group_id, target) = match self.leader_target(leader, target_name) {
            Ok(found) => found,
            Err(effect) => return vec![effect],
        };
        self.groups
            .get_mut(&group_id)
            .expect("group exists")
            .raid
            .leader = target;
        let mut effects = self.cancel_invites_from(leader);
        effects.extend(self.leader_changed_notices(group_id, target));
        effects.push(GroupEffect::Roster(group_id));
        effects
    }

    fn leader_changed_notices(&self, group_id: GroupId, new_leader: CharacterId) -> Effects {
        let name = self.name_of(new_leader);
        self.groups[&group_id]
            .members()
            .into_iter()
            .map(|m| {
                if m == new_leader {
                    notice(m, GroupMessageCode::NewLeaderYou, "")
                } else {
                    notice(m, GroupMessageCode::NewLeader, &name)
                }
            })
            .collect()
    }

    pub fn convert_to_raid(&mut self, leader: CharacterId) -> Effects {
        let group_id = match self.led_group(leader) {
            Ok(id) => id,
            Err(effect) => return vec![effect],
        };
        let group = self.groups.get_mut(&group_id).expect("group exists");
        if group.is_raid {
            return Vec::new();
        }
        group.is_raid = true;
        let mut effects: Effects = group
            .members()
            .into_iter()
            .map(|m| notice(m, GroupMessageCode::PartyConvertedToRaid, ""))
            .collect();
        effects.push(GroupEffect::Roster(group_id));
        effects
    }

    pub fn convert_to_party(&mut self, leader: CharacterId) -> Effects {
        let group_id = match self.led_group(leader) {
            Ok(id) => id,
            Err(effect) => return vec![effect],
        };
        let group = self.groups.get_mut(&group_id).expect("group exists");
        if !group.is_raid {
            return vec![notice(leader, GroupMessageCode::NotInRaid, "")];
        }
        if group.raid.collapse_to_party().is_err() {
            return vec![notice(leader, GroupMessageCode::GroupFull, "")];
        }
        group.is_raid = false;
        group.assistants.clear();
        let mut effects: Effects = group
            .members()
            .into_iter()
            .map(|m| notice(m, GroupMessageCode::RaidConvertedToParty, ""))
            .collect();
        effects.push(GroupEffect::Roster(group_id));
        effects
    }

    /// Moves a raid member to `subgroup` (1–8). A full target subgroup is a no-op, as in
    /// Retail where the client swaps members instead.
    pub fn set_subgroup(
        &mut self,
        leader: CharacterId,
        target_name: &str,
        subgroup: u8,
    ) -> Effects {
        let (group_id, target) = match self.leader_target(leader, target_name) {
            Ok(found) => found,
            Err(effect) => return vec![effect],
        };
        let group = self.groups.get_mut(&group_id).expect("group exists");
        if !group.is_raid {
            return vec![notice(leader, GroupMessageCode::NotInRaid, "")];
        }
        let Some(index) = usize::from(subgroup).checked_sub(1) else {
            return Vec::new();
        };
        if group.raid.subgroup_of(target) == Some(index) {
            return Vec::new();
        }
        match group.raid.move_to_subgroup(target, index) {
            Ok(()) => vec![GroupEffect::Roster(group_id)],
            Err(_) => {
                bevy::log::info!("[Group] subgroup {subgroup} full or invalid; move ignored");
                Vec::new()
            }
        }
    }

    pub fn set_role(
        &mut self,
        requester: CharacterId,
        target_name: &str,
        role: GroupRoleSnapshot,
    ) -> Effects {
        let Some((group_id, group)) = self.group_of(requester) else {
            return vec![notice(requester, GroupMessageCode::NotInGroup, "")];
        };
        let Some(target) = self.member_by_name(group, target_name) else {
            return vec![notice(
                requester,
                GroupMessageCode::TargetNotInGroup,
                target_name,
            )];
        };
        if target != requester && group.raid.leader != requester {
            return vec![notice(requester, GroupMessageCode::NotLeader, "")];
        }
        let group = self.groups.get_mut(&group_id).expect("group exists");
        match role_from_snapshot(role) {
            Some(role) => group.roles.assign(target, role),
            None => group.roles.remove(target),
        }
        vec![GroupEffect::Roster(group_id)]
    }

    pub fn loot_policy_of(&self, member: CharacterId) -> Option<(LootMode, u8)> {
        self.group_of(member)
            .map(|(_, group)| (group.raid.loot_mode, group.loot_threshold))
    }

    pub fn next_loot_owner(
        &mut self,
        member: CharacterId,
        eligible: &[CharacterId],
    ) -> CharacterId {
        let group_id = self.group_id_of(member).expect("group member");
        let group = self.groups.get_mut(&group_id).expect("group exists");
        let owner = eligible[group.loot_cursor % eligible.len()];
        group.loot_cursor += 1;
        owner
    }

    pub fn set_loot_threshold(&mut self, leader: CharacterId, quality: u8) -> Effects {
        let group_id = match self.led_group(leader) {
            Ok(id) => id,
            Err(effect) => return vec![effect],
        };
        if !(2..=4).contains(&quality) {
            return vec![notice(leader, GroupMessageCode::LootMethodUnavailable, "")];
        }
        self.groups
            .get_mut(&group_id)
            .expect("group exists")
            .loot_threshold = quality;
        vec![GroupEffect::Roster(group_id)]
    }

    pub fn set_loot_method(&mut self, leader: CharacterId, method: LootMode) -> Effects {
        let group_id = match self.led_group(leader) {
            Ok(id) => id,
            Err(effect) => return vec![effect],
        };
        let group = self.groups.get_mut(&group_id).expect("group exists");
        if !method.is_mainline() {
            return vec![notice(leader, GroupMessageCode::LootMethodUnavailable, "")];
        }
        group.raid.loot_mode = method;
        let code = loot_code(method);
        let mut effects: Effects = group
            .members()
            .into_iter()
            .map(|m| notice(m, code, ""))
            .collect();
        effects.push(GroupEffect::Roster(group_id));
        effects
    }

    pub fn set_assistant(&mut self, leader: CharacterId, name: &str, assistant: bool) -> Effects {
        let (id, member) = match self.leader_target(leader, name) {
            Ok(found) => found,
            Err(effect) => return vec![effect],
        };
        let group = self.groups.get_mut(&id).expect("group exists");
        if !group.is_raid {
            return vec![notice(leader, GroupMessageCode::NotInRaid, "")];
        }
        if assistant {
            group.assistants.insert(member);
        } else {
            group.assistants.remove(&member);
        }
        vec![GroupEffect::Assistants(id)]
    }

    pub fn build_assistants(&self, group_id: GroupId) -> GroupAssistants {
        let members = self
            .groups
            .get(&group_id)
            .map(|group| {
                group
                    .members()
                    .into_iter()
                    .filter(|id| group.assistants.contains(id))
                    .map(|id| self.name_of(id))
                    .collect()
            })
            .unwrap_or_default();
        GroupAssistants { members }
    }

    // --- Player countdown ---

    pub fn set_countdown(&mut self, from: CharacterId, seconds: u32, now_unix_ms: u64) -> Effects {
        let group_id = match self.privileged_group(from) {
            Ok(id) => id,
            Err(effect) => return vec![effect],
        };
        let update = GroupCountdownUpdate {
            deadline_unix_ms: now_unix_ms + u64::from(seconds) * 1000,
            initiator_name: self.name_of(from),
            duration_secs: seconds,
            time_remaining_secs: seconds as f32,
        };
        let group = self.groups.get_mut(&group_id).expect("group exists");
        if seconds == 0 && group.countdown.is_none() {
            return Vec::new();
        }
        group.countdown = (seconds > 0).then(|| update.clone());
        group
            .members()
            .into_iter()
            .map(|to| GroupEffect::Countdown {
                to,
                update: update.clone(),
            })
            .collect()
    }

    fn tick_countdowns(&mut self, dt_secs: f32) -> Effects {
        let mut effects = Vec::new();
        for group in self.groups.values_mut() {
            let Some(countdown) = group.countdown.as_mut() else {
                continue;
            };
            countdown.time_remaining_secs = (countdown.time_remaining_secs - dt_secs).max(0.0);
            if countdown.time_remaining_secs > 0.0 {
                continue;
            }
            let update = group.countdown.take().expect("expired countdown");
            effects.extend(
                group
                    .members()
                    .into_iter()
                    .map(|to| GroupEffect::Countdown {
                        to,
                        update: update.clone(),
                    }),
            );
        }
        effects
    }

    // --- Ready check ---

    pub fn start_ready_check(&mut self, leader: CharacterId, online: &OnlineView) -> Effects {
        let group_id = match self.privileged_group(leader) {
            Ok(id) => id,
            Err(effect) => return vec![effect],
        };
        let initiator_name = self.name_of(leader);
        let group = self.groups.get_mut(&group_id).expect("group exists");
        if group.ready_check.as_ref().is_some_and(|c| !c.finished) {
            return vec![notice(leader, GroupMessageCode::ReadyCheckInProgress, "")];
        }
        let participants: Vec<_> = group
            .members()
            .into_iter()
            .filter(|m| online.contains_key(m))
            .collect();
        let mut check = ReadyCheck::new(&participants);
        check.time_remaining = READY_CHECK_DURATION_SECS;
        // Retail: the initiator is ready.
        check.respond(leader, ReadyResponse::Ready);
        group.ready_check = Some(ActiveReadyCheck {
            initiator_name: initiator_name.clone(),
            check,
            finished: false,
        });
        let mut effects: Effects = participants
            .iter()
            .map(|&m| notice(m, GroupMessageCode::ReadyCheckStarted, &initiator_name))
            .collect();
        effects.push(GroupEffect::ReadyCheck(group_id));
        effects.extend(self.finish_ready_check_if_done(group_id, false));
        effects
    }

    pub fn respond_ready_check(&mut self, member: CharacterId, ready: bool) -> Effects {
        let Some(&group_id) = self.membership.get(&member) else {
            return Vec::new();
        };
        let group = self.groups.get_mut(&group_id).expect("group exists");
        let Some(active) = group.ready_check.as_mut().filter(|c| !c.finished) else {
            return Vec::new();
        };
        let response = if ready {
            ReadyResponse::Ready
        } else {
            ReadyResponse::NotReady
        };
        if !active.check.respond(member, response) {
            return Vec::new();
        }
        let mut effects = vec![GroupEffect::ReadyCheck(group_id)];
        effects.extend(self.finish_ready_check_if_done(group_id, false));
        effects
    }

    fn finish_ready_check_if_done(&mut self, group_id: GroupId, timed_out: bool) -> Effects {
        let group = self.groups.get_mut(&group_id).expect("group exists");
        let Some(active) = group.ready_check.as_mut().filter(|c| !c.finished) else {
            return Vec::new();
        };
        if !timed_out && !active.check.all_responded() {
            return Vec::new();
        }
        active.finished = true;
        let code = if active.check.all_ready() {
            GroupMessageCode::ReadyCheckAllReady
        } else {
            GroupMessageCode::ReadyCheckFinished
        };
        let mut effects: Effects = active
            .check
            .responses
            .iter()
            .map(|&(m, _)| notice(m, code, ""))
            .collect();
        effects.push(GroupEffect::ReadyCheck(group_id));
        effects.push(GroupEffect::Roster(group_id));
        effects
    }

    // --- Time and presence ---

    /// Expires invites (treated as a decline) and ready checks.
    pub fn tick(&mut self, dt_secs: f32) -> Effects {
        let mut effects = self.tick_countdowns(dt_secs);
        let mut expired = Vec::new();
        self.invites.retain_mut(|invite| {
            invite.remaining_secs -= dt_secs;
            let keep = invite.remaining_secs > 0.0;
            if !keep {
                expired.push(invite.clone());
            }
            keep
        });
        for invite in expired {
            let name = self.name_of(invite.invitee);
            effects.push(notice(
                invite.inviter,
                GroupMessageCode::DeclineGroup,
                &name,
            ));
            effects.push(GroupEffect::InviteCancelled {
                to: invite.invitee,
                inviter_name: self.name_of(invite.inviter),
            });
        }
        let mut timed_out = Vec::new();
        for (&id, group) in &mut self.groups {
            if let Some(active) = group.ready_check.as_mut().filter(|c| !c.finished)
                && active.check.tick(dt_secs)
            {
                timed_out.push(id);
            }
        }
        for id in timed_out {
            effects.extend(self.finish_ready_check_if_done(id, true));
        }
        effects
    }

    /// Atomically form a solo player's runtime companion group, without human invites.
    pub(crate) fn create_companion_party(
        &mut self,
        leader: CharacterId,
        role: GroupRole,
        companions: &[(CharacterId, GroupRole)],
        online: &OnlineView,
    ) -> Result<GroupId, String> {
        if companions.len() != MAX_PARTY_SIZE - 1 || self.group_id_of(leader).is_some() {
            return Err("companion party requires a solo player and four followers".into());
        }
        let ids: std::collections::HashSet<_> = companions.iter().map(|&(id, _)| id).collect();
        if ids.len() != companions.len() || ids.contains(&leader) {
            return Err("duplicate companion party identity".into());
        }
        for (required, count) in [
            (GroupRole::Tank, 1),
            (GroupRole::Healer, 1),
            (GroupRole::Dps, 3),
        ] {
            let assigned = companions
                .iter()
                .filter(|(_, role)| *role == required)
                .count()
                + usize::from(role == required);
            if assigned != count {
                return Err("invalid follower party composition".into());
            }
        }
        if std::iter::once(&leader)
            .chain(ids.iter())
            .any(|id| !online.contains_key(id) || self.membership.contains_key(id))
        {
            return Err("companion party member unavailable".into());
        }
        self.remember_profiles(online);
        let group_id = self.create_group(leader);
        let group = self
            .groups
            .get_mut(&group_id)
            .expect("created companion group");
        group.roles.assign(leader, role);
        for &(id, role) in companions {
            group.add(id).expect("validated party capacity");
            group.roles.assign(id, role);
            self.membership.insert(id, group_id);
        }
        Ok(group_id)
    }

    fn remember_profiles(&mut self, online: &OnlineView) {
        for (&id, member) in online {
            self.profiles.insert(
                id,
                Profile {
                    name: member.name.clone(),
                    class: member.class,
                    level: member.level,
                    portrait: member.portrait.clone(),
                    dungeon_difficulty: member.dungeon_difficulty,
                    raid_difficulty: member.raid_difficulty,
                },
            );
        }
    }

    /// Records who is online. Resends rosters when a member logs in, out or relogs, and
    /// disbands groups with no online member.
    pub fn sync_presence(&mut self, online: &OnlineView) -> Effects {
        self.remember_profiles(online);
        self.solo_target_icons
            .retain(|id, _| online.contains_key(id));
        let mut effects = Vec::new();
        let (kept, dropped): (Vec<_>, Vec<_>) = std::mem::take(&mut self.invites)
            .into_iter()
            .partition(|i| online.contains_key(&i.invitee) && online.contains_key(&i.inviter));
        self.invites = kept;
        for invite in dropped
            .into_iter()
            .filter(|i| online.contains_key(&i.invitee))
        {
            effects.push(GroupEffect::InviteCancelled {
                to: invite.invitee,
                inviter_name: self.name_of(invite.inviter),
            });
        }
        let mut abandoned = Vec::new();
        for (&id, group) in &mut self.groups {
            let current: HashMap<_, _> = group
                .members()
                .into_iter()
                .map(|m| (m, reported_state(online, m)))
                .collect();
            if current.values().all(Option::is_none) {
                abandoned.push(id);
            } else if current != group.reported_online {
                effects.push(GroupEffect::Roster(id));
            }
        }
        for id in abandoned {
            let group = self.groups.remove(&id).expect("abandoned group exists");
            for member in group.members() {
                self.membership.remove(&member);
            }
        }
        effects
    }

    /// `WorldSession::HandleUpdateRaidTargetOpcode` → `Group::SetTargetIcon`: `icon` 1–8
    /// moves that icon onto `target`, 0 clears the target's icon. In a raid only the leader
    /// marks (assistants are not modeled). Outside a group the icons are the player's own.
    pub fn set_raid_target(&mut self, from: CharacterId, target: u64, icon: u8) -> Effects {
        if usize::from(icon) > RAID_TARGET_ICON_COUNT {
            return Vec::new();
        }
        let Some(&group_id) = self.membership.get(&from) else {
            set_target_icon(
                self.solo_target_icons.entry(from).or_default(),
                target,
                icon,
            );
            return vec![GroupEffect::SoloTargetIcons(from)];
        };
        let group = self
            .groups
            .get_mut(&group_id)
            .expect("membership points at a group");
        if group.is_raid && group.raid.leader != from {
            return Vec::new();
        }
        set_target_icon(&mut group.target_icons, target, icon);
        vec![GroupEffect::TargetIcons(group_id)]
    }

    /// `WorldSession::HandleMinimapPingOpcode` (TrinityCore a352b1fa
    /// `GroupHandler.cpp:399-410`): a grouped sender's ping goes to its other group
    /// members (`BroadcastPacket(..., ignoredPlayer = sender)`); ungrouped senders and
    /// positions off the map (`MinimapPingRequest::is_valid`) are dropped.
    pub fn minimap_ping(
        &self,
        from: CharacterId,
        request: MinimapPingRequest,
        online: &OnlineView,
    ) -> Effects {
        let Some(sender) = online.get(&from) else {
            return Vec::new();
        };
        if !request.is_valid() {
            return Vec::new();
        }
        let ping = MinimapPing {
            sender: sender.entity,
            sender_name: sender.name.clone(),
            x: request.x,
            y: request.y,
        };
        self.group_members(from)
            .into_iter()
            .filter(|&member| member != from)
            .map(|to| GroupEffect::MinimapPing {
                to,
                ping: ping.clone(),
            })
            .collect()
    }

    /// The raid target icons `member` sees: its group's, or its own while solo.
    pub fn target_icons_of(&self, member: CharacterId) -> RaidTargetIcons {
        match self.group_of(member) {
            Some((_, group)) => group.target_icons.clone(),
            None => self
                .solo_target_icons
                .get(&member)
                .cloned()
                .unwrap_or_default(),
        }
    }

    // --- Snapshots ---

    pub fn roster_members(&self, group_id: GroupId) -> Vec<CharacterId> {
        self.groups
            .get(&group_id)
            .map(Group::members)
            .unwrap_or_default()
    }

    /// Builds the roster and remembers which online state it reported.
    pub fn build_roster(
        &mut self,
        group_id: GroupId,
        online: &OnlineView,
    ) -> Option<GroupRosterSnapshot> {
        let group = self.groups.get(&group_id)?;
        let members = group
            .members()
            .into_iter()
            .map(|m| self.member_snapshot(group, m, online))
            .collect();
        let (ready_count, total_count) = group
            .ready_check
            .as_ref()
            .map(|c| {
                let (_, ready, _) = c.check.counts();
                (ready as u16, c.check.responses.len() as u16)
            })
            .unwrap_or((0, 0));
        let snapshot = GroupRosterSnapshot {
            is_raid: group.is_raid,
            ready_count,
            total_count,
            members,
            loot_method: group.raid.loot_mode,
            loot_threshold: group.loot_threshold,
        };
        let reported = group
            .members()
            .into_iter()
            .map(|m| (m, reported_state(online, m)))
            .collect();
        self.groups.get_mut(&group_id)?.reported_online = reported;
        Some(snapshot)
    }

    fn member_snapshot(
        &self,
        group: &Group,
        member: CharacterId,
        online: &OnlineView,
    ) -> GroupMemberSnapshot {
        let profile = self.profiles.get(&member).cloned().unwrap_or_default();
        GroupMemberSnapshot {
            character_id: member,
            name: profile.name,
            role: group
                .roles
                .role_of(member)
                .map(role_to_snapshot)
                .unwrap_or(GroupRoleSnapshot::None),
            is_leader: group.raid.leader == member,
            online: online.contains_key(&member),
            subgroup: group.raid.subgroup_of(member).map_or(1, |g| g as u8 + 1),
            class: profile.class,
            level: profile.level,
            entity: online.get(&member).map(|o| o.entity),
            portrait: profile.portrait,
        }
    }

    pub fn build_ready_check(
        &self,
        group_id: GroupId,
    ) -> Option<(Vec<CharacterId>, ReadyCheckUpdate)> {
        let active = self.groups.get(&group_id)?.ready_check.as_ref()?;
        let members = active
            .check
            .responses
            .iter()
            .map(|&(m, response)| ReadyCheckMemberSnapshot {
                name: self.name_of(m),
                answer: match response {
                    ReadyResponse::Pending => ReadyCheckAnswer::Pending,
                    ReadyResponse::Ready => ReadyCheckAnswer::Ready,
                    ReadyResponse::NotReady => ReadyCheckAnswer::NotReady,
                },
            })
            .collect();
        let recipients = active.check.responses.iter().map(|&(m, _)| m).collect();
        Some((
            recipients,
            ReadyCheckUpdate {
                initiator_name: active.initiator_name.clone(),
                time_remaining_secs: active.check.time_remaining.max(0.0),
                members,
                finished: active.finished,
            },
        ))
    }

    /// `Group::SetDungeonDifficultyID` from `HandleSetDungeonDifficultyOpcode`: only the
    /// leader changes it (others are ignored, as TrinityCore returns silently); every
    /// member takes it. `None` when `member` is not in a group.
    pub fn set_dungeon_difficulty(
        &mut self,
        member: CharacterId,
        difficulty_id: u32,
    ) -> Option<Effects> {
        let (group_id, group) = self.group_of(member)?;
        if group.raid.leader != member || group.dungeon_difficulty == difficulty_id {
            return Some(Vec::new());
        }
        let members = group.members();
        self.groups
            .get_mut(&group_id)
            .expect("group exists")
            .dungeon_difficulty = difficulty_id;
        Some(
            members
                .into_iter()
                .map(|to| GroupEffect::DungeonDifficulty { to, difficulty_id })
                .collect(),
        )
    }

    pub fn set_raid_difficulty(
        &mut self,
        member: CharacterId,
        selection: crate::maps::RaidDifficulty,
    ) -> Option<Vec<CharacterId>> {
        let (id, group) = self.group_of(member)?;
        if group.raid.leader != member || group.raid_difficulty == selection {
            return Some(Vec::new());
        }
        let members = group.members();
        self.groups
            .get_mut(&id)
            .expect("group exists")
            .raid_difficulty = selection;
        Some(members)
    }

    pub fn raid_difficulty_of(&self, member: CharacterId) -> Option<crate::maps::RaidDifficulty> {
        self.group_of(member).map(|(_, g)| g.raid_difficulty)
    }

    pub fn is_raid_of(&self, member: CharacterId) -> bool {
        self.group_of(member).is_some_and(|(_, g)| g.is_raid)
    }

    // --- Lookups ---

    /// `Group::GetLeaderGUID` of `member`'s group.
    pub fn leader_of(&self, member: CharacterId) -> Option<CharacterId> {
        self.group_of(member).map(|(_, group)| group.raid.leader)
    }

    /// `Group::GetDungeonDifficultyID` of `member`'s group.
    pub fn dungeon_difficulty_of(&self, member: CharacterId) -> Option<u32> {
        self.group_of(member)
            .map(|(_, group)| group.dungeon_difficulty)
    }

    /// The group `member` belongs to (`Player::GetGroup`).
    pub fn group_id_of(&self, member: CharacterId) -> Option<GroupId> {
        self.membership.get(&member).copied()
    }

    fn group_of(&self, member: CharacterId) -> Option<(GroupId, &Group)> {
        let id = *self.membership.get(&member)?;
        Some((id, self.groups.get(&id)?))
    }

    fn privileged_group(&self, member: CharacterId) -> Result<GroupId, GroupEffect> {
        let Some((id, group)) = self.group_of(member) else {
            return Err(notice(member, GroupMessageCode::NotInGroup, ""));
        };
        let assistant = group.is_raid && group.assistants.contains(&member);
        if group.raid.leader != member && !assistant {
            return Err(notice(member, GroupMessageCode::NotLeader, ""));
        }
        Ok(id)
    }

    fn led_group(&self, leader: CharacterId) -> Result<GroupId, GroupEffect> {
        let Some((id, group)) = self.group_of(leader) else {
            return Err(notice(leader, GroupMessageCode::NotInGroup, ""));
        };
        if group.raid.leader != leader {
            return Err(notice(leader, GroupMessageCode::NotLeader, ""));
        }
        Ok(id)
    }

    fn leader_target(
        &self,
        leader: CharacterId,
        target_name: &str,
    ) -> Result<(GroupId, CharacterId), GroupEffect> {
        let id = self.led_group(leader)?;
        match self.member_by_name(&self.groups[&id], target_name) {
            Some(target) => Ok((id, target)),
            None => Err(notice(
                leader,
                GroupMessageCode::TargetNotInGroup,
                target_name,
            )),
        }
    }

    fn member_by_name(&self, group: &Group, name: &str) -> Option<CharacterId> {
        group.members().into_iter().find(|m| {
            self.profiles
                .get(m)
                .is_some_and(|p| p.name.eq_ignore_ascii_case(name))
        })
    }

    /// Drops every pending invite `inviter` sent, closing the invitees' popups.
    fn cancel_invites_from(&mut self, inviter: CharacterId) -> Effects {
        let inviter_name = self.name_of(inviter);
        let (cancelled, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.invites)
            .into_iter()
            .partition(|i| i.inviter == inviter);
        self.invites = kept;
        cancelled
            .into_iter()
            .map(|i| GroupEffect::InviteCancelled {
                to: i.invitee,
                inviter_name: inviter_name.clone(),
            })
            .collect()
    }

    fn has_pending_invite(&self, invitee: CharacterId) -> bool {
        self.invites.iter().any(|i| i.invitee == invitee)
    }

    fn name_of(&self, member: CharacterId) -> String {
        self.profiles
            .get(&member)
            .map(|p| p.name.clone())
            .unwrap_or_default()
    }
}

/// `Group::SetTargetIcon`: the target loses its other icon, and `icon` (1–8) leaves its
/// previous unit for `target`; 0 only clears.
fn set_target_icon(icons: &mut RaidTargetIcons, target: u64, icon: u8) {
    for slot in &mut icons.targets {
        if *slot == Some(target) {
            *slot = None;
        }
    }
    if let Some(slot) = usize::from(icon)
        .checked_sub(1)
        .and_then(|index| icons.targets.get_mut(index))
    {
        *slot = Some(target);
    }
}

fn reported_state(
    online: &OnlineView,
    member: CharacterId,
) -> Option<(u64, u8, GroupPortraitAppearance)> {
    online
        .get(&member)
        .map(|o| (o.entity, o.level, o.portrait.clone()))
}

fn find_online_by_name<'a>(
    online: &'a OnlineView,
    name: &str,
) -> Option<(CharacterId, &'a OnlineMember)> {
    online
        .iter()
        .find(|(_, m)| m.name.eq_ignore_ascii_case(name))
        .map(|(&id, m)| (id, m))
}

fn role_from_snapshot(role: GroupRoleSnapshot) -> Option<GroupRole> {
    match role {
        GroupRoleSnapshot::Tank => Some(GroupRole::Tank),
        GroupRoleSnapshot::Healer => Some(GroupRole::Healer),
        GroupRoleSnapshot::Damage => Some(GroupRole::Dps),
        GroupRoleSnapshot::None => None,
    }
}

fn role_to_snapshot(role: GroupRole) -> GroupRoleSnapshot {
    match role {
        GroupRole::Tank => GroupRoleSnapshot::Tank,
        GroupRole::Healer => GroupRoleSnapshot::Healer,
        GroupRole::Dps => GroupRoleSnapshot::Damage,
    }
}

fn loot_code(method: LootMode) -> GroupMessageCode {
    match method {
        LootMode::FreeForAll => GroupMessageCode::SetLootFreeForAll,
        LootMode::RoundRobin => GroupMessageCode::SetLootRoundRobin,
        LootMode::NeedBeforeGreed => GroupMessageCode::SetLootNeedBeforeGreed,
        LootMode::PersonalLoot => GroupMessageCode::SetLootPersonal,
        LootMode::GroupLoot => GroupMessageCode::SetLootGroup,
    }
}
