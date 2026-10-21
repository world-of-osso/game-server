//! Party/raid backend: invites, leadership, raid subgroups, roles, loot method and
//! ready checks. State lives in [`GroupRegistry`]; this module moves messages in and out.

mod member_state;
mod registry;

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{EquipmentAppearance, EquipmentVisualSlot, Player, UnitLevel};
use shared::protocol::{
    ConvertGroupToParty, ConvertGroupToRaid, GroupAssistants, GroupChannel, GroupCommandResponse,
    GroupCountdownRequest, GroupCountdownUpdate, GroupInviteCancelled, GroupInviteIntent,
    GroupInvitePrompt, GroupPortraitAppearance, GroupRosterSnapshot, GroupUninviteIntent,
    LeaveGroup, MinimapPing, MinimapPingRequest, PromoteGroupLeader, RaidTargetIcons,
    ReadyCheckUpdate, RespondGroupInvite, RespondReadyCheck, SetGroupAssistant, SetGroupLootMethod,
    SetGroupLootThreshold, SetGroupRole, SetRaidSubgroup, SetRaidTarget, StartReadyCheck,
};

use crate::auth::Authenticated;
use crate::networking::PlayerLink;
use crate::persistence::ClientId;

pub use registry::{CharacterId, GroupEffect, GroupId, GroupRegistry, OnlineMember, OnlineView};

pub struct GroupPlugin;

impl Plugin for GroupPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GroupRegistry>()
            .init_resource::<crate::maps::difficulty::DifficultyChanges>()
            .init_resource::<crate::maps::raid::RaidChanges>();
        app.init_resource::<member_state::SentMemberStates>();
        app.add_systems(
            Update,
            (
                process_group_commands,
                member_state::send_group_member_states,
            )
                .chain(),
        );
    }
}

type Receivers<'w, 's, M> =
    Query<'w, 's, (&'static PlayerLink, &'static mut MessageReceiver<M>), With<Authenticated>>;

#[derive(SystemParam)]
struct GroupCommandReceivers<'w, 's> {
    invites: Receivers<'w, 's, GroupInviteIntent>,
    uninvites: Receivers<'w, 's, GroupUninviteIntent>,
    invite_answers: Receivers<'w, 's, RespondGroupInvite>,
    leaves: Receivers<'w, 's, LeaveGroup>,
    promotes: Receivers<'w, 's, PromoteGroupLeader>,
    converts: Receivers<'w, 's, ConvertGroupToRaid>,
    party_converts: Receivers<'w, 's, ConvertGroupToParty>,
    subgroups: Receivers<'w, 's, SetRaidSubgroup>,
    roles: Receivers<'w, 's, SetGroupRole>,
    assistants: Receivers<'w, 's, SetGroupAssistant>,
    loot: Receivers<'w, 's, SetGroupLootMethod>,
    loot_threshold: Receivers<'w, 's, SetGroupLootThreshold>,
    ready_starts: Receivers<'w, 's, StartReadyCheck>,
    ready_answers: Receivers<'w, 's, RespondReadyCheck>,
    countdowns: Receivers<'w, 's, GroupCountdownRequest>,
    raid_targets: Receivers<'w, 's, SetRaidTarget>,
    minimap_pings: Receivers<'w, 's, MinimapPingRequest>,
}

#[derive(SystemParam)]
struct GroupSenders<'w, 's> {
    connections: Query<'w, 's, (Entity, &'static PlayerLink), With<Authenticated>>,
    rosters: Query<'w, 's, &'static mut MessageSender<GroupRosterSnapshot>>,
    responses: Query<'w, 's, &'static mut MessageSender<GroupCommandResponse>>,
    prompts: Query<'w, 's, &'static mut MessageSender<GroupInvitePrompt>>,
    cancelled_invites: Query<'w, 's, &'static mut MessageSender<GroupInviteCancelled>>,
    ready_checks: Query<'w, 's, &'static mut MessageSender<ReadyCheckUpdate>>,
    countdowns: Query<'w, 's, &'static mut MessageSender<GroupCountdownUpdate>>,
    assistants: Query<'w, 's, &'static mut MessageSender<GroupAssistants>>,
    raid_targets: Query<'w, 's, &'static mut MessageSender<RaidTargetIcons>>,
    minimap_pings: Query<'w, 's, &'static mut MessageSender<MinimapPing>>,
}

type OnlinePlayers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Player,
        &'static ClientId,
        Option<&'static UnitLevel>,
        Option<&'static crate::maps::difficulty::DungeonDifficulty>,
        Option<&'static EquipmentAppearance>,
        Option<&'static crate::maps::RaidDifficulty>,
    ),
>;

fn process_group_commands(
    mut receivers: GroupCommandReceivers,
    mut senders: GroupSenders,
    players: OnlinePlayers,
    followers: Query<(Entity, &crate::followers::Follower, &UnitLevel)>,
    mut registry: ResMut<GroupRegistry>,
    time: Res<Time>,
    mut difficulty_changes: ResMut<crate::maps::difficulty::DifficultyChanges>,
    mut raid_changes: ResMut<crate::maps::raid::RaidChanges>,
) {
    let mut online = build_online_view(&players);
    online.extend(followers.iter().filter_map(|(entity, follower, level)| {
        let (_, _, _, _, difficulty, _, _) = players.get(follower.owner).ok()?;
        let difficulty = difficulty?.0;
        Some((
            crate::followers::Follower::member_id(entity),
            follower.online(entity, level.0, difficulty),
        ))
    }));
    let mut effects = registry.sync_presence(&online);
    effects.extend(registry.tick(time.delta_secs()));
    effects.extend(apply_commands(
        &mut receivers,
        &players,
        &online,
        &mut registry,
    ));
    let connections = connections_by_character(&senders.connections, &players);
    deliver_effects(&effects, &mut registry, &online, &connections, &mut senders);
    raid_changes
        .0
        .extend(effects.iter().filter_map(|effect| match *effect {
            GroupEffect::RaidDifficulty { to, selection } => Some((to, selection)),
            _ => None,
        }));
    difficulty_changes
        .0
        .extend(effects.iter().filter_map(|effect| match *effect {
            GroupEffect::DungeonDifficulty { to, difficulty_id } => Some((to, difficulty_id)),
            _ => None,
        }));
}

fn build_online_view(players: &OnlinePlayers) -> OnlineView {
    players
        .iter()
        .map(
            |(entity, player, client_id, level, difficulty, equipment, raid_difficulty)| {
                (
                    client_id.0,
                    OnlineMember {
                        entity: entity.to_bits(),
                        name: player.name.clone(),
                        class: player.class,
                        level: level.map_or(1, |l| l.0),
                        portrait: portrait_appearance(player, equipment),
                        raid_difficulty: raid_difficulty.copied().unwrap_or_default(),
                        dungeon_difficulty: difficulty
                            .map_or(crate::maps::store::DIFFICULTY_NORMAL, |difficulty| {
                                difficulty.0
                            }),
                    },
                )
            },
        )
        .collect()
}

pub(crate) fn portrait_appearance(
    player: &Player,
    equipment: Option<&EquipmentAppearance>,
) -> GroupPortraitAppearance {
    let head = equipment.and_then(|equipment| {
        equipment
            .entries
            .iter()
            .find(|entry| entry.slot == EquipmentVisualSlot::Head)
            .cloned()
    });
    GroupPortraitAppearance {
        creature_display_id: None,
        race: player.race,
        appearance: player.appearance.clone(),
        head,
    }
}

/// Drains one message type as `(sender character id, message)` pairs.
fn drain<M: lightyear::prelude::Message>(
    receivers: &mut Receivers<M>,
    players: &OnlinePlayers,
) -> Vec<(CharacterId, M)> {
    let mut drained = Vec::new();
    for (link, mut receiver) in receivers.iter_mut() {
        let Ok((_, _, client_id, _, _, _, _)) = players.get(link.0) else {
            receiver.receive().for_each(drop);
            continue;
        };
        drained.extend(receiver.receive().map(|msg| (client_id.0, msg)));
    }
    drained
}

fn apply_commands(
    rx: &mut GroupCommandReceivers,
    players: &OnlinePlayers,
    online: &OnlineView,
    registry: &mut GroupRegistry,
) -> Vec<GroupEffect> {
    let mut effects = Vec::new();
    for (from, msg) in drain(&mut rx.invites, players) {
        effects.extend(registry.invite(from, &msg.name, online));
    }
    for (from, msg) in drain(&mut rx.invite_answers, players) {
        effects.extend(registry.respond_invite(from, msg.accept));
    }
    for (from, msg) in drain(&mut rx.uninvites, players) {
        effects.extend(registry.uninvite(from, &msg.name));
    }
    for (from, _) in drain(&mut rx.leaves, players) {
        effects.extend(registry.leave(from));
    }
    for (from, msg) in drain(&mut rx.promotes, players) {
        effects.extend(registry.promote(from, &msg.name));
    }
    for (from, _) in drain(&mut rx.converts, players) {
        effects.extend(registry.convert_to_raid(from));
    }
    for (from, _) in drain(&mut rx.party_converts, players) {
        effects.extend(registry.convert_to_party(from));
    }
    for (from, msg) in drain(&mut rx.subgroups, players) {
        effects.extend(registry.set_subgroup(from, &msg.name, msg.subgroup));
    }
    for (from, msg) in drain(&mut rx.assistants, players) {
        effects.extend(registry.set_assistant(from, &msg.name, msg.assistant));
    }
    for (from, msg) in drain(&mut rx.roles, players) {
        effects.extend(registry.set_role(from, &msg.name, msg.role));
    }
    for (from, msg) in drain(&mut rx.loot, players) {
        effects.extend(registry.set_loot_method(from, msg.method));
    }
    for (from, msg) in drain(&mut rx.loot_threshold, players) {
        effects.extend(registry.set_loot_threshold(from, msg.quality));
    }
    for (from, _) in drain(&mut rx.ready_starts, players) {
        effects.extend(registry.start_ready_check(from, online));
    }
    for (from, msg) in drain(&mut rx.ready_answers, players) {
        effects.extend(registry.respond_ready_check(from, msg.ready));
    }
    for (from, msg) in drain(&mut rx.countdowns, players) {
        let now_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("server wall clock predates Unix epoch")
            .as_millis() as u64;
        effects.extend(registry.set_countdown(from, msg.seconds, now_unix_ms));
    }
    for (from, msg) in drain(&mut rx.raid_targets, players) {
        effects.extend(registry.set_raid_target(from, msg.target, msg.icon));
    }
    for (from, msg) in drain(&mut rx.minimap_pings, players) {
        effects.extend(registry.minimap_ping(from, msg, online));
    }
    effects
}

fn connections_by_character(
    connections: &Query<(Entity, &PlayerLink), With<Authenticated>>,
    players: &OnlinePlayers,
) -> HashMap<CharacterId, Entity> {
    connections
        .iter()
        .filter_map(|(conn, link)| {
            let (_, _, client_id, _, _, _, _) = players.get(link.0).ok()?;
            Some((client_id.0, conn))
        })
        .collect()
}

fn deliver_effects(
    effects: &[GroupEffect],
    registry: &mut GroupRegistry,
    online: &OnlineView,
    connections: &HashMap<CharacterId, Entity>,
    senders: &mut GroupSenders,
) {
    for effect in effects {
        match effect {
            GroupEffect::Notice { to, code, name } => send_to(
                &mut senders.responses,
                connections,
                *to,
                GroupCommandResponse {
                    message: code.format(name),
                    code: *code,
                },
            ),
            GroupEffect::Roster(group_id) => {
                let Some(roster) = registry.build_roster(*group_id, online) else {
                    continue;
                };
                for member in registry.roster_members(*group_id) {
                    send_to(&mut senders.rosters, connections, member, roster.clone());
                }
                // HandleRequestPartyJoinUpdates → SendTargetIconList: a roster change
                // (join, relog) brings the member the group's icons.
                send_group_target_icons(registry, *group_id, connections, senders);
                send_group_assistants(registry, *group_id, connections, senders);
            }
            GroupEffect::ClearRoster(to) => {
                send_to(&mut senders.rosters, connections, *to, empty_roster());
                send_to(
                    &mut senders.assistants,
                    connections,
                    *to,
                    GroupAssistants::default(),
                );
                send_to(
                    &mut senders.raid_targets,
                    connections,
                    *to,
                    RaidTargetIcons::default(),
                );
            }
            GroupEffect::Assistants(group_id) => {
                send_group_assistants(registry, *group_id, connections, senders);
            }
            GroupEffect::TargetIcons(group_id) => {
                send_group_target_icons(registry, *group_id, connections, senders)
            }
            GroupEffect::SoloTargetIcons(to) => send_to(
                &mut senders.raid_targets,
                connections,
                *to,
                registry.target_icons_of(*to),
            ),
            GroupEffect::InvitePrompt { to, inviter_name } => send_to(
                &mut senders.prompts,
                connections,
                *to,
                GroupInvitePrompt {
                    inviter_name: inviter_name.clone(),
                    timeout_secs: shared::protocol::GROUP_INVITE_TIMEOUT_SECS,
                },
            ),
            GroupEffect::InviteCancelled { to, inviter_name } => send_to(
                &mut senders.cancelled_invites,
                connections,
                *to,
                GroupInviteCancelled {
                    inviter_name: inviter_name.clone(),
                },
            ),
            GroupEffect::MinimapPing { to, ping } => {
                send_to(&mut senders.minimap_pings, connections, *to, ping.clone())
            }
            GroupEffect::Countdown { to, update } => {
                send_to(&mut senders.countdowns, connections, *to, update.clone());
            }
            // Applied to the player by maps::difficulty (DifficultyChanges).
            GroupEffect::DungeonDifficulty { .. } | GroupEffect::RaidDifficulty { .. } => {}
            GroupEffect::ReadyCheck(group_id) => {
                let Some((recipients, update)) = registry.build_ready_check(*group_id) else {
                    continue;
                };
                for member in recipients {
                    send_to(
                        &mut senders.ready_checks,
                        connections,
                        member,
                        update.clone(),
                    );
                }
            }
        }
    }
}

fn send_group_assistants(
    registry: &GroupRegistry,
    group_id: GroupId,
    connections: &HashMap<CharacterId, Entity>,
    senders: &mut GroupSenders,
) {
    let assistants = registry.build_assistants(group_id);
    for member in registry.roster_members(group_id) {
        send_to(
            &mut senders.assistants,
            connections,
            member,
            assistants.clone(),
        );
    }
}

fn send_group_target_icons(
    registry: &GroupRegistry,
    group_id: GroupId,
    connections: &HashMap<CharacterId, Entity>,
    senders: &mut GroupSenders,
) {
    for member in registry.roster_members(group_id) {
        let icons = registry.target_icons_of(member);
        send_to(&mut senders.raid_targets, connections, member, icons);
    }
}

pub(crate) fn empty_roster() -> GroupRosterSnapshot {
    GroupRosterSnapshot {
        is_raid: false,
        ready_count: 0,
        total_count: 0,
        members: Vec::new(),
        loot_method: shared::loot::LootMode::PersonalLoot,
        loot_threshold: 2,
    }
}

/// Offline recipients are skipped; they get a fresh roster on their next login.
fn send_to<M: lightyear::prelude::Message>(
    senders: &mut Query<&mut MessageSender<M>>,
    connections: &HashMap<CharacterId, Entity>,
    to: CharacterId,
    message: M,
) {
    let Some(&conn) = connections.get(&to) else {
        return;
    };
    if let Ok(mut sender) = senders.get_mut(conn) {
        sender.send::<GroupChannel>(message);
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
