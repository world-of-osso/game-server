//! SPELL_AURA_TRIGGER_SPELL_ON_HEALTH_PCT (468): TC `Unit::TriggerOnHealthChangeAuras`
//! casts `EffectTriggerSpell` on the unit when health crosses `amount`% of max
//! (MiscValue 0 rising to/above, 1 falling to/below); applying the aura already
//! past the threshold casts once (`AuraEffect::HandleTriggerSpellOnHealthPercent`).
//! Health is observed once per fixed tick, so a change and its reversal within one
//! tick is not seen.
use bevy::prelude::*;
use shared::components::Health;

use super::ActiveAuras;

pub const TRIGGER_SPELL_ON_HEALTH_PCT: u32 = 468;
const ABOVE: i32 = 0;
const BELOW: i32 = 1;

/// Health the unit had at the previous observation, per aura instance/effect.
#[derive(Component, Debug, Default)]
pub(crate) struct ObservedHealth(Vec<(u32, u8, f32)>);

struct Watch {
    instance: u32,
    index: u8,
    pct: f32,
    direction: i32,
    trigger: u32,
}

/// Indexed at aura membership changes, not by scanning every world unit.
#[derive(Component)]
#[require(ObservedHealth)]
pub(super) struct HealthPercentWatch;

pub(super) fn reconcile(world: &mut World, unit: Entity) {
    let watches = world
        .get::<ActiveAuras>(unit)
        .is_some_and(|auras| auras.has_type(TRIGGER_SPELL_ON_HEALTH_PCT));
    let indexed = world.get::<HealthPercentWatch>(unit).is_some();
    match (watches, indexed) {
        (true, false) => {
            world.entity_mut(unit).insert(HealthPercentWatch);
        }
        (false, true) => {
            world
                .entity_mut(unit)
                .remove::<(HealthPercentWatch, ObservedHealth)>();
        }
        _ => {}
    }
}

type HealthWatches =
    QueryState<(Entity, &'static ActiveAuras, &'static Health), With<HealthPercentWatch>>;

pub(super) fn trigger_on_health_pct(world: &mut World, watched: &mut HealthWatches) {
    let units: Vec<(Entity, f32, f32, Vec<Watch>)> = watched
        .iter(world)
        .filter_map(|(unit, auras, health)| {
            let watches: Vec<_> = auras
                .iter()
                .flat_map(|aura| {
                    aura.effects
                        .iter()
                        .filter(|effect| effect.aura_type == TRIGGER_SPELL_ON_HEALTH_PCT)
                        .map(|effect| Watch {
                            instance: aura.instance_id,
                            index: effect.index,
                            pct: effect.amount,
                            direction: effect.misc_values[0],
                            trigger: effect.trigger_spell,
                        })
                })
                .collect();
            (!watches.is_empty()).then_some((unit, health.current, health.max, watches))
        })
        .collect();
    for (unit, current, max, watches) in units {
        let previous = world
            .get::<ObservedHealth>(unit)
            .map(|observed| observed.0.clone())
            .unwrap_or_default();
        let mut next = Vec::with_capacity(watches.len());
        for watch in watches {
            let threshold = max * watch.pct / 100.0;
            let old = previous
                .iter()
                .find(|(instance, index, _)| *instance == watch.instance && *index == watch.index)
                .map(|(_, _, health)| *health);
            if fires(watch.direction, old, current, threshold) && watch.trigger != 0 {
                crate::spell_cast::cast_triggered(world, unit, watch.trigger, Some(unit), 1);
            }
            next.push((watch.instance, watch.index, current));
        }
        if let Ok(mut entity) = world.get_entity_mut(unit) {
            entity.insert(ObservedHealth(next));
        }
    }
}

fn fires(direction: i32, old: Option<f32>, new: f32, threshold: f32) -> bool {
    match (direction, old) {
        (ABOVE, None) => new > threshold,
        (BELOW, None) => new < threshold,
        (ABOVE, Some(old)) => new >= threshold && old <= threshold && old != new,
        (BELOW, Some(old)) => new <= threshold && old >= threshold && old != new,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_percent_idle_population_fits_one_frame_even_during_catch_up() {
        let mut app = App::new();
        app.add_systems(Update, trigger_on_health_pct);
        app.world_mut().spawn_batch((0..250_000).map(|_| {
            (
                ActiveAuras::default(),
                Health {
                    current: 1000.0,
                    max: 1000.0,
                },
            )
        }));
        app.update();
        let fastest = (0..3)
            .map(|_| {
                let start = std::time::Instant::now();
                for _ in 0..5 {
                    app.update();
                }
                start.elapsed()
            })
            .min()
            .unwrap();
        println!("250000 idle units, five catch-up observations: {fastest:?}");
        assert!(
            fastest < std::time::Duration::from_millis(50),
            "idle health watchers consume the entire frame budget: {fastest:?}"
        );
    }

    #[test]
    fn below_fires_once_on_the_crossing_and_on_apply_only_when_already_below() {
        assert!(fires(1, None, 150.0, 200.0));
        assert!(!fires(1, None, 250.0, 200.0));
        assert!(fires(1, Some(250.0), 200.0, 200.0));
        assert!(!fires(1, Some(190.0), 150.0, 200.0));
        assert!(!fires(1, Some(150.0), 150.0, 200.0));
        assert!(fires(0, Some(150.0), 210.0, 200.0));
        assert!(!fires(0, Some(210.0), 300.0, 200.0));
    }
}
