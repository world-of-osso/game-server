//! APL_LIVE_FACTS_FROM_OBSERVED_COMBAT: combat events landing on units an APL watches.
use bevy::prelude::*;
use shared::protocol::{CombatLogEvent, CombatLogKind};
use std::collections::VecDeque;

const WINDOW_SECONDS: f64 = 10.0;

/// Damage and heal events landing on a unit an APL watches, kept for `WINDOW_SECONDS`.
#[derive(Component, Default)]
pub struct ObservedEvents {
    next: u64,
    events: VecDeque<(u64, f64, CombatLogEvent)>,
}

impl ObservedEvents {
    /// Events numbered `seen` or later, and the number to pass on the next read.
    pub fn since(&self, seen: u64) -> (impl Iterator<Item = &CombatLogEvent>, u64) {
        let events = self
            .events
            .iter()
            .filter(move |(number, _, _)| *number >= seen)
            .map(|(_, _, event)| event);
        (events, self.next)
    }

    /// Whether recent observed damage, including absorbed damage, matches a school.
    pub fn took_school_damage_since(&self, since: f64, school_mask: u32) -> bool {
        self.events.iter().any(|(_, at, event)| {
            *at >= since
                && event.kind == CombatLogKind::Damage
                && event.school_mask & school_mask != 0
                && (event.amount > 0 || event.absorbed > 0)
        })
    }
}

/// Start collecting the events that land on `unit`; existing history is kept.
pub fn observe_unit(world: &mut World, unit: Entity) {
    if world.get::<ObservedEvents>(unit).is_none() {
        world.entity_mut(unit).insert(ObservedEvents::default());
    }
}

pub(crate) fn record_event(world: &mut World, event: &CombatLogEvent) {
    let Some(target) = event.target.and_then(Entity::try_from_bits) else {
        return;
    };
    if matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Heal) {
        record_observed(world, target, event);
    }
}

fn record_observed(world: &mut World, target: Entity, event: &CombatLogEvent) {
    let now = crate::spell_cast::now(world);
    let Some(mut observed) = world.get_mut::<ObservedEvents>(target) else {
        return;
    };
    while observed
        .events
        .front()
        .is_some_and(|(_, at, _)| *at < now - WINDOW_SECONDS)
    {
        observed.events.pop_front();
    }
    let number = observed.next;
    observed.next += 1;
    observed.events.push_back((number, now, event.clone()));
}
