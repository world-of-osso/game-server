//! Incoming damage observed at the native combat-log boundary.
use bevy::prelude::*;
use shared::protocol::{CombatLogEvent, CombatLogKind};
use std::collections::VecDeque;

use crate::maps::InMap;

const DAMAGE_WINDOW_SECONDS: f64 = 10.0;
const MIN_RATE_WINDOW_SECONDS: f64 = 1.0;

#[derive(Component)]
struct IncomingDamage {
    map: InMap,
    started: f64,
    samples: VecDeque<(f64, f32)>,
}

pub(super) fn record_damage(world: &mut World, event: &CombatLogEvent) {
    if !matches!(
        event.kind,
        CombatLogKind::Damage | CombatLogKind::Environmental(_)
    ) {
        return;
    }
    let Some(target) = event.target.and_then(Entity::try_from_bits) else {
        return;
    };
    let Some(map) = world.get::<InMap>(target).copied() else {
        return;
    };
    let amount = effective_health_loss(world, target, event);
    if amount <= 0.0 {
        return;
    }
    let now = crate::auras::aura_now(world);
    let same_copy = world
        .get::<IncomingDamage>(target)
        .is_some_and(|history| history.map == map);
    if !same_copy {
        world.entity_mut(target).insert(IncomingDamage {
            map,
            started: now,
            samples: VecDeque::new(),
        });
    }
    let mut history = world.get_mut::<IncomingDamage>(target).unwrap();
    while history
        .samples
        .front()
        .is_some_and(|(at, _)| *at < now - DAMAGE_WINDOW_SECONDS)
    {
        history.samples.pop_front();
    }
    if history.samples.is_empty() {
        history.started = now;
    }
    history.samples.push_back((now, amount));
}

fn effective_health_loss(world: &World, target: Entity, event: &CombatLogEvent) -> f32 {
    let amount = event.amount.saturating_sub(event.overflow.max(0)).max(0) as f32;
    let Some(source) = event.source.and_then(Entity::try_from_bits) else {
        return amount;
    };
    let source = crate::level_scaling::UnitScaling::of(world, source);
    let target = crate::level_scaling::UnitScaling::of(world, target);
    crate::level_scaling::health_loss(&source, &target, amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_effective_damage_from_the_combat_log() {
        let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[], 1).unwrap();
        let event = CombatLogEvent {
            kind: CombatLogKind::Damage,
            source: None,
            target: Some(h.caster.to_bits()),
            amount: 125,
            overflow: 25,
            spell_id: None,
            school_mask: 1,
            absorbed: 0,
            resisted: 0,
            blocked: 0,
            crit: false,
            glancing: false,
            periodic: false,
            extra_spell_id: None,
            timestamp_unix_ms: 0,
        };
        crate::followers::pull_history::record_event(h.app.world_mut(), &event);
        let world = h.app.world();
        assert_eq!(
            incoming_rate(world, h.caster, crate::auras::aura_now(world)),
            100.0
        );
    }
}

pub(super) fn incoming_rate(world: &World, target: Entity, now: f64) -> f32 {
    let Some(history) = world.get::<IncomingDamage>(target) else {
        return 0.0;
    };
    if world.get::<InMap>(target) != Some(&history.map) {
        return 0.0;
    }
    let amount: f32 = history
        .samples
        .iter()
        .filter(|(at, _)| *at >= now - DAMAGE_WINDOW_SECONDS)
        .map(|(_, amount)| amount)
        .sum();
    let seconds = (now - history.started).clamp(MIN_RATE_WINDOW_SECONDS, DAMAGE_WINDOW_SECONDS);
    amount / seconds as f32
}
