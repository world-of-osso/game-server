//! TC a352b1fa Creature::GetAttackDistance (Creature.cpp:2165–2206), with
//! Rate.Creature.Aggro=1 and MaxPlayerLevel=90 (World.cpp:1002,751).
use bevy::prelude::*;
use shared::components::{ModelDisplay, Npc, Player, UnitScale};

use crate::auras::{ActiveAuras, handlers::MOD_DETECTED_RANGE};
use crate::scripted_ai::model_reach::{ModelReaches, creature_combat_reach};
use crate::world_data::WorldData;

const BASE_ATTACK_DISTANCE: f32 = 15.0;
const MIN_ATTACK_DISTANCE: f32 = 5.0;
const PLAYER_COMBAT_REACH: f32 = 1.5;
const MAX_PLAYER_LEVEL: u8 = 90;
const MOD_DETECT_RANGE: u32 = 91;

#[derive(Debug, Clone, Copy)]
struct AttackDistance {
    creature_level: u8,
    target_level: u8,
    creature_reach: f32,
    target_reach: f32,
    expansion_max_level: u8,
    detect_range: f32,
    detected_range: f32,
}

impl AttackDistance {
    /// IsWithinDistInMap adds both combat reaches after GetAttackDistance clamps.
    fn center_radius(self) -> f32 {
        let base = BASE_ATTACK_DISTANCE - self.creature_reach;
        let level_difference = f32::from(self.creature_level) - f32::from(self.target_level);
        let radius = if self.creature_level > self.expansion_max_level {
            base + f32::from(self.expansion_max_level) - f32::from(self.target_level)
        } else {
            let range_modifiers =
                if u16::from(self.creature_level) + 5 <= u16::from(MAX_PLAYER_LEVEL) {
                    self.detect_range + self.detected_range
                } else {
                    0.0
                };
            base + level_difference + range_modifiers
        };
        radius.clamp(MIN_ATTACK_DISTANCE, super::MAX_AGGRO_RADIUS)
            + self.creature_reach
            + self.target_reach
    }
}

/// SharedDefines.h:109–140, including EXPANSION_LEVEL_CURRENT=-1.
fn expansion_max_level(expansion: i32) -> u8 {
    match expansion {
        0..=2 => 30,
        3..=4 => 35,
        5 => 40,
        6 => 45,
        7 => 50,
        8 => 60,
        9 => 70,
        10 => 80,
        -1 | 11 => 90,
        _ => 0,
    }
}

type UnitGeometry<'w, 's> = Query<
    'w,
    's,
    (
        Has<Player>,
        Option<&'static Npc>,
        Option<&'static ModelDisplay>,
        Option<&'static UnitScale>,
        Option<&'static ActiveAuras>,
        Option<&'static crate::zones::CreatureDifficulty>,
    ),
>;

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct AggroDistance<'w, 's> {
    units: UnitGeometry<'w, 's>,
    models: Option<Res<'w, ModelReaches>>,
    data: Option<Res<'w, WorldData>>,
}

impl AggroDistance<'_, '_> {
    pub(crate) fn center_radius(
        &self,
        creature: Entity,
        target: Entity,
        creature_level: u8,
        target_level: u8,
    ) -> Option<f32> {
        let (_, _, _, _, auras, _) = self.units.get(creature).ok()?;
        let (_, _, _, _, target_auras, _) = self.units.get(target).ok()?;
        Some(
            AttackDistance {
                creature_level,
                target_level,
                creature_reach: self.combat_reach(creature)?,
                target_reach: self.combat_reach(target)?,
                expansion_max_level: self.expansion_max_level(creature)?,
                detect_range: aura_range(auras, MOD_DETECT_RANGE),
                detected_range: aura_range(target_auras, MOD_DETECTED_RANGE),
            }
            .center_radius(),
        )
    }

    fn expansion_max_level(&self, creature: Entity) -> Option<u8> {
        let (_, npc, _, _, _, difficulty) = self.units.get(creature).ok()?;
        let npc = npc?;
        let difficulty = difficulty.map_or(0, |difficulty| difficulty.0);
        let template = self
            .data
            .as_deref()
            .and_then(|data| data.template_for(npc.template_id, difficulty));
        let Some(template) = template else {
            error!(
                ?creature,
                entry = npc.template_id,
                difficulty,
                "Cannot evaluate creature aggro: authored difficulty template is missing"
            );
            return None;
        };
        Some(expansion_max_level(template.health_scaling_expansion))
    }

    pub(crate) fn combat_reach(&self, unit: Entity) -> Option<f32> {
        let (player, _, display, scale, _, _) = self.units.get(unit).ok()?;
        let scale = scale.map_or(1.0, |scale| scale.0);
        if player {
            return Some(PLAYER_COMBAT_REACH * scale);
        }
        let display = display?;
        let models = self.models.as_deref()?;
        Some(creature_combat_reach(models, display.display_id, scale))
    }
}

fn aura_range(auras: Option<&ActiveAuras>, aura_type: u32) -> f32 {
    auras.map_or(0.0, |auras| {
        auras
            .iter()
            .map(|aura| {
                let amount: f32 = aura
                    .effects
                    .iter()
                    .filter(|effect| effect.aura_type == aura_type)
                    .map(|effect| effect.amount)
                    .sum();
                amount * f32::from(aura.stacks.max(1))
            })
            .sum()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_geometry(creature_level: u8, target_level: u8) -> AttackDistance {
        AttackDistance {
            creature_level,
            target_level,
            creature_reach: 1.5,
            target_reach: 1.5,
            expansion_max_level: 90,
            detect_range: 0.0,
            detected_range: 0.0,
        }
    }

    #[test]
    fn tc_attack_distance_level_difference_and_reach_clamps() {
        assert_eq!(source_geometry(30, 32).center_radius(), 14.5);
        assert_eq!(source_geometry(32, 32).center_radius(), 16.5);
        assert_eq!(source_geometry(10, 5).center_radius(), 21.5);
        assert_eq!(source_geometry(1, 90).center_radius(), 8.0);
        assert_eq!(source_geometry(90, 1).center_radius(), 48.0);
        assert_eq!(
            AttackDistance {
                creature_reach: 24.0,
                ..source_geometry(32, 32)
            }
            .center_radius(),
            30.5
        );
    }

    #[test]
    fn tc_attack_distance_uses_expansion_cap_before_range_auras() {
        let geometry = AttackDistance {
            creature_level: 33,
            expansion_max_level: 30,
            detect_range: 100.0,
            detected_range: 100.0,
            ..source_geometry(33, 32)
        };
        assert_eq!(geometry.center_radius(), 14.5);
    }

    #[test]
    fn tc_attack_distance_detect_modifiers_stop_five_levels_below_max() {
        for (level, expected) in [(85, 24.5), (86, 16.5)] {
            let geometry = AttackDistance {
                detect_range: 6.0,
                detected_range: 2.0,
                ..source_geometry(level, level)
            };
            assert_eq!(geometry.center_radius(), expected);
        }
    }

    #[test]
    fn tc_attack_distance_expansion_maxima_match_installed_era() {
        let expected = [30, 30, 30, 35, 35, 40, 45, 50, 60, 70, 80, 90];
        for (expansion, maximum) in expected.into_iter().enumerate() {
            assert_eq!(expansion_max_level(expansion as i32), maximum);
        }
        assert_eq!(expansion_max_level(-1), 90);
    }
}
