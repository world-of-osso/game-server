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
                    super::handlers::MOD_INVISIBILITY => &mut state.hidden,
                    super::handlers::MOD_INVISIBILITY_DETECT => &mut state.detection,
                    super::handlers::DETECT_VISIBLE_WHILE_INVISIBLE => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invisibility_detection_requires_every_type_and_sufficient_value() {
        let mut target = Invisibility {
            hidden: BTreeMap::from([(12, 100), (35, 100)]),
            ..default()
        };
        let mut observer = Invisibility {
            detection: BTreeMap::from([(12, 99), (35, 100)]),
            ..default()
        };
        assert!(!observer.detects(&target), "one detection value is too low");
        observer.detection.insert(12, 100);
        assert!(observer.detects(&target));
        observer.detection.remove(&35);
        assert!(
            !observer.detects(&target),
            "missing a type still hides the unit"
        );
        target.hidden = BTreeMap::from([(35, 0)]);
        assert!(
            !observer.detects(&target),
            "zero invisibility retains type presence"
        );
        observer.detection.insert(35, 0);
        assert!(observer.detects(&target));
    }

    #[test]
    fn invisibility_detection_requires_reciprocal_flags_unless_aura335() {
        let mut observer = Invisibility {
            hidden: BTreeMap::from([(12, 100)]),
            detection: BTreeMap::from([(35, 100)]),
            ..default()
        };
        let mut target = Invisibility {
            hidden: BTreeMap::from([(35, 100)]),
            ..default()
        };
        assert!(!observer.detects(&target));
        target.detection.insert(12, 0);
        assert!(
            observer.detects(&target),
            "reciprocal check needs the type, not its value"
        );
        target.detection.clear();
        observer.detect_while_invisible = true;
        assert!(observer.detects(&target));
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
