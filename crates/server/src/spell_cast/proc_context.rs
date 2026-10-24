//! Actual proc-event context for the existing ordinary CAST and aura-only HIT events
//! (TC a352b1fa Spell.cpp3900–3918, 2849–2871). Only frozen per-cast receipts are
//! copied; applied-modifier provenance is not captured yet and stays unavailable.

use bevy::prelude::*;

use super::{CostPool, TargetPlan, effects};
use crate::auras::procs::ProcEventContext;
use crate::spell_info::SpellInfo;

/// TC SpellMgr.h ProcFlagsSpellPhase / ProcFlagsSpellType / ProcFlagsHit.
const PROC_SPELL_PHASE_CAST: u32 = 0x1;
const PROC_SPELL_PHASE_HIT: u32 = 0x2;
const PROC_SPELL_TYPE_MASK_ALL: u32 = 0x7;
const PROC_SPELL_TYPE_NO_DMG_HEAL: u32 = 0x4;
const PROC_HIT_NORMAL: u32 = 0x1;

/// TC GetPowerCost: a positive Amount in the final prepared vector, even when waived.
pub(super) fn has_power_cost(costs: &[(CostPool, f32)]) -> bool {
    costs.iter().any(|(_, amount)| *amount > 0.0)
}

/// Successful ordinary CAST: pre-hit phase, normal hit, no DamageInfo attack type.
pub(super) fn cast(spell: &SpellInfo) -> Option<ProcEventContext> {
    frozen(spell, PROC_SPELL_PHASE_CAST, PROC_SPELL_TYPE_MASK_ALL)
}

/// A landed hit whose every landed effect applies an aura, which resolved no damage
/// (not even zero or fully absorbed) and no healing, and whose plan proves it is the
/// original unblocked selection rather than a reflected copy. Anything else stays None.
pub(super) fn aura_only_hit(
    spell: &SpellInfo,
    plan: &TargetPlan,
    target: Entity,
    damage_recorded: bool,
    heal: Option<f32>,
) -> Option<ProcEventContext> {
    if damage_recorded || heal.is_some() {
        return None;
    }
    let mut landed = plan
        .iter()
        .filter(|planned| planned.units.contains(&target))
        .peekable();
    landed.peek()?;
    let aura_only = landed.all(|planned| {
        let effect = &spell.effects[planned.index];
        crate::auras::is_aura_effect(effect)
            && effect.effect != effects::EFFECT_PERSISTENT_AREA_AURA
            // Reflected landings carry no original selection for the caster.
            && planned.selected_units.contains(&target)
            && !planned.blocked_units.contains(&target)
    });
    if !aura_only {
        return None;
    }
    frozen(spell, PROC_SPELL_PHASE_HIT, PROC_SPELL_TYPE_NO_DMG_HEAL)
}

fn frozen(spell: &SpellInfo, phase: u32, spell_type: u32) -> Option<ProcEventContext> {
    let school_mask = spell.misc.as_ref()?.school_mask;
    let has_power_cost = spell.cast_has_power_cost?;
    Some(ProcEventContext {
        school_mask,
        spell_type_mask: spell_type,
        spell_phase_mask: phase,
        hit_mask: PROC_HIT_NORMAL,
        attack_type: None,
        has_power_cost,
        cast_from_item: spell.cast_from_item,
        applied_aura_modifiers: None,
    })
}
