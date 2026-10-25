//! Quest wall clock and expiration. Uses the realm's existing UTC instance reset schedule.
use crate::auth::InWorld;
use crate::maps::difficulty::ServerClock;
use crate::persistence::GameDatabase;
use crate::persistence::quest_store::{load_quest_log, save_quest_log};
use crate::quest::QuestDirty;
use crate::quest_data::QuestDb;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{QuestChannel, QuestFailed, QuestFailedReason};

pub(crate) fn expire_log(
    quests: &QuestDb,
    log: &mut crate::persistence::quest_store::QuestLogData,
    now: i64,
) -> Vec<u32> {
    let mut expired = Vec::new();
    let ended_tasks: Vec<_> = log
        .active
        .iter()
        .filter(|active| {
            quests
                .world_quests
                .iter()
                .any(|q| q.quest_id == active.quest_id)
                && active.expires_at.is_none_or(|end| now >= end)
        })
        .map(|active| active.quest_id)
        .collect();
    for quest_id in ended_tasks {
        log.remove(quest_id);
        expired.push(quest_id);
    }
    for active in &mut log.active {
        let timed = quests
            .quests
            .get(&active.quest_id)
            .is_some_and(|q| q.time_allowed > 0);
        // Legacy timed logs have no recoverable start time. Fail instead of granting a new timer.
        if !active.failed && timed && active.expires_at.is_none_or(|end| now >= end) {
            active.failed = true;
            expired.push(active.quest_id);
        }
    }
    expired
}

pub(crate) fn refresh(
    clock: Res<ServerClock>,
    rotation: Res<crate::world_quests::Rotation>,
    mut quests: Option<ResMut<QuestDb>>,
    db: Option<Res<GameDatabase>>,
    connections: Query<(Entity, &InWorld)>,
    mut failures: Query<&mut MessageSender<QuestFailed>>,
    mut dirty: ResMut<QuestDirty>,
) {
    let (Some(quests), Some(db)) = (quests.as_mut(), db) else {
        return;
    };
    let now = clock.unix();
    if quests.now == Some(now) {
        return;
    }
    let reset_crossed = quests.now.is_some_and(|old| {
        crate::maps::locks::next_reset_time(crate::maps::store::ResetInterval::Daily, old) <= now
    });
    quests.now = Some(now);
    quests.world_quest_windows = rotation
        .active(&quests.world_quests, now)
        .into_iter()
        .map(|window| (window.quest_id, window))
        .collect();
    for (connection, owner) in &connections {
        let mut log = load_quest_log(&db.db, owner.character_id);
        let expired = expire_log(quests, &mut log, now);
        if !expired.is_empty() {
            save_quest_log(&db.db, owner.character_id, &log);
            dirty.0.insert(owner.character_id);
            if let Ok(mut sender) = failures.get_mut(connection) {
                for quest_id in expired {
                    sender.send::<QuestChannel>(QuestFailed {
                        quest_id,
                        reason: QuestFailedReason::TimedOut,
                    });
                }
            }
        }
        if reset_crossed {
            dirty.0.insert(owner.character_id);
        }
    }
}
