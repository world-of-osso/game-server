//! Live party/raid frame data: every recipient gets `GroupMemberStates` with the states
//! of its online group members that changed since its last update, at most every
//! `GROUP_MEMBER_STATE_INTERVAL_SECS`. Sent regardless of interest range.

use std::collections::HashMap;

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{AuraView, Health, Player, Position, UnitAuras, UnitPowers};
use shared::death::DeathState;
use shared::protocol::{
    GROUP_MEMBER_STATE_INTERVAL_SECS, GroupChannel, GroupMemberState, GroupMemberStates,
};

use super::{CharacterId, GroupRegistry};
use crate::auth::Authenticated;
use crate::death::PlayerDeathState;
use crate::networking::PlayerLink;
use crate::persistence::ClientId;

/// What each connection was last sent, keyed by member name. A new connection (login
/// or relog) starts empty, so its first update carries every member.
#[derive(Resource, Default)]
pub(super) struct SentMemberStates {
    since_last_send: f32,
    sent: HashMap<Entity, HashMap<String, GroupMemberState>>,
}

type MemberUnits<'w, 's> = Query<
    'w,
    's,
    (
        &'static Player,
        &'static ClientId,
        &'static Health,
        &'static Position,
        Option<&'static UnitPowers>,
        Option<&'static UnitAuras>,
        Option<&'static PlayerDeathState>,
    ),
>;

pub(super) fn send_group_member_states(
    mut sent: ResMut<SentMemberStates>,
    time: Res<Time>,
    registry: Res<GroupRegistry>,
    units: MemberUnits,
    followers: Query<(
        Entity,
        &crate::followers::Follower,
        &Health,
        &Position,
        Option<&UnitPowers>,
        Option<&UnitAuras>,
    )>,
    mut connections: Query<
        (Entity, &PlayerLink, &mut MessageSender<GroupMemberStates>),
        With<Authenticated>,
    >,
) {
    sent.since_last_send += time.delta_secs();
    if sent.since_last_send < GROUP_MEMBER_STATE_INTERVAL_SECS {
        return;
    }
    sent.since_last_send = 0.0;
    let mut states = current_states(&units);
    states.extend(
        followers
            .iter()
            .map(|(entity, follower, health, position, powers, auras)| {
                (
                    crate::followers::Follower::member_id(entity),
                    GroupMemberState {
                        name: follower.profile.name.clone(),
                        health: health.current.max(0.0).round() as u32,
                        max_health: health.max.max(0.0).round() as u32,
                        power: powers.and_then(|power| power.entries.first().cloned()),
                        death: if health.current > 0.0 {
                            DeathState::Alive
                        } else {
                            DeathState::Dead
                        },
                        position: *position,
                        debuffs: auras.map(visible_debuffs).unwrap_or_default(),
                    },
                )
            }),
    );
    let mut next_sent = HashMap::new();
    for (conn, link, mut sender) in &mut connections {
        let Ok((_, recipient, ..)) = units.get(link.0) else {
            continue;
        };
        let group: HashMap<String, GroupMemberState> = registry
            .group_members(recipient.0)
            .iter()
            .filter_map(|member| states.get(member))
            .map(|state| (state.name.clone(), state.clone()))
            .collect();
        let previous = sent.sent.remove(&conn).unwrap_or_default();
        let changed = changed_states(&previous, &group);
        if !changed.is_empty() {
            sender.send::<GroupChannel>(GroupMemberStates { members: changed });
        }
        if !group.is_empty() {
            next_sent.insert(conn, group);
        }
    }
    sent.sent = next_sent;
}

fn current_states(units: &MemberUnits) -> HashMap<CharacterId, GroupMemberState> {
    units
        .iter()
        .map(|(player, id, health, position, powers, auras, death)| {
            let state = GroupMemberState {
                name: player.name.clone(),
                health: health.current.max(0.0).round() as u32,
                max_health: health.max.max(0.0).round() as u32,
                power: powers.and_then(|p| p.entries.first().cloned()),
                death: death.map_or(DeathState::Alive, |d| d.state),
                position: *position,
                debuffs: auras.map(visible_debuffs).unwrap_or_default(),
            };
            (id.0, state)
        })
        .collect()
}

fn visible_debuffs(auras: &UnitAuras) -> Vec<AuraView> {
    auras
        .auras
        .iter()
        .filter(|a| a.harmful && a.flags & AuraView::FLAG_HIDDEN == 0)
        .cloned()
        .collect()
}

/// States that are new or differ from what was sent, in name order.
fn changed_states(
    previous: &HashMap<String, GroupMemberState>,
    current: &HashMap<String, GroupMemberState>,
) -> Vec<GroupMemberState> {
    let mut changed: Vec<_> = current
        .values()
        .filter(|state| previous.get(&state.name) != Some(*state))
        .cloned()
        .collect();
    changed.sort_by(|a, b| a.name.cmp(&b.name));
    changed
}
