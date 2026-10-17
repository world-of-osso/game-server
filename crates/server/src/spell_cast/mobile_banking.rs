//! Mobile Banking: user decision2026-10-17 plus installed70448 child spells.
//! Friendly1168 is a chest-USER requirement, never a cast requirement.
use bevy::prelude::*;
use shared::components::{GuildMembership, Player};

use super::{CastFailure, effects::EffectCtx};
use crate::class_progression::KnownSpells;
use crate::spell_info::{SpellEffect, SpellStore};

pub(crate) const MOBILE_BANKING: u32 = 83958;
const ALLIANCE_CHEST: u32 = 88304;
const HORDE_CHEST: u32 = 88306;

fn child_spell(world: &World, caster: Entity) -> Result<u32, CastFailure> {
    let player = world.get::<Player>(caster).ok_or(CastFailure::NotKnown)?;
    match crate::faction::faction_for_race(player.race) {
        1 => Ok(ALLIANCE_CHEST),
        2 => Ok(HORDE_CHEST),
        _ => Err(CastFailure::Unsupported(
            "Mobile Banking requires an established faction".into(),
        )),
    }
}

pub(super) fn check_cast(world: &World, caster: Entity, spell: u32) -> Result<(), CastFailure> {
    if spell != MOBILE_BANKING {
        return Ok(());
    }
    let granted = world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.guild_grants().contains(&MOBILE_BANKING));
    if world.get::<GuildMembership>(caster).is_none() || !granted {
        return Err(CastFailure::NotKnown);
    }
    let child = child_spell(world, caster)?;
    let spell = world
        .resource::<SpellStore>()
        .get(child)
        .ok_or(CastFailure::UnknownSpell)?;
    for effect in &spell.effects {
        if let Some(reason) = crate::game_objects::summoned::unsupported_reason(world, effect) {
            return Err(CastFailure::Unsupported(reason));
        }
    }
    Ok(())
}

pub(super) fn summon_chest(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect) {
    match child_spell(world, ctx.caster) {
        Ok(child) => {
            super::cast_triggered(world, ctx.caster, child, Some(ctx.caster), ctx.depth + 1)
        }
        Err(error) => warn!(?error, "Mobile Banking faction changed during cast"),
    }
}
