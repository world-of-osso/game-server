//! Soulstone self-resurrection selected from the holder's native dummy aura.
use bevy::prelude::*;
use shared::components::{Health, Mana};

use crate::maps::InMap;
use crate::spell_cast::effects::EffectCtx;
use crate::spell_info::{SpellEffect, SpellStore};

const USE_SOULSTONE: u32 = 3026;
const WARLOCK_CLASS: u32 = 5;
const SOULSTONE_FAMILY_FLAG: u32 = 0x0100_0000;
const DUMMY_AURA: u32 = 4;
pub(crate) const EFFECT_SELF_RESURRECT: u32 = 94;

#[derive(Component)]
struct SoulstoneAvailable(u32);

/// TC Player.cpp InitializeSelfResurrectionSpells: the matching dummy aura grants 3026.
pub(crate) fn remember_on_death(world: &mut World, unit: Entity) {
    if world.get::<crate::followers::Follower>(unit).is_none() {
        return;
    }
    let Some(auras) = world.get::<crate::auras::ActiveAuras>(unit) else {
        return;
    };
    let Some(store) = world.get_resource::<SpellStore>() else {
        return;
    };
    let found = auras.iter().find_map(|aura| {
        store
            .get(aura.spell_id)
            .filter(|spell| {
                spell.class_options.as_ref().is_some_and(|class| {
                    class.spell_class_set == WARLOCK_CLASS
                        && class.spell_class_mask[1] & SOULSTONE_FAMILY_FLAG != 0
                }) && spell.effects.iter().any(|effect| effect.aura == DUMMY_AURA)
            })
            .map(|_| aura.spell_id)
    });
    if let Some(spell) = found {
        world.entity_mut(unit).insert(SoulstoneAvailable(spell));
    }
}

#[cfg(test)]
pub(crate) fn available(world: &World, unit: Entity) -> bool {
    world.get::<SoulstoneAvailable>(unit).is_some()
}

pub(crate) fn clear_available(world: &mut World, unit: Entity) {
    world.entity_mut(unit).remove::<SoulstoneAvailable>();
}

pub(crate) fn use_follower_soulstone(world: &mut World, unit: Entity) -> bool {
    if world.get::<SoulstoneAvailable>(unit).is_none() {
        return false;
    }
    let Some(map) = world.get::<InMap>(unit).copied() else {
        return false;
    };
    if in_combat(world, unit, map)
        && crate::mythic_plus::combat_res_available(world, map) == Some(false)
    {
        return false;
    }
    crate::spell_cast::try_cast_triggered(world, unit, USE_SOULSTONE, Some(unit), 0)
}

fn in_combat(world: &World, unit: Entity, map: InMap) -> bool {
    if crate::scripted_ai::instance_script::boss_in_progress(world, map) {
        return true;
    }
    let Some(follower) = world.get::<crate::followers::Follower>(unit) else {
        return false;
    };
    crate::followers::party_in_combat(world, follower.owner)
}

/// TC SpellEffects.cpp EffectSelfResurrect: the holder restores its own health and mana.
pub(crate) fn self_resurrect(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
) {
    if ctx.caster != target
        || world.get::<SoulstoneAvailable>(target).is_none()
        || world
            .get::<Health>(target)
            .is_none_or(|health| health.current > 0.0)
    {
        return;
    }
    let Some(map) = world.get::<InMap>(target).copied() else {
        return;
    };
    if in_combat(world, target, map) && !crate::mythic_plus::consume_combat_res(world, map) {
        return;
    }
    let aura = world.get::<SoulstoneAvailable>(target).unwrap().0;
    crate::auras::remove_auras(
        world,
        target,
        crate::auras::RemoveReason::Death,
        |instance| instance.spell_id == aura,
    );
    let percent = effect.base_points.clamp(0.0, 100.0) / 100.0;
    if let Some(mut health) = world.get_mut::<Health>(target) {
        health.current = (health.max * percent).floor().max(1.0);
    }
    if let Some(mut mana) = world.get_mut::<Mana>(target) {
        mana.current = (mana.max * percent).floor();
    }
    world
        .entity_mut(target)
        .remove::<(
            SoulstoneAvailable,
            crate::combat::DeathState,
            crate::combat::Corpse,
        )>()
        .insert(crate::creature_ai::CreatureState::Idle);
    crate::spell_cast::mark_dirty(world, target);
}
