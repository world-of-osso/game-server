use std::collections::BTreeSet;

use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use shared::components::Level;
use shared::protocol::{
    AchievementChannel, AchievementProgressSnapshot, AchievementSnapshot, AchievementStateUpdate,
    AchievementToastSnapshot,
};

use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{self, GameDatabase};

const LEVEL_ACHIEVEMENTS: &[LevelAchievementDef] = &[
    LevelAchievementDef {
        id: 1,
        name: "Level 10",
        level: 10,
        points: 10,
    },
    LevelAchievementDef {
        id: 2,
        name: "Level 20",
        level: 20,
        points: 10,
    },
    LevelAchievementDef {
        id: 3,
        name: "Level 40",
        level: 40,
        points: 10,
    },
    LevelAchievementDef {
        id: 4,
        name: "Level 60",
        level: 60,
        points: 10,
    },
    LevelAchievementDef {
        id: 5,
        name: "Level 70",
        level: 70,
        points: 10,
    },
    LevelAchievementDef {
        id: 6,
        name: "Level 80",
        level: 80,
        points: 10,
    },
];

#[derive(Component, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PlayerAchievementState {
    pub earned_ids: BTreeSet<u32>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct AchievementTracker {
    last_processed_level: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LevelAchievementDef {
    id: u32,
    name: &'static str,
    level: u8,
    points: u32,
}

pub struct AchievementPlugin;

impl Plugin for AchievementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                send_initial_achievement_snapshot,
                process_level_achievement_updates,
            )
                .chain(),
        );
    }
}

type NewInWorldConnection<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static PlayerLink, &'static InWorld),
    (With<Authenticated>, Added<InWorld>),
>;

fn send_initial_achievement_snapshot(
    mut commands: Commands,
    connections: NewInWorldConnection,
    mut achievements: Query<(&Level, &mut PlayerAchievementState)>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<AchievementStateUpdate>>,
) {
    for (conn_entity, link, in_world) in &connections {
        let Ok((level, mut state)) = achievements.get_mut(link.0) else {
            send_achievement_error(
                conn_entity,
                &mut senders,
                "achievements unavailable for player",
            );
            continue;
        };
        let earned_before = state.earned_ids.len();
        sync_level_achievements_for_snapshot(&mut state, level.0);
        if state.earned_ids.len() != earned_before
            && let Some(db) = db.as_ref()
        {
            persistence::save_achievements(&db.db, in_world.character_id, &state);
        }
        send_achievement_update(
            conn_entity,
            build_achievement_snapshot(level.0, &state),
            None,
            Some("achievements loaded".into()),
            None,
            &mut senders,
        );
        commands.entity(link.0).insert(AchievementTracker {
            last_processed_level: level.0,
        });
    }
}

fn process_level_achievement_updates(
    connections: Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    mut achievements: Query<
        (
            Entity,
            &Level,
            &mut PlayerAchievementState,
            &mut AchievementTracker,
        ),
        Changed<Level>,
    >,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<AchievementStateUpdate>>,
) {
    let Some(db) = db else { return };
    for (player_entity, level, mut state, mut tracker) in &mut achievements {
        if tracker.last_processed_level == level.0 {
            continue;
        }
        let Some((conn_entity, character_id)) = connection_for_player(&connections, player_entity)
        else {
            continue;
        };
        tracker.last_processed_level = level.0;
        let completed = update_level_achievements(&mut state, level.0);
        let snapshot = build_achievement_snapshot(level.0, &state);
        if !completed.is_empty() {
            persistence::save_achievements(&db.db, character_id, &state);
        }
        send_level_updates(conn_entity, snapshot, completed, &mut senders);
    }
}

fn send_level_updates(
    conn_entity: Entity,
    snapshot: AchievementSnapshot,
    completed: Vec<AchievementToastSnapshot>,
    senders: &mut Query<&mut MessageSender<AchievementStateUpdate>>,
) {
    if completed.is_empty() {
        send_achievement_update(
            conn_entity,
            snapshot,
            None,
            Some("achievement progress updated".into()),
            None,
            senders,
        );
        return;
    }

    for completion in completed {
        send_achievement_update(
            conn_entity,
            snapshot.clone(),
            Some(completion),
            Some("achievement completed".into()),
            None,
            senders,
        );
    }
}

fn connection_for_player(
    connections: &Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    player_entity: Entity,
) -> Option<(Entity, u64)> {
    connections
        .iter()
        .find_map(|(conn_entity, link, in_world)| {
            (link.0 == player_entity).then_some((conn_entity, in_world.character_id))
        })
}

fn sync_level_achievements_for_snapshot(state: &mut PlayerAchievementState, level: u8) {
    for achievement in LEVEL_ACHIEVEMENTS {
        if level >= achievement.level {
            state.earned_ids.insert(achievement.id);
        }
    }
}

fn update_level_achievements(
    state: &mut PlayerAchievementState,
    level: u8,
) -> Vec<AchievementToastSnapshot> {
    let mut completed = Vec::new();
    for achievement in LEVEL_ACHIEVEMENTS {
        if level < achievement.level || state.earned_ids.contains(&achievement.id) {
            continue;
        }
        state.earned_ids.insert(achievement.id);
        completed.push(AchievementToastSnapshot {
            achievement_id: achievement.id,
            name: achievement.name.into(),
            points: achievement.points,
        });
    }
    completed
}

fn build_achievement_snapshot(level: u8, state: &PlayerAchievementState) -> AchievementSnapshot {
    AchievementSnapshot {
        earned_ids: state.earned_ids.iter().copied().collect(),
        progress: LEVEL_ACHIEVEMENTS
            .iter()
            .map(|achievement| AchievementProgressSnapshot {
                achievement_id: achievement.id,
                current: u32::from(level.min(achievement.level)),
                required: u32::from(achievement.level),
                completed: state.earned_ids.contains(&achievement.id),
            })
            .collect(),
    }
}

fn send_achievement_update(
    conn_entity: Entity,
    snapshot: AchievementSnapshot,
    completed: Option<AchievementToastSnapshot>,
    message: Option<String>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<AchievementStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<AchievementChannel>(AchievementStateUpdate {
        snapshot: Some(snapshot),
        completed,
        message,
        error,
    });
}

fn send_achievement_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<AchievementStateUpdate>>,
    error: impl Into<String>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<AchievementChannel>(AchievementStateUpdate {
        snapshot: None,
        completed: None,
        message: None,
        error: Some(error.into()),
    });
}

#[cfg(test)]
#[path = "achievement_tests.rs"]
mod tests;
