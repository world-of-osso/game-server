//! TC SpellInfo::CanBeInterrupted, evaluated with live caster/target aura state.
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{Health, Player};

use crate::auras::{ActiveAuras, handlers, spell_mods};
use crate::spell_info::{SpellInfo, SpellStore};

const ATTR7_NO_UI_NOT_INTERRUPTIBLE: u32 = 0x0000_2000;
const CHANNEL_DAMAGE_OR_ENTERING_COMBAT: u32 = 0x0000_0002 | 0x1000_0000;
const DAMAGE_CANCELS_PLAYER_ONLY: u32 = 0x10;
const DAMAGE_CANCELS: u32 = 0x400;
const PREVENTION_SILENCE: u32 = 1;
const MECHANIC_INTERRUPT: u32 = 26;

/// The legacy CastState.interruptible field controls damage pushback, not kick authority.
pub(crate) fn can_interrupt_cast(world: &World, interrupter: Entity, target: Entity) -> bool {
    let Some(cast) = world.get::<CastState>(target) else {
        return false;
    };
    if cast.duration <= 0.0 || cast.elapsed >= cast.duration {
        return false;
    }
    let Some(spell) = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(cast.spell_id))
    else {
        warn!(
            ?target,
            spell = cast.spell_id,
            "Interrupt eligibility has no source spell"
        );
        return false;
    };
    can_be_interrupted(world, spell, Some(interrupter), target, false)
}

pub(super) fn can_be_interrupted(
    world: &World,
    spell: &SpellInfo,
    interrupter: Option<Entity>,
    target: Entity,
    ignore_immunity: bool,
) -> bool {
    if intrinsic_override(world, spell, target) || caster_allows_spell(world, interrupter, spell.id)
    {
        return true;
    }
    let immune =
        crate::auras::mechanic_immunity::has_mechanic_immunity(world, target, MECHANIC_INTERRUPT);
    let protected =
        spell_mods::has_aura_type_affecting(world, target, handlers::PREVENT_INTERRUPT, spell);
    let prevention = spell
        .categories
        .as_ref()
        .map_or(0, |category| category.prevention_type);
    (ignore_immunity || !immune) && !protected && prevention & PREVENTION_SILENCE != 0
}

fn intrinsic_override(world: &World, spell: &SpellInfo, target: Entity) -> bool {
    if super::attributes(spell, 7) & ATTR7_NO_UI_NOT_INTERRUPTIBLE != 0 {
        return true;
    }
    spell.interrupts.as_ref().is_some_and(|flags| {
        flags.channel_interrupt_flags[0] & CHANNEL_DAMAGE_OR_ENTERING_COMBAT != 0
            || flags.interrupt_flags & DAMAGE_CANCELS != 0
            || world.get::<Player>(target).is_some()
                && flags.interrupt_flags & DAMAGE_CANCELS_PLAYER_ONLY != 0
    })
}

fn caster_allows_spell(world: &World, interrupter: Option<Entity>, spell: u32) -> bool {
    interrupter
        .filter(|unit| world.get::<Health>(*unit).is_some())
        .and_then(|unit| world.get::<ActiveAuras>(unit))
        .is_some_and(|auras| {
            auras.iter().any(|aura| {
                aura.effects.iter().any(|effect| {
                    effect.aura_type == handlers::ALLOW_INTERRUPT_SPELL
                        && u32::try_from(effect.misc_values[0]) == Ok(spell)
                })
            })
        })
}
