//! Explicit SimC relationships absent from native Demon Hunter trigger metadata.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::auras::ActiveAuras;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OnCast(before_cast),
    SpellScriptHook::EarlyAuraRegistration(demonsurge::register_havoc),
    SpellScriptHook::LandedDamagePolicy(damage::apply_havoc_damage_policies),
    SpellScriptHook::AbilityUseAvailable(demonsurge::havoc_demonsurge_available),
    SpellScriptHook::ChannelPeriodic(beam::periodic_trigger),
    SpellScriptHook::CriticalChance(beam::crit_chance),
];
use combat::class_progression::KnownSpells;

#[path = "havoc/beam.rs"]
mod beam;
#[path = "havoc/damage.rs"]
mod damage;
#[path = "havoc/demonsurge.rs"]
mod demonsurge;

fn is_havoc(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 577)
}

pub(super) fn has_talent(world: &World, caster: Entity, talent: u32) -> bool {
    is_havoc(world, caster)
        && world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(talent))
}

fn before_runtime_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    damage::before_cast(world, caster, spell);
    demonsurge::before_cast(world, caster, spell, depth);
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
    parents: &[198013, 452497],
    payload: 162264,
    duration_effect: 0,
};

pub(super) fn before_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    before_runtime_cast(world, caster, spell, depth);
    if !DEMONIC.parents.contains(&spell) || !selected(world, caster) {
        return;
    }
    let duration = combat::spell_cast::scripts::script_effect_value(
        world,
        caster,
        DEMONIC.talent,
        DEMONIC.duration_effect,
    );
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
        .resource::<combat::spell_info::SpellStore>()
        .get(spell)
        .expect("native Eye Beam/Gaze parent")
        .clone();
    let channel = combat::spell_cast::duration_ms(world, caster, &parent);
    if channel > 0 {
        combat::auras::extend_owned_duration(
            world,
            caster,
            caster,
            &[DEMONIC.payload],
            channel as u32,
        );
    }
}

fn selected(world: &World, caster: Entity) -> bool {
    let correct_spec = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == DEMONIC.spec);
    let learned = world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(DEMONIC.talent));
    correct_spec && learned
}

fn transform_or_extend(world: &mut World, caster: Entity, milliseconds: u32, depth: u8) {
    let now = combat::auras::aura_now(world);
    let active = world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.spell_id == DEMONIC.payload
                && aura.caster == caster
                && aura.expires_at.is_some_and(|end| end > now)
        })
    });
    if active {
        combat::auras::extend_owned_duration(
            world,
            caster,
            caster,
            &[DEMONIC.payload],
            milliseconds,
        );
        return;
    }
    if !combat::spell_cast::try_cast_triggered(
        world,
        caster,
        DEMONIC.payload,
        Some(caster),
        depth + 1,
    ) {
        error!(
            policy = DEMONIC.name,
            "Native Demonic transformation refused"
        );
        return;
    }
    if !combat::auras::set_owned_duration(world, caster, caster, DEMONIC.payload, milliseconds) {
        error!(
            policy = DEMONIC.name,
            "Native Demonic transformation aura absent"
        );
    }
}
