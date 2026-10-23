use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::time::Duration;

use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::components::{Player, PresenceStatus};

const AUTO_AFK_DURATION: Duration = Duration::from_secs(300);

#[cfg(test)]
use std::sync::Mutex;
#[cfg(any(test, feature = "test-support"))]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(any(test, feature = "test-support"))]
pub(super) static AUTO_AFK_PLAYERS_VISITED: AtomicUsize = AtomicUsize::new(0);
#[cfg(any(test, feature = "test-support"))]
pub(super) static AUTO_AFK_TRANSITIONS: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
pub(super) static AUTO_AFK_TEST_LOCK: Mutex<()> = Mutex::new(());

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PresenceTracker {
    pub(crate) generation: u64,
    pub(crate) manual: Option<ManualPresence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct AutoAfkDeadline {
    due_at: Duration,
    player: Entity,
    generation: u64,
}

#[derive(Resource, Default)]
pub(crate) struct AutoAfkScheduler {
    deadlines: BinaryHeap<Reverse<AutoAfkDeadline>>,
    next_generation: u64,
}

impl AutoAfkScheduler {
    fn next_generation(&mut self) -> u64 {
        self.next_generation = self.next_generation.wrapping_add(1).max(1);
        self.next_generation
    }

    fn invalidate(&mut self, tracker: &mut PresenceTracker) {
        tracker.generation = self.next_generation();
    }

    fn schedule(&mut self, player: Entity, tracker: &mut PresenceTracker, now: Duration) {
        self.invalidate(tracker);
        self.deadlines.push(Reverse(AutoAfkDeadline {
            due_at: now + AUTO_AFK_DURATION,
            player,
            generation: tracker.generation,
        }));
    }

    fn pop_due(&mut self, now: Duration) -> Option<AutoAfkDeadline> {
        let Reverse(deadline) = self.deadlines.peek().copied()?;
        if deadline.due_at > now {
            return None;
        }
        self.deadlines.pop().map(|Reverse(deadline)| deadline)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManualPresence {
    Afk,
    Dnd,
}

pub(super) fn process_due_auto_afk(
    time: Res<Time>,
    mut scheduler: ResMut<AutoAfkScheduler>,
    dirty_queue: Option<Res<ReplicationDirtyQueue>>,
    mut players: Query<(&mut PresenceStatus, &PresenceTracker), With<Player>>,
) {
    while let Some(deadline) = scheduler.pop_due(time.elapsed()) {
        #[cfg(any(test, feature = "test-support"))]
        AUTO_AFK_PLAYERS_VISITED.fetch_add(1, Ordering::SeqCst);

        let Ok((mut status, tracker)) = players.get_mut(deadline.player) else {
            continue;
        };
        if tracker.generation != deadline.generation || tracker.manual.is_some() {
            continue;
        }
        if *status == PresenceStatus::Afk {
            continue;
        }
        *status = PresenceStatus::Afk;
        #[cfg(any(test, feature = "test-support"))]
        AUTO_AFK_TRANSITIONS.fetch_add(1, Ordering::SeqCst);
        mark_presence_changed(dirty_queue.as_deref(), deadline.player);
    }
}

pub(super) fn schedule_auto_afk(
    player_entity: Entity,
    tracker: &mut PresenceTracker,
    now: Duration,
    scheduler: &mut AutoAfkScheduler,
) {
    scheduler.schedule(player_entity, tracker, now);
}

pub fn mark_player_activity(
    player_entity: Entity,
    players: &mut Query<(&mut PresenceStatus, &mut PresenceTracker), With<Player>>,
    scheduler: &mut AutoAfkScheduler,
    now: Duration,
    dirty_queue: Option<&ReplicationDirtyQueue>,
) {
    let Ok((mut status, mut tracker)) = players.get_mut(player_entity) else {
        return;
    };
    let next_status = match tracker.manual {
        Some(ManualPresence::Dnd) => {
            scheduler.invalidate(&mut tracker);
            PresenceStatus::Dnd
        }
        Some(ManualPresence::Afk) => {
            tracker.manual = None;
            schedule_auto_afk(player_entity, &mut tracker, now, scheduler);
            PresenceStatus::Online
        }
        None => {
            schedule_auto_afk(player_entity, &mut tracker, now, scheduler);
            PresenceStatus::Online
        }
    };
    if *status != next_status {
        *status = next_status;
        mark_presence_changed(dirty_queue, player_entity);
    }
}

pub(super) fn set_presence(
    player_entity: Entity,
    requested: PresenceStatus,
    players: &mut Query<(&mut PresenceStatus, &mut PresenceTracker), With<Player>>,
    scheduler: &mut AutoAfkScheduler,
    now: Duration,
    dirty_queue: Option<&ReplicationDirtyQueue>,
) -> Result<String, String> {
    let Ok((mut status, mut tracker)) = players.get_mut(player_entity) else {
        return Err("presence unavailable for player".into());
    };
    tracker.manual = match requested {
        PresenceStatus::Online | PresenceStatus::Offline => None,
        PresenceStatus::Afk => Some(ManualPresence::Afk),
        PresenceStatus::Dnd => Some(ManualPresence::Dnd),
    };
    if tracker.manual.is_some() {
        scheduler.invalidate(&mut tracker);
    } else {
        schedule_auto_afk(player_entity, &mut tracker, now, scheduler);
    }
    let next_status = match requested {
        PresenceStatus::Offline => PresenceStatus::Online,
        other => other,
    };
    if *status != next_status {
        *status = next_status;
        mark_presence_changed(dirty_queue, player_entity);
    }
    Ok(format!(
        "presence set to {}",
        match *status {
            PresenceStatus::Online => "online",
            PresenceStatus::Afk => "afk",
            PresenceStatus::Dnd => "dnd",
            PresenceStatus::Offline => "offline",
        }
    ))
}

fn mark_presence_changed(dirty_queue: Option<&ReplicationDirtyQueue>, player_entity: Entity) {
    if let Some(dirty_queue) = dirty_queue {
        dirty_queue.mark(player_entity);
    }
}
