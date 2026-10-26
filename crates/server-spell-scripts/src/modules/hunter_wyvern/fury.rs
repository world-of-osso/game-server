//! Selected Fury extends an already-earned Cry, with one budget per lifetime.
use super::super::cast_relationships::has_talent;
use super::{WYVERN_CRY, combat};
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_info::SpellInfo;

const FURY: u32 = 472550;

#[derive(Component, Default)]
struct ExtendedMilliseconds(u32);

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(WYVERN_CRY).or_default().on_remove = Some(reset_budget);
}

fn reset_budget(world: &mut World, target: Entity, aura: &AuraInstance, _: RemoveReason) {
    // SimC76f8118 hunter8279–8283 resets only when Cry actually leaves.
    if aura.caster == target {
        world.entity_mut(target).remove::<ExtendedMilliseconds>();
    }
}

fn has_owned_cry(world: &World, caster: Entity) -> bool {
    world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == WYVERN_CRY && aura.caster == caster)
    })
}

pub(super) fn extend_after_command(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    foreground: bool,
) {
    if !foreground || spell.id != 34026 || !has_talent(world, caster, FURY) {
        return;
    }
    if !has_owned_cry(world, caster) {
        return;
    }
    let used = world
        .get::<ExtendedMilliseconds>(caster)
        .map_or(0, |used| used.0);
    let cap = (script_effect_value(world, caster, FURY, 3) * 1000.0) as u32;
    if used >= cap {
        return;
    }
    // SimC76f8118 hunter6626–6630 runs after Howl consumption. Native
    // effect2 supplies milliseconds; effect4 supplies the total cap in seconds.
    let extension = script_effect_value(world, caster, FURY, 1) as u32;
    auras::extend_owned_duration(world, caster, caster, &[WYVERN_CRY], extension);
    world
        .entity_mut(caster)
        .insert(ExtendedMilliseconds(used + extension));
}
