//! TrinityCore `EventMap` (common/Utilities/EventMap.cpp): script events keyed
//! by the time they are due, on a clock only `update` advances. Events due at
//! the same time run in the order they were scheduled (`std::multimap`).
//! Phases, groups and series are not ported: no Stockade script uses them.

use crate::random as rand;
use rand::Rng;

#[derive(Debug, Default, Clone)]
pub struct EventMap {
    /// `_time`, ms.
    time: u64,
    /// `_eventMap`: (due time ms, event id), sorted by due time, stable.
    events: Vec<(u64, u32)>,
    /// `_lastEvent`: what `repeat` schedules again.
    last_event: u32,
    last_due: u64,
    #[cfg(test)]
    rng: Option<rand::rngs::StdRng>,
}

/// `randtime(min, max)`: uniform in `[min, max]` ms.
pub fn rand_ms(min_ms: u64, max_ms: u64) -> u64 {
    if max_ms <= min_ms {
        return min_ms;
    }
    rand::thread_rng().gen_range(min_ms..=max_ms)
}

impl EventMap {
    #[cfg(test)]
    pub fn seeded(seed: u64) -> Self {
        use rand::SeedableRng;
        Self {
            rng: Some(rand::rngs::StdRng::seed_from_u64(seed)),
            ..Default::default()
        }
    }

    fn roll_delay(&mut self, min_ms: u64, max_ms: u64) -> u64 {
        #[cfg(test)]
        if let Some(rng) = &mut self.rng {
            return if max_ms <= min_ms {
                min_ms
            } else {
                rng.gen_range(min_ms..=max_ms)
            };
        }
        rand_ms(min_ms, max_ms)
    }

    /// `EventMap::Reset`: no events, the clock back at zero.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// `EventMap::Update`.
    pub fn update(&mut self, diff_ms: u64) {
        self.time += diff_ms;
    }

    fn insert(&mut self, due: u64, event_id: u32) {
        let at = self.events.partition_point(|&(time, _)| time <= due);
        self.events.insert(at, (due, event_id));
    }

    /// `ScheduleEvent(eventId, time)`.
    pub fn schedule(&mut self, event_id: u32, delay_ms: u64) {
        self.insert(self.time + delay_ms, event_id);
    }

    /// `CancelEvent`: remove all queued instances without changing clock/repeat state.
    pub fn cancel(&mut self, event_id: u32) {
        self.events.retain(|&(_, id)| id != event_id);
    }

    /// `RescheduleEvent`: cancel every queued event of this id before scheduling.
    pub fn reschedule(&mut self, event_id: u32, delay_ms: u64) {
        self.cancel(event_id);
        self.schedule(event_id, delay_ms);
    }

    /// `ScheduleEvent(eventId, minTime, maxTime)`.
    pub fn schedule_range(&mut self, event_id: u32, min_ms: u64, max_ms: u64) {
        let delay = self.roll_delay(min_ms, max_ms);
        self.schedule(event_id, delay);
    }

    /// `Repeat(time)`: the last executed event again, `delay_ms` from now.
    pub fn repeat(&mut self, delay_ms: u64) {
        self.insert(self.time + delay_ms, self.last_event);
    }

    /// `Repeat(minTime, maxTime)`.
    pub fn repeat_range(&mut self, min_ms: u64, max_ms: u64) {
        let delay = self.roll_delay(min_ms, max_ms);
        self.repeat(delay);
    }

    /// TaskScheduler::TaskContext::Repeat adds to the task's original deadline,
    /// not the dispatch time. EventMap's existing repeat methods stay unchanged.
    pub fn repeat_from_due_range(&mut self, min_ms: u64, max_ms: u64) {
        let delay = self.roll_delay(min_ms, max_ms);
        self.insert(self.last_due + delay, self.last_event);
    }

    /// `ExecuteEvent`: the first due event, removed; `None` (TrinityCore 0) when
    /// none is due.
    pub fn execute_event(&mut self) -> Option<u32> {
        let &(due, event_id) = self.events.first()?;
        if due > self.time {
            return None;
        }
        self.events.remove(0);
        self.last_event = event_id;
        self.last_due = due;
        Some(event_id)
    }

    /// Seed only fixture randomness, preserving the clock and pending source events.
    #[cfg(test)]
    pub(crate) fn seed_rng(&mut self, seed: u64) {
        use rand::SeedableRng;
        self.rng = Some(rand::rngs::StdRng::seed_from_u64(seed));
    }

    /// Exact EventMap clock for opt-in fixture evidence; never advances it.
    #[cfg(test)]
    pub(crate) fn elapsed_ms(&self) -> u64 {
        self.time
    }

    /// Scheduled event ids in due order (tests).
    #[cfg(test)]
    pub fn pending(&self) -> Vec<u32> {
        self.events.iter().map(|&(_, id)| id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::EventMap;

    #[test]
    fn event_map_cancel_removes_all_matching_deadlines_and_preserves_repeat() {
        let mut events = EventMap::default();
        events.schedule(3, 500);
        events.schedule(1, 750);
        events.schedule(1, 900);
        events.schedule(2, 800);
        events.schedule(4, 900);
        events.update(500);
        assert_eq!(events.execute_event(), Some(3));
        events.cancel(1);
        events.cancel(2);
        events.repeat(500);
        events.update(399);
        assert_eq!(events.execute_event(), None);
        events.update(1);
        assert_eq!(events.execute_event(), Some(4));
        events.update(100);
        assert_eq!(events.execute_event(), Some(3));
        assert_eq!(events.execute_event(), None);
        events.schedule(1, 10);
        events.schedule(1, 20);
        events.reschedule(1, 30);
        events.update(29);
        assert_eq!(events.execute_event(), None);
        events.update(1);
        assert_eq!(events.execute_event(), Some(1));
        assert_eq!(events.execute_event(), None);
    }
}
