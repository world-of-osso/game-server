//! Native Mountains Will Fall Earthquake overload fields.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::class_progression::ActiveSpec;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterElementalLaunch(after_quake),
    SpellScriptHook::TargetDamageMultiplier(ascendance_multiplier),
];
const MASTERY: u32 = 168534;
const MOUNTAINS: u32 = 381726;
const ASCENDANCE: u32 = 1219480;
const OVERLOAD: u32 = 298762;
const DAMAGE: u32 = 298765;
const DELAY_MS: u32 = 400;

fn aura_amount(world: &World, caster: Entity, spell: u32, index: u8) -> Option<f32> {
    let now = spell_cast::now(world);
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| {
            aura.spell_id == spell
                && aura.caster == caster
                && aura.expires_at.is_none_or(|at| at > now)
        })?
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .map(|effect| effect.amount)
}

fn after_quake(world: &mut World, caster: Entity, parent: &SpellInfo, foreground: bool) {
    if !foreground
        || parent.cast_parent_owns_modifier_charges
        || !matches!(parent.id, 61882 | 462620)
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 262)
    {
        return;
    }
    let Some(mountains) = aura_amount(world, caster, MOUNTAINS, 0) else {
        return;
    };
    let Some(chance) = aura_amount(world, caster, MASTERY, 0) else {
        return;
    };
    let Some(effectiveness) = aura_amount(world, caster, MASTERY, 1) else {
        return;
    };
    let center = parent.cast_destination.map(Vec3::from_array).or_else(|| {
        parent
            .cast_primary_target
            .and_then(|target| spell_cast::targets::position(world, target))
    });
    let Some(center) = center else {
        warn!(?caster, "Earthquake overload has no native center");
        return;
    };
    // SimC76f8118 sc_shaman.cpp3316–3351/3379–3414: one mastery roll and one Ascendance addition.
    let ordinary = combat::random::random::<f32>() * 100.0 < chance;
    let additional = aura_amount(world, caster, ASCENDANCE, 8).unwrap_or(0.0) as u32;
    let mut overload = world
        .resource::<SpellStore>()
        .get(OVERLOAD)
        .expect("native Earthquake overload field")
        .clone();
    overload.cast_damage_multiplier = Some(
        parent.cast_damage_multiplier.unwrap_or(1.0) * mountains / 100.0 * effectiveness / 100.0,
    );
    for _ in 0..u32::from(ordinary) + additional {
        spell_cast::scripts::spawn_delayed_ground_pulses(
            world, caster, &overload, DAMAGE, center, DELAY_MS,
        );
    }
}

fn ascendance_multiplier(world: &World, caster: Entity, _target: Entity, spell: &SpellInfo) -> f32 {
    // SimC76f8118 sc_shaman.cpp7904–7915: live Ascendance modifies each overload pulse.
    if spell.id != DAMAGE {
        return 1.0;
    }
    1.0 + aura_amount(world, caster, ASCENDANCE, 7).unwrap_or(0.0) / 100.0
}
