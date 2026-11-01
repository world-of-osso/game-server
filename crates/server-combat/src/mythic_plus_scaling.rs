//! Cited keystone creature scaling; the observed reference is an unweakened +2.
use bevy::prelude::*;
use serde::Deserialize;
use shared::components::{Health, Npc};

#[derive(Component, Clone, Copy, Debug)]
pub struct KeystoneCreatureScale {
    pub damage: f32,
}

#[derive(Clone, Debug, Deserialize)]
struct Sourced<T> {
    value: T,
    citation: String,
    qualification: String,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct LevelRates {
    through_10: f64,
    from_11: f64,
    round_to: f64,
    reference_level: u32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Multipliers {
    pub health: f64,
    pub damage: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ScalingPolicy {
    per_level: Sourced<LevelRates>,
    fortified: Sourced<Multipliers>,
    tyrannical: Sourced<Multipliers>,
}

impl ScalingPolicy {
    pub fn retail() -> &'static Self {
        static POLICY: std::sync::OnceLock<ScalingPolicy> = std::sync::OnceLock::new();
        POLICY.get_or_init(|| {
            let policy: Self = serde_json::from_str(include_str!("mythic_plus_scaling.json"))
                .expect("parse cited Mythic+ scaling policy");
            for (citation, qualification) in [
                (&policy.per_level.citation, &policy.per_level.qualification),
                (&policy.fortified.citation, &policy.fortified.qualification),
                (
                    &policy.tyrannical.citation,
                    &policy.tyrannical.qualification,
                ),
            ] {
                assert!(!citation.is_empty() && !qualification.is_empty());
            }
            policy
        })
    }

    fn coefficient(&self, level: u32) -> f64 {
        let rates = self.per_level.value;
        let normal_levels = level.saturating_sub(1).min(9) as i32;
        let higher_levels = level.saturating_sub(10) as i32;
        let coefficient = rates.through_10.powi(normal_levels) * rates.from_11.powi(higher_levels);
        (coefficient * rates.round_to).round() / rates.round_to
    }

    pub fn multipliers(
        &self,
        level: u32,
        boss: bool,
        fortified: bool,
        tyrannical: bool,
    ) -> Multipliers {
        let health = self.coefficient(level);
        let damage = health / self.coefficient(self.per_level.value.reference_level);
        let affix = if boss && tyrannical {
            self.tyrannical.value
        } else if !boss && fortified {
            self.fortified.value
        } else {
            Multipliers {
                health: 1.0,
                damage: 1.0,
            }
        };
        Multipliers {
            health: health * affix.health,
            damage: damage * affix.damage,
        }
    }
}

/// Scale each creature in the copy once, including creatures spawned after activation.
pub(crate) fn scale_copy(world: &mut World, map: crate::maps::InMap, level: u32, affixes: &[u32]) {
    let creatures: Vec<Entity> = world
        .query_filtered::<(Entity, &crate::maps::InMap), (
            With<Npc>,
            With<Health>,
            Without<KeystoneCreatureScale>,
            Without<crate::followers::Follower>,
        )>()
        .iter(world)
        .filter_map(|(entity, creature_map)| (*creature_map == map).then_some(entity))
        .collect();
    let policy = ScalingPolicy::retail();
    for creature in creatures {
        scale_creature(world, creature, level, affixes, policy);
    }
}

pub(crate) fn is_eligible(world: &World, creature: Entity) -> bool {
    if world.get::<KeystoneCreatureScale>(creature).is_some()
        || world.get::<crate::followers::Follower>(creature).is_some()
    {
        return false;
    }
    if let Some(summon) = world.get::<crate::zones::TempSummon>(creature)
        && crate::followers::is_player_like_combatant(world, summon.summoner)
    {
        return false;
    }
    if let Some(owner) = world.get::<shared::components::UnitSummonedBy>(creature)
        && let Some(owner) = Entity::try_from_bits(owner.0)
        && crate::followers::is_player_like_combatant(world, owner)
    {
        return false;
    }
    true
}

pub(crate) fn scale_creature(
    world: &mut World,
    creature: Entity,
    level: u32,
    affixes: &[u32],
    policy: &ScalingPolicy,
) {
    if !is_eligible(world, creature) {
        return;
    }
    let dungeon_boss = world
        .get::<crate::zones::CreatureFlagsExtra>(creature)
        .is_some_and(|flags| flags.0 & 0x1000_0000 != 0);
    let boss = dungeon_boss || crate::followers::is_boss(world, creature);
    let multipliers = policy.multipliers(level, boss, affixes.contains(&10), affixes.contains(&9));
    apply_creature_scale(world, creature, multipliers);
}

fn apply_creature_scale(world: &mut World, creature: Entity, multipliers: Multipliers) {
    let mut entity = world.entity_mut(creature);
    let mut health = entity.get_mut::<Health>().expect("queried creature health");
    health.max *= multipliers.health as f32;
    health.current *= multipliers.health as f32;
    entity.insert(KeystoneCreatureScale {
        damage: multipliers.damage as f32,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mythic_scaling_existing_and_later_creatures_keep_health_proportion() {
        let mut world = World::new();
        world.insert_resource(crate::world_data::WorldData::default());
        let map = crate::maps::InMap {
            map_id: 2515,
            instance_id: 7,
        };
        let make_npc = |world: &mut World| {
            world
                .spawn((
                    map,
                    Npc {
                        template_id: 100,
                        name: "Trash".into(),
                    },
                    Health {
                        current: 40.0,
                        max: 100.0,
                    },
                ))
                .id()
        };
        let owner = world
            .spawn(shared::components::Player {
                name: "Hunter".into(),
                race: 1,
                class: 3,
                appearance: Default::default(),
            })
            .id();
        let pet = make_npc(&mut world);
        world
            .entity_mut(pet)
            .insert(shared::components::UnitSummonedBy(owner.to_bits()));
        let before = make_npc(&mut world);
        scale_copy(&mut world, map, 12, &[10, 9]);
        let after = make_npc(&mut world);
        let boss = make_npc(&mut world);
        world
            .entity_mut(boss)
            .insert(crate::zones::CreatureFlagsExtra(0x1000_0000));
        scale_copy(&mut world, map, 12, &[10, 9]);
        let multiplier = 2.22_f32 * 1.2;
        for entity in [before, after] {
            let health = world.get::<Health>(entity).unwrap();
            assert!((health.max - 100.0 * multiplier).abs() < 0.001);
            assert!((health.current - 40.0 * multiplier).abs() < 0.001);
            let attacker = crate::level_scaling::UnitScaling::of(&world, entity);
            let target = crate::level_scaling::UnitScaling::from_item((None, None, None));
            assert_eq!(
                crate::level_scaling::damage_dealt(&attacker, &target, 100.0),
                (100.0_f32 * (2.22 / 1.07) * 1.2).trunc()
            );
        }
        assert_eq!(world.get::<Health>(pet).unwrap().max, 100.0);
        assert!(world.get::<KeystoneCreatureScale>(pet).is_none());
        let health = world.get::<Health>(boss).unwrap();
        assert!((health.max - 100.0 * 2.22_f32 * 1.25).abs() < 0.001);
        let boss_scale = crate::level_scaling::UnitScaling::of(&world, boss);
        let target = crate::level_scaling::UnitScaling::from_item((None, None, None));
        assert_eq!(
            crate::level_scaling::damage_dealt(&boss_scale, &target, 100.0),
            (100.0_f32 * (2.22 / 1.07) * 1.15).trunc()
        );
    }

    #[test]
    fn mythic_scaling_fitted_level_and_affixes() {
        let policy = ScalingPolicy::retail();
        let trash = policy.multipliers(12, false, true, true);
        assert!((trash.health - 2.22 * 1.2).abs() < 1e-12);
        assert!((trash.damage - 2.22 / 1.07 * 1.2).abs() < 1e-12);
        let boss = policy.multipliers(12, true, true, true);
        assert!((boss.health - 2.22 * 1.25).abs() < 1e-12);
        assert!((boss.damage - 2.22 / 1.07 * 1.15).abs() < 1e-12);
        let reference = policy.multipliers(2, false, false, false);
        assert_eq!(reference.health, 1.07);
        assert_eq!(reference.damage, 1.0);
    }
}
