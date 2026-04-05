use bevy::prelude::*;
use shared::components::{Health, Level, Mana, Player, UnitStats};
use shared::formulas;

pub struct StatsPlugin;

impl Plugin for StatsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, update_derived_stats);
    }
}

type StatsQuery = (
    &'static Player,
    &'static UnitStats,
    &'static Level,
    &'static mut Health,
    Option<&'static mut Mana>,
);

type StatsFilter = Or<(Changed<UnitStats>, Changed<Level>)>;

/// Compute new current value preserving the current-to-max ratio.
fn rescaled_current(current: f32, old_max: f32, new_max: f32) -> f32 {
    let ratio = if old_max > 0.0 { current / old_max } else { 1.0 };
    (new_max * ratio).min(new_max)
}

/// Recalculate Health.max and Mana.max when UnitStats or Level change.
///
/// Mirrors AzerothCore `StatSystem.cpp:UpdateAllStats()`.
fn update_derived_stats(mut query: Query<StatsQuery, StatsFilter>) {
    for (player, stats, level, mut health, mana) in &mut query {
        let lv = level.0;
        let class = player.class;

        if let Some(new_max) = formulas::max_health(class, lv, stats.stamina) {
            health.current = rescaled_current(health.current, health.max, new_max);
            health.max = new_max;
        }

        if let Some(mut mana) = mana
            && let Some(new_max) = formulas::max_mana(class, lv, stats.intellect)
        {
            mana.current = rescaled_current(mana.current, mana.max, new_max);
            mana.max = new_max;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use shared::components::CombatRatings;

    fn spawn_player(world: &mut World, class: u8, level: u8, stamina: f32, intellect: f32) -> Entity {
        world.spawn((
            Player {
                name: "Test".into(),
                race: 1,
                class,
                appearance: Default::default(),
            },
            UnitStats {
                stamina,
                intellect,
                ..Default::default()
            },
            CombatRatings::default(),
            Level(level),
            Health {
                current: 100.0,
                max: 100.0,
            },
        )).id()
    }

    #[test]
    fn update_stats_recalculates_health() {
        let mut world = World::new();
        let entity = spawn_player(&mut world, 1, 80, 159.0, 0.0);

        world.run_system_once(update_derived_stats).unwrap();

        let health = world.get::<Health>(entity).unwrap();
        // Warrior L80: base 8121 + 159 * 20 = 11301
        assert!((health.max - 11301.0).abs() < 1.0);
    }

    #[test]
    fn update_stats_preserves_health_ratio() {
        let mut world = World::new();
        let entity = spawn_player(&mut world, 1, 80, 159.0, 0.0);

        // Set current to 50% of max before recalc
        world.get_mut::<Health>(entity).unwrap().current = 50.0;

        world.run_system_once(update_derived_stats).unwrap();

        let health = world.get::<Health>(entity).unwrap();
        let expected_current = health.max * 0.5;
        assert!(
            (health.current - expected_current).abs() < 1.0,
            "health ratio should be preserved: {} vs {}",
            health.current,
            expected_current,
        );
    }

    #[test]
    fn update_stats_recalculates_mana_for_mana_class() {
        let mut world = World::new();
        let entity = spawn_player(&mut world, 2, 80, 100.0, 98.0);
        // Add Mana component for paladin
        world.entity_mut(entity).insert(Mana {
            current: 100.0,
            max: 100.0,
        });

        world.run_system_once(update_derived_stats).unwrap();

        let mana = world.get::<Mana>(entity).unwrap();
        // Paladin L80: base 4394 + 98 * 20 = 6354
        assert!((mana.max - 6354.0).abs() < 1.0);
    }

    #[test]
    fn update_stats_skips_mana_for_warrior() {
        let mut world = World::new();
        let entity = spawn_player(&mut world, 1, 80, 159.0, 0.0);

        world.run_system_once(update_derived_stats).unwrap();

        // Warrior has no Mana component
        assert!(world.get::<Mana>(entity).is_none());
    }

    #[test]
    fn update_stats_only_runs_on_change() {
        let mut world = World::new();
        let entity = spawn_player(&mut world, 1, 80, 159.0, 0.0);

        // First run processes the newly-added components
        world.run_system_once(update_derived_stats).unwrap();
        let max_after_first = world.get::<Health>(entity).unwrap().max;

        // Manually set health to something wrong
        world.get_mut::<Health>(entity).unwrap().max = 999.0;

        // Second run should NOT recalculate (UnitStats/Level unchanged)
        world.run_system_once(update_derived_stats).unwrap();
        let max_after_second = world.get::<Health>(entity).unwrap().max;

        // Should still be 999.0 since no stat/level change triggered
        assert!(
            (max_after_second - 999.0).abs() < 0.01
                || (max_after_second - max_after_first).abs() < 0.01,
            "system should only run on Changed<UnitStats|Level>"
        );
    }
}
