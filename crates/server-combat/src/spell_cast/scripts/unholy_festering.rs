//! SimC76f8118 sc_death_knight.cpp10411–10503: ready ghouls and Scythe lifecycle.
use crate::auras::{ActiveAuras, RemoveReason};
use crate::random::Rng;
use crate::spell_cast;
use crate::spell_info::SpellInfo;
use bevy::prelude::*;

const FESTERING_STRIKE: u32 = 85948;
const FESTERING_SCYTHE: u32 = 458128;
const SCYTHE_TALENT: u32 = 455397;
const SCYTHE_READY: u32 = 458123;
const SCYTHE_TRACKER: u32 = 1241077;
const GHOULS_READY: u32 = 1254252;

pub(super) fn after_cast_hook(spell: u32) -> Option<super::AfterCastHook> {
    matches!(spell, FESTERING_STRIKE | FESTERING_SCYTHE).then_some(grant_ghouls_and_scythe)
}

fn grant_ghouls_and_scythe(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if world
        .get::<shared::components::ActiveSpec>(caster)
        .map(|spec| spec.0)
        != Some(252)
    {
        return;
    }
    grant_ready_ghouls(world, caster);
    if spell.id == FESTERING_SCYTHE {
        crate::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == SCYTHE_READY && aura.caster == caster
        });
        cast_self_buff(world, caster, SCYTHE_TRACKER);
    } else if world
        .get::<ActiveAuras>(caster)
        .is_some_and(|auras| auras.has_spell(SCYTHE_TALENT))
    {
        cast_self_buff(world, caster, SCYTHE_READY);
    }
}

fn grant_ready_ghouls(world: &mut World, caster: Entity) {
    // sc_death_knight.cpp10415–10417/10440–10446: exactly one uniform roll
    // from the native Festering Strike min/max fields per successful execution.
    let min = super::script_effect_value(world, caster, FESTERING_STRIKE, 2) as u32;
    let max = super::script_effect_value(world, caster, FESTERING_STRIKE, 3) as u32;
    assert!(min > 0 && max >= min, "native ready-ghoul count bounds");
    let count = crate::random::thread_rng().gen_range(min..=max);
    for _ in 0..count {
        cast_self_buff(world, caster, GHOULS_READY);
    }
}

fn cast_self_buff(world: &mut World, caster: Entity, spell: u32) {
    if !spell_cast::try_cast_triggered(world, caster, spell, Some(caster), 1) {
        warn!(?caster, spell, "Native Unholy ready-state cast refused");
    }
}
