//! TrinityCore `EventMap` (common/Utilities/EventMap.cpp): script events keyed
//! by the time they are due, on a clock only `update` advances. Events due at
//! the same time run in the order they were scheduled (`std::multimap`).
//! Phases, groups and series are not ported: no Stockade script uses them.

use rand::Rng;

#[derive(Debug, Default, Clone)]
pub struct EventMap {
    /// `_time`, ms.
    time: u64,
    /// `_eventMap`: (due time ms, event id), sorted by due time, stable.
    events: Vec<(u64, u32, u64)>,
    revision: u64,
    /// `_lastEvent`: what `repeat` schedules again.
    last_event: u32,
}

/// `randtime(min, max)`: uniform in `[min, max]` ms.
pub fn rand_ms(min_ms: u64, max_ms: u64) -> u64 {
    if max_ms <= min_ms {
        return min_ms;
    }
    rand::thread_rng().gen_range(min_ms..=max_ms)
}

impl EventMap {
    /// `EventMap::Reset`: no events, the clock back at zero.
    pub fn reset(&mut self) {
        let revision = self.revision.wrapping_add(1);
        *self = Self {
            revision,
            ..Self::default()
        };
    }

    /// `EventMap::Update`.
    pub fn update(&mut self, diff_ms: u64) {
        self.time += diff_ms;
    }

    fn insert(&mut self, due: u64, event_id: u32) {
        let at = self.events.partition_point(|&(time, _, _)| time <= due);
        self.events
            .insert(at, (due, event_id, due.saturating_sub(self.time)));
        self.revision = self.revision.wrapping_add(1);
    }

    /// `ScheduleEvent(eventId, time)`.
    pub fn schedule(&mut self, event_id: u32, delay_ms: u64) {
        self.insert(self.time + delay_ms, event_id);
    }

    /// `ScheduleEvent(eventId, minTime, maxTime)`.
    pub fn schedule_range(&mut self, event_id: u32, min_ms: u64, max_ms: u64) {
        self.schedule(event_id, rand_ms(min_ms, max_ms));
    }

    /// `Repeat(time)`: the last executed event again, `delay_ms` from now.
    pub fn repeat(&mut self, delay_ms: u64) {
        self.insert(self.time + delay_ms, self.last_event);
    }

    /// `Repeat(minTime, maxTime)`.
    pub fn repeat_range(&mut self, min_ms: u64, max_ms: u64) {
        self.repeat(rand_ms(min_ms, max_ms));
    }

    /// `ExecuteEvent`: the first due event, removed; `None` (TrinityCore 0) when
    /// none is due.
    pub fn execute_event(&mut self) -> Option<u32> {
        let &(due, event_id, _) = self.events.first()?;
        if due > self.time {
            return None;
        }
        self.events.remove(0);
        self.revision = self.revision.wrapping_add(1);
        self.last_event = event_id;
        Some(event_id)
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Actual selected schedule: event id, remaining ms and selected duration.
    pub fn timers(&self) -> Vec<(u32, u32, u32)> {
        self.events
            .iter()
            .map(|&(due, id, duration)| {
                (
                    id,
                    due.saturating_sub(self.time).min(u64::from(u32::MAX)) as u32,
                    duration.min(u64::from(u32::MAX)) as u32,
                )
            })
            .collect()
    }

    /// Scheduled event ids in due order (tests).
    #[cfg(test)]
    pub fn pending(&self) -> Vec<u32> {
        self.events.iter().map(|&(_, id, _)| id).collect()
    }
}
