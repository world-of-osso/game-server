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

    /// TC Object.cpp:1003 limits reciprocal detection to units, not arena doors.
    pub fn detects(&self, target: &Self, target_is_unit: bool) -> bool {
        target.hidden.iter().all(|(kind, value)| {
            self.detection
                .get(kind)
                .is_some_and(|detect| detect >= value)
        }) && (!target_is_unit
            || self.detect_while_invisible
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
    fn invisibility_detection_spell_relations_follow_observer_values() {
        use crate::spell_cast::targets::{Relation, relation_matches};
        let mut h =
            crate::spell_cast::test_harness::harness(Vec::new(), &[89302, 100616], 1).unwrap();
        for (unit, spell_id) in [(h.npc, 89302), (h.caster, 100616)] {
            let spell = h
                .app
                .world()
                .resource::<crate::spell_info::SpellStore>()
                .get(spell_id)
                .unwrap();
            let mask = super::super::aura_effect_mask(spell);
            super::super::apply_aura(
                h.app.world_mut(),
                &super::super::AuraApplication {
                    caster: unit,
                    target: unit,
                    spell_id,
                    effect_mask: mask,
                    source: super::super::AuraSource::Cast,
                },
            );
            assert_eq!(
                relation_matches(h.app.world(), h.caster, h.npc, Relation::Enemy),
                unit == h.caster
            );
        }
        h.app
            .world_mut()
            .entity_mut(h.npc)
            .insert(shared::components::UnitFactionTemplate(1));
        assert!(relation_matches(
            h.app.world(),
            h.caster,
            h.npc,
            Relation::Ally
        ));
        super::super::remove_auras(
            h.app.world_mut(),
            h.caster,
            super::super::RemoveReason::Cancelled,
            |aura| aura.spell_id == 100616,
        );
        assert!(!relation_matches(
            h.app.world(),
            h.caster,
            h.npc,
            Relation::Ally
        ));
    }

    #[test]
    fn invisibility_detection_sums_real_aura_values_and_stacks() {
        let mut h =
            crate::spell_cast::test_harness::harness(Vec::new(), &[89302, 101058], 1).unwrap();
        for spell_id in [89302, 101058] {
            let spell = h
                .app
                .world()
                .resource::<crate::spell_info::SpellStore>()
                .get(spell_id)
                .unwrap();
            let mask = super::super::aura_effect_mask(spell);
            super::super::apply_aura(
                h.app.world_mut(),
                &super::super::AuraApplication {
                    caster: h.npc,
                    target: h.npc,
                    spell_id,
                    effect_mask: mask,
                    source: super::super::AuraSource::Cast,
                },
            );
        }
        let mut auras = h.app.world_mut().get_mut::<ActiveAuras>(h.npc).unwrap();
        auras
            .auras
            .iter_mut()
            .find(|aura| aura.spell_id == 89302)
            .unwrap()
            .stacks = 2;
        let target = Invisibility::from_auras(&auras);
        let mut observer = Invisibility {
            detection: BTreeMap::from([(35, 299)]),
            ..default()
        };
        assert!(
            !observer.detects(&target, true),
            "two 100-point stacks plus a 100-point aura need300 detection"
        );
        observer.detection.insert(35, 300);
        assert!(observer.detects(&target, true));
    }

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
        assert!(
            !observer.detects(&target, true),
            "one detection value is too low"
        );
        observer.detection.insert(12, 100);
        assert!(observer.detects(&target, true));
        observer.detection.remove(&35);
        assert!(
            !observer.detects(&target, true),
            "missing a type still hides the unit"
        );
        target.hidden = BTreeMap::from([(35, 0)]);
        assert!(
            !observer.detects(&target, true),
            "zero invisibility retains type presence"
        );
        observer.detection.insert(35, 0);
        assert!(observer.detects(&target, true));
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
        assert!(!observer.detects(&target, true));
        assert!(
            observer.detects(&target, false),
            "non-units do not need reciprocal flags"
        );
        observer.detection.remove(&35);
        assert!(
            !observer.detects(&target, false),
            "the object's own hidden types still require detection"
        );
        observer.detection.insert(35, 100);
        target.detection.insert(12, 0);
        assert!(
            observer.detects(&target, true),
            "reciprocal check needs the type, not its value"
        );
        target.detection.clear();
        observer.detect_while_invisible = true;
        assert!(observer.detects(&target, true));
    }
}

pub fn can_detect(world: &World, observer: Entity, target: Entity) -> bool {
    observer == target
        || (super::stealth::can_detect(world, observer, target)
            && world
                .get::<Invisibility>(observer)
                .cloned()
                .unwrap_or_default()
                .detects(
                    &world
                        .get::<Invisibility>(target)
                        .cloned()
                        .unwrap_or_default(),
                    world.get::<shared::components::Player>(target).is_some()
                        || world.get::<shared::components::Npc>(target).is_some(),
                ))
}
