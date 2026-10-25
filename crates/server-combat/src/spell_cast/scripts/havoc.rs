//! Explicit SimC relationships absent from native Demon Hunter trigger metadata.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use crate::auras::ActiveAuras;
use crate::class_progression::KnownSpells;

mod damage;
mod demonsurge;
pub(crate) use damage::apply_havoc_damage_policies;
pub use demonsurge::havoc_demonsurge_available;
pub(crate) use demonsurge::register_havoc;

fn is_havoc(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == DEMONIC.spec)
}

fn has_talent(world: &World, caster: Entity, talent: u32) -> bool {
    is_havoc(world, caster)
        && world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(talent))
}

struct TransformPolicy {
    name: &'static str,
    spec: u32,
    talent: u32,
    parents: &'static [u32],
    payload: u32,
    duration_effect: u8,
}

// SimC76f8118 sc_demon_hunter.cpp4020–4023,4078–4081,9153–9172:
// Eye Beam/Gaze enters or extends Meta before channel execution. Native213410
// effect0 supplies the duration; native162264 owns every transformation effect.
const DEMONIC: TransformPolicy = TransformPolicy {
    name: "SIMC_HAVOC_DEMONIC_TRANSFORMATION",
    spec: 577,
    talent: 213410,
    parents: &[super::EYE_BEAM, 452497],
    payload: 162264,
    duration_effect: 0,
};

pub(crate) fn is_eye_beam_channel(spell: u32) -> bool {
    DEMONIC.parents.contains(&spell)
}

pub(super) fn before_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    damage::before_cast(world, caster, spell);
    demonsurge::before_cast(world, caster, spell, depth);
    if !DEMONIC.parents.contains(&spell) || !selected(world, caster) {
        return;
    }
    let duration =
        super::script_effect_value(world, caster, DEMONIC.talent, DEMONIC.duration_effect);
    if !duration.is_finite() || duration <= 0.0 || duration > u32::MAX as f32 {
        error!(
            policy = DEMONIC.name,
            duration, "Native Demonic duration invalid"
        );
        return;
    }
    transform_or_extend(world, caster, duration as u32, depth);
    demonsurge::arm_demonic(world, caster);
    // SimC76f8118 sc_demon_hunter.cpp4051–4057: add the hasted channel
    // duration after Meta has supplied its native haste, not another flat grant.
    let parent = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .expect("native Eye Beam/Gaze parent")
        .clone();
    let channel = super::super::duration_ms(world, caster, &parent);
    if channel > 0 {
        crate::auras::extend_owned_duration(
            world,
            caster,
            caster,
            &[DEMONIC.payload],
            channel as u32,
        );
    }
}

fn selected(world: &World, caster: Entity) -> bool {
    has_talent(world, caster, DEMONIC.talent)
}

fn transform_or_extend(world: &mut World, caster: Entity, milliseconds: u32, depth: u8) {
    let now = crate::auras::aura_now(world);
    let active = world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.spell_id == DEMONIC.payload
                && aura.caster == caster
                && aura.expires_at.is_some_and(|end| end > now)
        })
    });
    if active {
        crate::auras::extend_owned_duration(
            world,
            caster,
            caster,
            &[DEMONIC.payload],
            milliseconds,
        );
        return;
    }
    if !super::super::try_cast_triggered(world, caster, DEMONIC.payload, Some(caster), depth + 1) {
        error!(
            policy = DEMONIC.name,
            "Native Demonic transformation refused"
        );
        return;
    }
    if !crate::auras::set_owned_duration(world, caster, caster, DEMONIC.payload, milliseconds) {
        error!(
            policy = DEMONIC.name,
            "Native Demonic transformation aura absent"
        );
    }
}
