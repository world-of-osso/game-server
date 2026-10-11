//! TC a352b1fa: HandleModInvisibility/Detect and WorldObject::CanDetectInvisibilityOf.
use super::ActiveAuras;
use bevy::prelude::*;
use std::collections::BTreeMap;

/// Presence of a type is distinct from its (possibly zero) summed value.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct Invisibility {
    hidden: BTreeMap<i32, i32>,
    detection: BTreeMap<i32, i32>,
    detect_while_invisible: bool,
}

impl Invisibility {
    pub(super) fn from_auras(auras: &ActiveAuras) -> Self {
        let mut state = Self::default();
        for aura in auras.iter() {
            for effect in &aura.effects {
                let values = match effect.aura_type {
                    18 => &mut state.hidden,
                    19 => &mut state.detection,
                    335 => {
                        state.detect_while_invisible = true;
                        continue;
                    }
                    _ => continue,
                };
                *values.entry(effect.misc_values[0]).or_default() +=
                    (effect.amount * aura.stack_multiplier()) as i32;
            }
        }
        state
    }

    pub fn detects(&self, target: &Self) -> bool {
        target.hidden.iter().all(|(kind, value)| {
            self.detection
                .get(kind)
                .is_some_and(|detect| detect >= value)
        }) && (self.detect_while_invisible
            || self
                .hidden
                .keys()
                .all(|kind| target.detection.contains_key(kind)))
    }
}

pub fn can_detect(world: &World, observer: Entity, target: Entity) -> bool {
    observer == target
        || world
            .get::<Invisibility>(observer)
            .cloned()
            .unwrap_or_default()
            .detects(
                &world
                    .get::<Invisibility>(target)
                    .cloned()
                    .unwrap_or_default(),
            )
}
