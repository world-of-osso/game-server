//! Named70448 Agony ramp; SimC76f8118 sc_warlock_actions.cpp1405–1491.
//! Soul Shard progress RNG/Shared Agony and old Writhe196102 are not implemented here.
use bevy::prelude::*;

use super::ActiveAuras;

pub(crate) const AGONY: u32 = 980;
pub(super) const SUDDEN_ONSET: u32 = 1_260_209;

pub(super) fn initial_stacks(world: &World, caster: Entity, spell: u32) -> u8 {
    if spell != AGONY {
        return 1;
    }
    world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == SUDDEN_ONSET && aura.caster == caster)
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 1))
        .map_or(1, |effect| effect.amount.max(1.0) as u8)
}

/// Due ticks are collected before execution. Read this owner's live stack for
/// each damage event, including deadline flushes; keep normal damage modifiers
/// downstream rather than multiplying an already stacked/modified snapshot.
pub(crate) fn tick_damage(world: &World, caster: Entity, target: Entity, _amount: f32) -> f32 {
    let aura = world
        .get::<ActiveAuras>(target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == AGONY && aura.caster == caster)
        })
        .expect("Agony damage requires its live owned aura");
    let effect = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Agony requires its native periodic damage effect");
    effect.amount * aura.stack_multiplier()
}

/// Damage is resolved first. Immune ticks never call this; a fully absorbed
/// landed tick still advances. A lethal tick may already have removed its aura.
pub(super) fn advance_stack(world: &mut World, target: Entity, instance: u32) {
    let Some(mut auras) = world.get_mut::<ActiveAuras>(target) else {
        return;
    };
    let Some(aura) = auras.get_mut(instance) else {
        return;
    };
    let next = aura.stacks.saturating_add(1).min(aura.max_stacks.max(1));
    if next != aura.stacks {
        aura.stacks = next;
        super::auras_changed(world, target);
    }
}
