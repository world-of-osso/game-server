//! APL_LIVE_FACTS_FROM_OBSERVED_COMBAT: estimate target lifetimes from damage events.
use bevy::prelude::*;
use shared::components::Health;
use shared::protocol::{CombatLogEvent, CombatLogKind};
use std::collections::VecDeque;

const WINDOW_SECONDS: f64 = 10.0;
const MIN_SAMPLE_SECONDS: f64 = 1.0;
/// A finite value keeps APL arithmetic defined until a rate can be observed.
pub(crate) const UNKNOWN_TTD: f64 = 1_000_000_000.0;

#[derive(Component, Default)]
struct DamageHistory {
    samples: VecDeque<(f64, f64)>,
}

/// Start collecting only for enemies a follower actually engages.
pub(crate) fn track_target(world: &mut World, target: Entity) {
    if world.get::<DamageHistory>(target).is_none() {
        world.entity_mut(target).insert(DamageHistory::default());
    }
}

pub(crate) fn record_damage(world: &mut World, event: &CombatLogEvent) {
    if event.kind != CombatLogKind::Damage {
        return;
    }
    let Some(target) = event.target.and_then(Entity::try_from_bits) else {
        return;
    };
    if world.get::<DamageHistory>(target).is_none() {
        return;
    }
    let now = crate::spell_cast::now(world);
    let mut history = world
        .get_mut::<DamageHistory>(target)
        .expect("tracked target");
    let damage = event.amount.saturating_sub(event.overflow.max(0)).max(0);
    history.record(now, f64::from(damage));
}

impl DamageHistory {
    fn record(&mut self, now: f64, damage: f64) {
        while self
            .samples
            .front()
            .is_some_and(|(at, _)| *at < now - WINDOW_SECONDS)
        {
            self.samples.pop_front();
        }
        if damage > 0.0 {
            self.samples.push_back((now, damage));
        }
    }

    fn time_to_die(&self, now: f64, health: f64) -> f64 {
        let mut samples = self
            .samples
            .iter()
            .filter(|(at, _)| *at >= now - WINDOW_SECONDS);
        let Some((first, _)) = samples.next() else {
            return UNKNOWN_TTD;
        };
        let elapsed = now - first;
        if elapsed < MIN_SAMPLE_SECONDS {
            return UNKNOWN_TTD;
        }
        let damage: f64 = samples.map(|(_, amount)| amount).sum();
        if damage <= 0.0 {
            return UNKNOWN_TTD;
        }
        health.max(0.0) * elapsed / damage
    }
}

pub(crate) fn target_time_to_die(world: &World, target: Entity, now: f64) -> Result<f64, String> {
    let health = world
        .get::<Health>(target)
        .ok_or("native target lacks Health")?;
    Ok(world
        .get::<DamageHistory>(target)
        .map_or(UNKNOWN_TTD, |history| {
            history.time_to_die(now, f64::from(health.current))
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_follower_observed_damage_changes_live_apl_execute_selection() {
        use crate::simc_apl::Program;
        use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
        use std::time::Duration;

        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        let actor = world.spawn_empty().id();
        let target = world
            .spawn(Health {
                current: 1000.0,
                max: 1000.0,
            })
            .id();
        track_target(&mut world, target);
        let mut event = CombatLogEvent {
            source: Some(actor.to_bits()),
            target: Some(target.to_bits()),
            spell_id: Some(133),
            school_mask: 4,
            amount: 100,
            overflow: 0,
            absorbed: 0,
            resisted: 0,
            blocked: 0,
            crit: false,
            glancing: false,
            periodic: false,
            extra_spell_id: None,
            timestamp_unix_ms: 0,
            kind: CombatLogKind::Damage,
        };
        record_damage(&mut world, &event);
        let program = Program::parse(&[("default", "execute,if=target.time_to_die<10")]).unwrap();
        let bindings = Bindings::default();
        let targets = [target];
        let encounter = Encounter::Dungeon { started_at: 0.0 };
        let mut state = program.state();
        let facts = NativeFacts::new(&world, actor, &bindings, encounter, 1)
            .with_targets(Some(target), &targets);
        assert!(
            program
                .select("default", &facts, &mut state, |_| Ok(true))
                .unwrap()
                .is_none()
        );
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(2));
        world.get_mut::<Health>(target).unwrap().current = 800.0;
        event.amount = 200;
        record_damage(&mut world, &event);
        let facts = NativeFacts::new(&world, actor, &bindings, encounter, 1)
            .with_targets(Some(target), &targets);
        assert_eq!(
            program
                .select("default", &facts, &mut state, |_| Ok(true))
                .unwrap()
                .unwrap()
                .name,
            "execute"
        );
        assert_eq!(target_time_to_die(&world, target, 2.0).unwrap(), 8.0);
        assert_eq!(
            target_time_to_die(&world, target, 13.0).unwrap(),
            UNKNOWN_TTD
        );
    }
}
