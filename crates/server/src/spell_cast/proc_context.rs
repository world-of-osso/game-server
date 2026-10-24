//! Forward actual evaluation inputs; never reconstruct crit/type from aggregate loss.
use bevy::prelude::*;

use super::{CostPool, TargetPlan, effects};
use crate::auras::procs::ProcEventContext;
use crate::spell_info::SpellInfo;

pub(super) const PHASE_CAST: u32 = 1;
pub(super) const PHASE_HIT: u32 = 2;
pub(super) const TYPE_DAMAGE: u32 = 1;
pub(super) const TYPE_HEAL: u32 = 2;
const TYPE_NO_DAMAGE_HEAL: u32 = 4;
const TYPE_ALL: u32 = 7;
const HIT_NORMAL: u32 = 1;
const HIT_CRITICAL: u32 = 2;
const HIT_BLOCK: u32 = 0x40;
const HIT_ABSORB: u32 = 0x400;
const HIT_FULL_BLOCK: u32 = 0x2000;

/// Result bits from the actual damage/heal operation, retained separately from totals.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ResolvedProcHit {
    pub spell_type_mask: u32,
    pub hit_mask: u32,
}

impl ResolvedProcHit {
    pub fn damage(crit: bool, blocked: f32, absorbed: f32) -> Self {
        let mut hit_mask = if crit { HIT_CRITICAL } else { HIT_NORMAL };
        if blocked > 0.0 {
            hit_mask |= HIT_BLOCK;
        }
        if absorbed > 0.0 {
            hit_mask |= HIT_ABSORB;
        }
        Self {
            spell_type_mask: TYPE_DAMAGE,
            hit_mask,
        }
    }

    pub fn healing(crit: bool) -> Self {
        Self {
            spell_type_mask: TYPE_HEAL,
            hit_mask: if crit { HIT_CRITICAL } else { HIT_NORMAL },
        }
    }

    /// TC createProcHitMask: nullified damage has no NORMAL/CRITICAL bit.
    pub fn mask_nullified_damage(&mut self, full_block: bool, full_absorb: bool) {
        if full_block {
            self.hit_mask |= HIT_BLOCK | HIT_FULL_BLOCK;
        }
        if full_absorb {
            self.hit_mask |= HIT_ABSORB;
        }
        if full_block || full_absorb {
            self.hit_mask &= !(HIT_NORMAL | HIT_CRITICAL);
        }
    }

    pub fn record_absorb(&mut self, full_absorb: bool) {
        self.hit_mask |= HIT_ABSORB;
        self.mask_nullified_damage(false, full_absorb);
    }

    pub fn merge(&mut self, other: Self) {
        self.spell_type_mask |= other.spell_type_mask;
        self.hit_mask |= other.hit_mask;
    }
}

pub(super) fn capture_cast_origin(
    world: &World,
    caster: Entity,
    spell: &mut SpellInfo,
    triggered: bool,
) {
    spell.cast_proc_chain_length = Some(crate::auras::proc_chain::current(world, caster));
    spell.cast_is_triggered = Some(triggered);
    spell.cast_from_item.get_or_insert(false);
}

pub(super) fn has_power_cost(costs: &[(CostPool, f32)]) -> bool {
    costs.iter().any(|(_, amount)| *amount > 0.0)
}

pub(super) fn cast(spell: &SpellInfo) -> Option<ProcEventContext> {
    from_spell(
        spell,
        PHASE_CAST,
        ResolvedProcHit {
            spell_type_mask: TYPE_ALL,
            hit_mask: HIT_NORMAL,
        },
    )
}

pub(super) fn hit(spell: &SpellInfo, result: ResolvedProcHit) -> Option<ProcEventContext> {
    from_spell(spell, PHASE_HIT, result)
}

/// Only an actually landed, unblocked aura-only selection supplies this no-damage result.
pub(super) fn aura_only_hit(
    spell: &SpellInfo,
    plan: &TargetPlan,
    target: Entity,
) -> Option<ProcEventContext> {
    let mut landed = plan
        .iter()
        .filter(|planned| planned.units.contains(&target))
        .peekable();
    landed.peek()?;
    let aura_only = landed.all(|planned| {
        let effect = &spell.effects[planned.index];
        crate::auras::is_aura_effect(effect)
            && effect.effect != effects::EFFECT_PERSISTENT_AREA_AURA
            && planned.selected_units.contains(&target)
            && !planned.blocked_units.contains(&target)
    });
    if !aura_only {
        return None;
    }
    from_spell(
        spell,
        PHASE_HIT,
        ResolvedProcHit {
            spell_type_mask: TYPE_NO_DAMAGE_HEAL,
            hit_mask: HIT_NORMAL,
        },
    )
}

pub(super) fn periodic(spell: &SpellInfo, result: ResolvedProcHit) -> Option<ProcEventContext> {
    let mut context = from_spell(spell, PHASE_HIT, result)?;
    // TC ticks pass DamageInfo/HealInfo but no live Spell pointer.
    context.has_power_cost = None;
    context.cast_from_item = None;
    context.applied_aura_modifiers = None;
    context.proc_chain_length = None;
    context.is_triggered = None;
    Some(context)
}

/// TC Unit.cpp292–300: no Spell/DamageInfo/HealInfo means explicit school NONE.
/// The caller supplies the phase/type/hit facts of its distinct inline source.
pub(crate) fn without_spell(
    school_mask: u32,
    phase: u32,
    result: ResolvedProcHit,
    attack_type: Option<crate::auras::procs::ProcAttackType>,
) -> ProcEventContext {
    ProcEventContext {
        school_mask,
        spell_type_mask: result.spell_type_mask,
        spell_phase_mask: phase,
        hit_mask: result.hit_mask,
        attack_type,
        has_power_cost: None,
        cast_from_item: None,
        applied_aura_modifiers: None,
        proc_chain_length: None,
        is_triggered: None,
    }
}

fn from_spell(spell: &SpellInfo, phase: u32, result: ResolvedProcHit) -> Option<ProcEventContext> {
    Some(ProcEventContext {
        school_mask: spell.misc.as_ref()?.school_mask,
        spell_type_mask: result.spell_type_mask,
        spell_phase_mask: phase,
        hit_mask: result.hit_mask,
        attack_type: None,
        has_power_cost: spell.cast_has_power_cost,
        cast_from_item: spell.cast_from_item,
        applied_aura_modifiers: spell.cast_applied_modifiers.clone(),
        proc_chain_length: spell.cast_proc_chain_length,
        is_triggered: spell.cast_is_triggered,
    })
}
