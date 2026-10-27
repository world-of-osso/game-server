//! SimC76f8118 sc_monk.cpp889/1288–1309/1328–1359; native selected values.
use bevy::prelude::*;

use crate::auras::{self, ActiveAuras, RemoveReason};
use crate::class_progression::TraitLoadout;
use crate::random as rand;
use crate::spell_cast::{self, effects::EffectCtx, queued_triggers};
use crate::spell_info::SpellInfo;

const TALENT: u32 = 116645;
const BUFF: u32 = 202090;
const BLACKOUT_KICK: u32 = 100784;
pub(super) const DAMAGE: u32 = 228649;
const SPACING_MS: u32 = 100;

fn selected(world: &World, caster: Entity) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == TALENT)
    })
}

pub(super) fn tiger_palm_hit(world: &mut World, caster: Entity, depth: u8) {
    if selected(world, caster) {
        spell_cast::cast_triggered(world, caster, BUFF, Some(caster), depth + 1);
    }
}

/// The same base impact owns recovery for the main and extra BOK actions.
pub(super) fn blackout_hit(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    if !selected(world, caster) {
        return;
    }
    let chance = super::script_effect_value(world, caster, TALENT, 0);
    if rand::random::<f32>() * 100.0 < chance {
        super::windwalker::reset_rising_sun_kick(world, caster);
    }
}

pub(super) fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if spell.id != BLACKOUT_KICK || !selected(world, caster) {
        return;
    }
    let count = earned_stacks(world, caster);
    if count == 0 {
        return;
    }
    let Some(primary) = spell.cast_primary_target else {
        warn!(?caster, "Teachings release lacks the actual BOK primary");
        return;
    };
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == BUFF && aura.caster == caster
    });
    schedule_children(world, caster, spell, depth, primary, count);
}

fn schedule_children(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    depth: u8,
    primary: Entity,
    count: u8,
) {
    let ctx = EffectCtx {
        caster,
        spell,
        depth,
        dest: None,
        chain_multiplier: 1.0,
    };
    for index in 0..count {
        queued_triggers::schedule_full_trigger(
            world,
            &ctx,
            primary,
            DAMAGE,
            u32::from(index) * SPACING_MS,
        );
    }
}

fn earned_stacks(world: &World, caster: Entity) -> u8 {
    world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == BUFF && aura.caster == caster)
        })
        .map_or(0, |aura| aura.stacks)
}
