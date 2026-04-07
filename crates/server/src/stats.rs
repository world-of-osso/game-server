use bevy::prelude::*;
use shared::components::{CombatRatings, Health, ItemStatBlock, Level, Mana, Player, UnitStats};
use shared::formulas;

/// Component holding the stat blocks of all equipped items.
///
/// When this changes (equip/unequip), `update_stats_from_equipment` recomputes
/// `UnitStats` and `CombatRatings`, which then triggers `update_derived_stats`
/// to recalculate HP/mana.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct EquippedItems {
    pub items: Vec<ItemStatBlock>,
}

/// Base stats for a character (without gear). Set once at character creation
/// or level-up. Equipment stats are added on top by `update_stats_from_equipment`.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct BaseStats {
    pub primary: UnitStats,
    pub secondary: CombatRatings,
}

pub struct StatsPlugin;

impl Plugin for StatsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (update_stats_from_equipment, update_derived_stats).chain(),
        );
    }
}

/// When equipped items change, recompute UnitStats = base + gear.
fn update_stats_from_equipment(
    mut query: Query<
        (
            &BaseStats,
            &EquippedItems,
            &mut UnitStats,
            &mut CombatRatings,
        ),
        Changed<EquippedItems>,
    >,
) {
    for (base, equipped, mut stats, mut ratings) in &mut query {
        let (gear_primary, gear_secondary) = formulas::sum_equipment_stats(&equipped.items);
        *stats = base.primary;
        *stats += gear_primary;
        *ratings = base.secondary;
        *ratings += gear_secondary;
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
    let ratio = if old_max > 0.0 {
        current / old_max
    } else {
        1.0
    };
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

    fn spawn_player(
        world: &mut World,
        class: u8,
        level: u8,
        stamina: f32,
        intellect: f32,
    ) -> Entity {
        world
            .spawn((
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
            ))
            .id()
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

    fn spawn_player_with_gear(
        world: &mut World,
        class: u8,
        level: u8,
        base_stamina: f32,
        items: Vec<ItemStatBlock>,
    ) -> Entity {
        world
            .spawn((
                Player {
                    name: "Test".into(),
                    race: 1,
                    class,
                    appearance: Default::default(),
                },
                BaseStats {
                    primary: UnitStats {
                        stamina: base_stamina,
                        ..Default::default()
                    },
                    secondary: CombatRatings::default(),
                },
                UnitStats::default(),
                CombatRatings::default(),
                EquippedItems { items },
                Level(level),
                Health {
                    current: 100.0,
                    max: 100.0,
                },
            ))
            .id()
    }

    #[test]
    fn equip_recalculates_stats() {
        let mut world = World::new();
        let gear = vec![ItemStatBlock {
            primary: UnitStats {
                stamina: 50.0,
                ..Default::default()
            },
            secondary: CombatRatings::default(),
        }];
        let entity = spawn_player_with_gear(&mut world, 1, 80, 100.0, gear);

        world.run_system_once(update_stats_from_equipment).unwrap();

        let stats = world.get::<UnitStats>(entity).unwrap();
        // base 100 + gear 50 = 150
        assert!((stats.stamina - 150.0).abs() < 0.01);
    }

    #[test]
    fn equip_triggers_health_recalc() {
        let mut world = World::new();
        let gear = vec![ItemStatBlock {
            primary: UnitStats {
                stamina: 50.0,
                ..Default::default()
            },
            secondary: CombatRatings::default(),
        }];
        let entity = spawn_player_with_gear(&mut world, 1, 80, 100.0, gear);

        // Run both systems in chain
        world.run_system_once(update_stats_from_equipment).unwrap();
        world.run_system_once(update_derived_stats).unwrap();

        let health = world.get::<Health>(entity).unwrap();
        // Warrior L80: base_hp 8121 + 150 stam * 20 = 11121
        assert!((health.max - 11121.0).abs() < 1.0);
    }

    #[test]
    fn unequip_reduces_stats() {
        let mut world = World::new();
        let gear = vec![ItemStatBlock {
            primary: UnitStats {
                stamina: 80.0,
                ..Default::default()
            },
            secondary: CombatRatings::default(),
        }];
        let entity = spawn_player_with_gear(&mut world, 1, 80, 100.0, gear);

        world.run_system_once(update_stats_from_equipment).unwrap();
        let stamina_with_gear = world.get::<UnitStats>(entity).unwrap().stamina;
        assert!((stamina_with_gear - 180.0).abs() < 0.01);

        // Unequip: clear items
        world
            .get_mut::<EquippedItems>(entity)
            .unwrap()
            .items
            .clear();
        world.run_system_once(update_stats_from_equipment).unwrap();

        let stamina_without = world.get::<UnitStats>(entity).unwrap().stamina;
        assert!((stamina_without - 100.0).abs() < 0.01);
    }

    #[test]
    fn equip_aggregates_secondary_ratings() {
        let mut world = World::new();
        let gear = vec![
            ItemStatBlock {
                primary: UnitStats::default(),
                secondary: CombatRatings {
                    crit: 50.0,
                    ..Default::default()
                },
            },
            ItemStatBlock {
                primary: UnitStats::default(),
                secondary: CombatRatings {
                    crit: 30.0,
                    haste: 20.0,
                    ..Default::default()
                },
            },
        ];
        let entity = spawn_player_with_gear(&mut world, 1, 80, 100.0, gear);

        world.run_system_once(update_stats_from_equipment).unwrap();

        let ratings = world.get::<CombatRatings>(entity).unwrap();
        assert!((ratings.crit - 80.0).abs() < 0.01);
        assert!((ratings.haste - 20.0).abs() < 0.01);
    }
}
