//! Refractive Images defers a fraction of damage into a rolling periodic pool.
use bevy::prelude::*;

use super::{ActiveAuras, AuraApplication, AuraSource, has_aura};
use crate::class_progression::TraitLoadout;
use crate::spell_info::SpellStore;

const REFRACTIVE_IMAGES: u32 = 1309497;
const REFRACTION: u32 = 1309786;
const REFRACTION_DURATION: u32 = 1309787;
const AMPLIFIED_REFRACTION: u32 = 1309793;

/// Snapshot the talent and settle already-due ticks before the next hit.
pub(crate) fn snapshot(world: &mut World, target: Entity) -> Option<f32> {
    if !world.get::<TraitLoadout>(target).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == REFRACTIVE_IMAGES)
    }) {
        return None;
    }
    let instance = world.get::<ActiveAuras>(target).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == REFRACTION && aura.caster == target)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        let now = crate::spell_cast::now(world);
        super::periodic::run_ticks_until(world, target, instance, now);
    }
    let store = world.resource::<SpellStore>();
    let effect = |spell: u32| {
        store
            .get(spell)
            .expect("Refractive Images spell")
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("Refractive Images percentage")
            .base_points
    };
    // VERIFIED_WCL dtmp7Lg2abzyZVBG fight 3: 30% during Mirror Image.
    let percent = effect(REFRACTIVE_IMAGES)
        + if has_aura(world, target, AMPLIFIED_REFRACTION) {
            effect(AMPLIFIED_REFRACTION)
        } else {
            0.0
        };
    Some(percent / 100.0)
}

/// Roll deferred damage into eight seconds of one-second Refraction ticks.
pub(crate) fn deposit(world: &mut World, target: Entity, amount: f32) {
    if amount <= 0.0 {
        return;
    }
    let store = world.resource::<SpellStore>();
    let spell = store
        .get(REFRACTION)
        .expect("Refraction damage spell")
        .clone();
    let duration = store
        .get(REFRACTION_DURATION)
        .and_then(|spell| spell.misc.as_ref())
        .and_then(|misc| misc.duration)
        .expect("Refraction duration")
        .duration_ms;
    let application = AuraApplication {
        caster: target,
        target,
        spell_id: REFRACTION,
        effect_mask: 1,
        source: AuraSource::Cast,
    };
    super::apply_refraction_deposit(world, &application, &spell, duration as u32, amount);
}
