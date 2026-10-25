//! SIMC_DEVASTATION_SURGE_AZURE_SWEEP and native MID1 tier relationships.
//! SimC76f8118 sc_evoker.cpp5592–5599/5702–5708/10853–10856.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::AfterCastHook;
use crate::auras::{SpellModOp, has_aura, spell_mods};
use crate::spell_info::{SpellInfo, SpellStore};

const AZURE_SWEEP: u32 = 1265872;
const AZURE_SWEEP_TALENT: u32 = 1265867;
const AZURE_SWEEP_BUFF: u32 = 1265871;
const ETERNITY_SURGE: u32 = 359073;
const MID1_TWO_PIECE: u32 = 1264821;
const MID1_FOUR_PIECE: u32 = 1264822;

pub(super) fn hook(spell: u32) -> Option<AfterCastHook> {
    match spell {
        359073 | 382411 => Some(grant_sweep_uses),
        AZURE_SWEEP => Some(reduce_surge_cooldown),
        _ => None,
    }
}

/// SIMC_DEVASTATION_MID1_SWEEP_PRIMARY: sc_evoker.cpp5691–5698.
/// Native effect0 already modifies all recipients; effect1 is primary-only.
pub(in crate::spell_cast) fn damage_multiplier(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
) -> f32 {
    let selected_primary = spell.id == AZURE_SWEEP && spell.cast_primary_target == Some(target);
    if !selected_primary {
        return 1.0;
    }
    if !is_devastation(world, caster) || !has_aura(world, caster, MID1_TWO_PIECE) {
        return 1.0;
    }
    let percent = super::script_effect_value(world, caster, MID1_TWO_PIECE, 1);
    1.0 + percent / 100.0
}

fn is_devastation(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 1467)
}

fn grant_sweep_uses(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if !is_devastation(world, caster) || !has_aura(world, caster, AZURE_SWEEP_TALENT) {
        return;
    }
    let buff = world
        .resource::<SpellStore>()
        .get(AZURE_SWEEP_BUFF)
        .expect("native Azure Sweep buff metadata");
    // MID1 four-piece's native label5988/Charges31 adds to the ordinary one use.
    let uses = spell_mods::modify(world, caster, buff, SpellModOp::Charges, 1.0) as u32;
    let depth = super::super::next_trigger_depth(world);
    for _ in 0..uses {
        if !crate::spell_cast::try_cast_triggered(
            world,
            caster,
            AZURE_SWEEP_BUFF,
            Some(caster),
            depth,
        ) {
            warn!(?caster, "Native Azure Sweep use grant refused");
            break;
        }
    }
}

fn reduce_surge_cooldown(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if !is_devastation(world, caster) || !has_aura(world, caster, MID1_FOUR_PIECE) {
        return;
    }
    let milliseconds = super::script_effect_value(world, caster, MID1_FOUR_PIECE, 1);
    super::reduce_cooldown(
        world,
        caster,
        ETERNITY_SURGE,
        f64::from(milliseconds) / 1000.0,
    );
}
