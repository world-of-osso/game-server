//! TC a352b1fa Object.cpp:1023-1097; SpellAuraEffects.cpp:1724-1810.
use super::{ActiveAuras, handlers};
use bevy::prelude::*;
use shared::components::{Level, Npc, Player, Position, Rotation, UnitScale, UnitSummonedBy};
use shared::protocol::GameObjectInfo;
use std::collections::{BTreeMap, BTreeSet};

const PLAYER_COMBAT_REACH: f32 = 1.5;
const MAX_PLAYER_DETECTION_RANGE: f32 = 30.0;
const DETECTION_POINTS_PER_LEVEL: f32 = 5.0;
const YARDS_PER_POINT: f32 = 0.3;
const BASE_DETECTION_POINTS: f32 = 30.0;

#[derive(Component, Clone, Debug, Default, PartialEq)]
pub(crate) struct Stealth {
    hidden: BTreeMap<i32, f32>,
    detection: BTreeMap<i32, f32>,
    absolute: BTreeSet<i32>,
}

impl Stealth {
    pub(super) fn from_auras(auras: &ActiveAuras) -> Self {
        let mut state = Self::default();
        for aura in auras.iter() {
            for effect in &aura.effects {
                state.add_detection_effect(effect, aura.stack_multiplier());
            }
        }
        state.add_level_modifiers(auras);
        state
    }

    fn add_detection_effect(&mut self, effect: &super::AuraEffectState, stacks: f32) {
        let kind = effect.misc_values[0];
        let amount = effect.amount * stacks;
        match effect.aura_type {
            handlers::MOD_STEALTH => *self.hidden.entry(kind).or_default() += amount,
            handlers::MOD_STEALTH_DETECT => *self.detection.entry(kind).or_default() += amount,
            handlers::DETECT_STEALTH => {
                self.absolute.insert(kind);
            }
            _ => {}
        }
    }

    fn add_level_modifiers(&mut self, auras: &ActiveAuras) {
        for aura in auras.iter() {
            for effect in &aura.effects {
                if effect.aura_type == handlers::MOD_STEALTH_LEVEL
                    && let Some(value) = self.hidden.get_mut(&effect.misc_values[0])
                {
                    *value += effect.amount * aura.stack_multiplier();
                }
            }
        }
    }

    pub(crate) fn detects(
        &self,
        target: &Self,
        geometry: DetectionGeometry,
        observer: Position,
        victim: Position,
    ) -> bool {
        if target.hidden.is_empty() {
            return true;
        }
        let victim = Vec3::new(victim.x, victim.y, victim.z);
        let observer = Vec3::new(observer.x, observer.y, observer.z);
        let offset = victim - observer;
        let distance = offset.length();
        let Some(reach) = geometry.reach else {
            return false; // Local sources do not contain NPC model CombatReach rows.
        };
        if distance < reach {
            return true;
        }
        if geometry.unit && !geometry.faces(offset) {
            return false;
        }
        if geometry.trap {
            return true;
        }
        target
            .hidden
            .iter()
            .all(|(kind, hidden)| self.detects_type(*kind, *hidden, geometry, distance))
    }

    fn detects_type(
        &self,
        kind: i32,
        hidden: f32,
        geometry: DetectionGeometry,
        distance: f32,
    ) -> bool {
        if geometry.unit && self.absolute.contains(&kind) {
            return true;
        }
        let points = BASE_DETECTION_POINTS
            + f32::from(geometry.level.saturating_sub(1)) * DETECTION_POINTS_PER_LEVEL
            + self.detection.get(&kind).copied().unwrap_or(0.0)
            - hidden;
        geometry
            .range(points)
            .is_some_and(|range| distance <= range)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DetectionGeometry {
    level: u8,
    facing: Option<f32>,
    reach: Option<f32>,
    unit: bool,
    player: bool,
    trap: bool,
}

pub(crate) type GeometryParts<'a> = (
    Option<&'a Level>,
    Option<&'a Rotation>,
    Option<&'a UnitScale>,
    bool,
    bool,
    Option<&'a GameObjectInfo>,
);

impl DetectionGeometry {
    pub(crate) fn is_unit(&self) -> bool {
        self.unit
    }
    pub(crate) fn from_parts(
        (level, rotation, scale, player, npc, object): GeometryParts<'_>,
    ) -> Self {
        let unit = player || npc;
        Self {
            level: level.map_or(1, |level| level.0),
            facing: rotation.map(|rotation| rotation.y),
            reach: if player {
                Some(PLAYER_COMBAT_REACH * scale.map_or(1.0, |scale| scale.0))
            } else if npc {
                None
            } else {
                Some(0.0)
            },
            unit,
            player,
            trap: object.is_some_and(|object| object.go_type == 6),
        }
    }
    fn range(self, points: f32) -> Option<f32> {
        let range = points * YARDS_PER_POINT + self.reach?;
        Some(if self.player {
            range.min(MAX_PLAYER_DETECTION_RANGE)
        } else {
            range
        })
    }

    fn faces(self, offset: Vec3) -> bool {
        self.facing
            .is_some_and(|angle| Vec3::new(angle.sin(), 0.0, angle.cos()).dot(offset) >= 0.0)
    }
    fn in_world(world: &World, entity: Entity) -> Self {
        Self::from_parts((
            world.get::<Level>(entity),
            world.get::<Rotation>(entity),
            world.get::<UnitScale>(entity),
            world.get::<Player>(entity).is_some(),
            world.get::<Npc>(entity).is_some(),
            world.get::<GameObjectInfo>(entity),
        ))
    }
}
impl Default for DetectionGeometry {
    fn default() -> Self {
        Self::from_parts((None, None, None, false, false, None))
    }
}

pub(crate) fn can_detect(world: &World, observer: Entity, target: Entity) -> bool {
    let observer = world
        .get::<UnitSummonedBy>(observer)
        .map_or(observer, |owner| {
            Entity::try_from_bits(owner.0).expect("server-owned summon controller")
        });
    let Some(target_state) = world
        .get::<Stealth>(target)
        .filter(|state| !state.hidden.is_empty())
    else {
        return true;
    };
    let (Some(observer_pos), Some(target_pos)) = (
        world.get::<Position>(observer),
        world.get::<Position>(target),
    ) else {
        return false;
    };
    world
        .get::<Stealth>(observer)
        .cloned()
        .unwrap_or_default()
        .detects(
            target_state,
            DetectionGeometry::in_world(world, observer),
            *observer_pos,
            *target_pos,
        )
}

pub(crate) type DetectionUnits<'w, 's> = Query<
    'w,
    's,
    (
        &'static Position,
        Option<&'static Level>,
        Option<&'static Rotation>,
        Option<&'static UnitScale>,
        Has<Player>,
        Has<Npc>,
        Option<&'static GameObjectInfo>,
        Option<&'static Stealth>,
        Option<&'static UnitSummonedBy>,
    ),
>;

pub(crate) fn query_can_detect(
    units: &DetectionUnits<'_, '_>,
    observer: Entity,
    target: Entity,
) -> bool {
    let Ok((target_pos, _, _, _, _, _, _, target_state, _)) = units.get(target) else {
        return false;
    };
    let Some(target_state) = target_state.filter(|state| !state.hidden.is_empty()) else {
        return true;
    };
    let Ok((_, _, _, _, _, _, _, _, owner)) = units.get(observer) else {
        return false;
    };
    let observer = owner.map_or(observer, |owner| {
        Entity::try_from_bits(owner.0).expect("server-owned summon controller")
    });
    let Ok((position, level, rotation, scale, player, npc, object, state, _)) = units.get(observer)
    else {
        return false;
    };
    state.cloned().unwrap_or_default().detects(
        target_state,
        DetectionGeometry::from_parts((level, rotation, scale, player, npc, object)),
        *position,
        *target_pos,
    )
}
