use bevy::prelude::*;
use lightyear::prelude::*;
use shared::class_spec::SelectedTalents;
use shared::protocol::{
    ApplyTalentChoice, QueryTalents, ResetTalents, TalentChannel, TalentNodeSnapshot,
    TalentSnapshot, TalentSpecTabSnapshot, TalentStateUpdate,
};

use crate::auth::Authenticated;
use crate::networking::PlayerLink;

const MAX_TALENT_POINTS: u16 = 51;
const SPEC_TABS: [&str; 3] = ["Protection", "Holy", "Retribution"];

struct TalentDef {
    id: u32,
    name: &'static str,
    spec_tab: usize,
    max_points: u8,
}

const TALENT_DEFS: [TalentDef; 28] = [
    TalentDef {
        id: 101,
        name: "Divine Strength",
        spec_tab: 0,
        max_points: 1,
    },
    TalentDef {
        id: 102,
        name: "Divine Intellect",
        spec_tab: 1,
        max_points: 1,
    },
    TalentDef {
        id: 103,
        name: "Stoicism",
        spec_tab: 0,
        max_points: 1,
    },
    TalentDef {
        id: 104,
        name: "Guardian's Favor",
        spec_tab: 1,
        max_points: 1,
    },
    TalentDef {
        id: 105,
        name: "Anticipation",
        spec_tab: 0,
        max_points: 1,
    },
    TalentDef {
        id: 106,
        name: "Conviction",
        spec_tab: 2,
        max_points: 1,
    },
    TalentDef {
        id: 107,
        name: "Toughness",
        spec_tab: 0,
        max_points: 1,
    },
    TalentDef {
        id: 108,
        name: "Improved Devotion Aura",
        spec_tab: 1,
        max_points: 1,
    },
    TalentDef {
        id: 109,
        name: "Improved Righteous Fury",
        spec_tab: 0,
        max_points: 1,
    },
    TalentDef {
        id: 110,
        name: "Seal of the Crusader",
        spec_tab: 2,
        max_points: 1,
    },
    TalentDef {
        id: 111,
        name: "Deflection",
        spec_tab: 2,
        max_points: 1,
    },
    TalentDef {
        id: 112,
        name: "Precision",
        spec_tab: 2,
        max_points: 1,
    },
    TalentDef {
        id: 113,
        name: "Redoubt",
        spec_tab: 0,
        max_points: 1,
    },
    TalentDef {
        id: 114,
        name: "Combat Expertise",
        spec_tab: 0,
        max_points: 1,
    },
    TalentDef {
        id: 115,
        name: "Spell Warding",
        spec_tab: 1,
        max_points: 1,
    },
    TalentDef {
        id: 116,
        name: "Blessing of Kings",
        spec_tab: 1,
        max_points: 1,
    },
    TalentDef {
        id: 117,
        name: "Ardent Defender",
        spec_tab: 0,
        max_points: 1,
    },
    TalentDef {
        id: 118,
        name: "Reckoning",
        spec_tab: 2,
        max_points: 1,
    },
    TalentDef {
        id: 119,
        name: "Shield Specialization",
        spec_tab: 0,
        max_points: 1,
    },
    TalentDef {
        id: 120,
        name: "Holy Shield",
        spec_tab: 0,
        max_points: 1,
    },
    TalentDef {
        id: 121,
        name: "One-Handed Weapon Spec",
        spec_tab: 0,
        max_points: 1,
    },
    TalentDef {
        id: 122,
        name: "Weapon Expertise",
        spec_tab: 2,
        max_points: 1,
    },
    TalentDef {
        id: 123,
        name: "Improved Holy Shield",
        spec_tab: 0,
        max_points: 1,
    },
    TalentDef {
        id: 124,
        name: "Sacred Duty",
        spec_tab: 1,
        max_points: 1,
    },
    TalentDef {
        id: 125,
        name: "Holy Shield Mastery",
        spec_tab: 0,
        max_points: 1,
    },
    TalentDef {
        id: 126,
        name: "Avenger's Shield",
        spec_tab: 0,
        max_points: 1,
    },
    TalentDef {
        id: 127,
        name: "Hammer of the Righteous",
        spec_tab: 0,
        max_points: 1,
    },
    TalentDef {
        id: 128,
        name: "Touched by the Light",
        spec_tab: 1,
        max_points: 1,
    },
];

pub struct TalentPlugin;

impl Plugin for TalentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, process_talent_queries);
        app.add_systems(Update, process_talent_apply_requests);
        app.add_systems(Update, process_talent_reset_requests);
    }
}

fn process_talent_queries(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<QueryTalents>),
        With<Authenticated>,
    >,
    talents: Query<&SelectedTalents>,
    mut senders: Query<&mut MessageSender<TalentStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok(selected) = talents.get(link.0) else {
                send_talent_error(conn_entity, &mut senders, "talents unavailable for player");
                continue;
            };
            send_talent_snapshot(
                conn_entity,
                &mut senders,
                build_talent_snapshot(selected),
                None,
                None,
            );
        }
    }
}

fn process_talent_apply_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<ApplyTalentChoice>),
        With<Authenticated>,
    >,
    mut talents: Query<&mut SelectedTalents>,
    mut senders: Query<&mut MessageSender<TalentStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            let Ok(mut selected) = talents.get_mut(link.0) else {
                send_talent_error(conn_entity, &mut senders, "talents unavailable for player");
                continue;
            };
            if !is_known_talent(request.talent_id) {
                send_talent_snapshot(
                    conn_entity,
                    &mut senders,
                    build_talent_snapshot(&selected),
                    None,
                    Some("unknown talent choice".into()),
                );
                continue;
            }
            if apply_talent_choice(&mut selected, request.talent_id) {
                send_talent_snapshot(
                    conn_entity,
                    &mut senders,
                    build_talent_snapshot(&selected),
                    Some("talent applied".into()),
                    None,
                );
            } else {
                send_talent_snapshot(
                    conn_entity,
                    &mut senders,
                    build_talent_snapshot(&selected),
                    None,
                    Some("talent already selected".into()),
                );
            }
        }
    }
}

fn process_talent_reset_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<ResetTalents>),
        With<Authenticated>,
    >,
    mut talents: Query<&mut SelectedTalents>,
    mut senders: Query<&mut MessageSender<TalentStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok(mut selected) = talents.get_mut(link.0) else {
                send_talent_error(conn_entity, &mut senders, "talents unavailable for player");
                continue;
            };
            let message = if reset_talents(&mut selected) {
                Some("talents reset".into())
            } else {
                Some("talents already clear".into())
            };
            send_talent_snapshot(
                conn_entity,
                &mut senders,
                build_talent_snapshot(&selected),
                message,
                None,
            );
        }
    }
}

fn send_talent_snapshot(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<TalentStateUpdate>>,
    snapshot: TalentSnapshot,
    message: Option<String>,
    error: Option<String>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<TalentChannel>(TalentStateUpdate {
        snapshot: Some(snapshot),
        message,
        error,
    });
}

fn send_talent_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<TalentStateUpdate>>,
    error: impl Into<String>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<TalentChannel>(TalentStateUpdate {
        snapshot: None,
        message: None,
        error: Some(error.into()),
    });
}

fn is_known_talent(talent_id: u32) -> bool {
    TALENT_DEFS.iter().any(|talent| talent.id == talent_id)
}

fn apply_talent_choice(selected: &mut SelectedTalents, talent_id: u32) -> bool {
    if selected.talent_ids.contains(&talent_id) {
        return false;
    }
    selected.talent_ids.push(talent_id);
    true
}

fn reset_talents(selected: &mut SelectedTalents) -> bool {
    let had_any = !selected.talent_ids.is_empty();
    selected.talent_ids.clear();
    had_any
}

pub(crate) fn build_talent_snapshot(selected: &SelectedTalents) -> TalentSnapshot {
    let active_spec = TALENT_DEFS
        .iter()
        .filter(|talent| selected.talent_ids.contains(&talent.id))
        .fold([0usize; SPEC_TABS.len()], |mut counts, talent| {
            counts[talent.spec_tab] += 1;
            counts
        })
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| **count)
        .map(|(index, _)| index)
        .unwrap_or(0);
    let talents = TALENT_DEFS
        .iter()
        .map(|talent| {
            let active = selected.talent_ids.contains(&talent.id);
            TalentNodeSnapshot {
                talent_id: talent.id,
                name: talent.name.to_string(),
                points_spent: u8::from(active),
                max_points: talent.max_points,
                active,
            }
        })
        .collect();
    let points_spent = selected.talent_ids.len() as u16;
    TalentSnapshot {
        spec_tabs: SPEC_TABS
            .iter()
            .enumerate()
            .map(|(index, name)| TalentSpecTabSnapshot {
                name: (*name).to_string(),
                active: index == active_spec,
            })
            .collect(),
        talents,
        points_remaining: MAX_TALENT_POINTS.saturating_sub(points_spent),
    }
}

#[cfg(test)]
#[path = "talent_tests.rs"]
mod tests;
