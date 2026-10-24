//! SIMC_RET_TIER_MID2_DIVINE_PURPOSE_RELATIONSHIP.
//! SimC76f8118 sc_paladin.hpp1665–1718 supplies only the scripted relationship.
//! Native70448 owns set eligibility, child effects, amounts and durations.
//! TC a352b1fa has no handler; the set roots' native Dummy effects have Trigger0.
use bevy::prelude::*;

use crate::auras::{self, RemoveReason};
use crate::spell_info::SpellInfo;

const TWO_PIECE: u32 = 1296660;
const FOUR_PIECE: u32 = 1296661;
const DIVINE_PURPOSE: u32 = 223819;
const DIVINE_POWER: u32 = 1305230;
const VERDICT_ARBITER: u32 = 1306161;
const STORM_ARBITER: u32 = 1306162;
const LIGHT_ARBITER: u32 = 1310461;
const DIVINE_ARBITER: u32 = 1306923;

fn arbiter_kind(spell: u32) -> Option<u32> {
    match spell {
        85256 | 383328 => Some(VERDICT_ARBITER),
        53385 => Some(STORM_ARBITER),
        427453 => Some(LIGHT_ARBITER),
        _ => None,
    }
}

fn opposite_kinds(kind: u32) -> [u32; 2] {
    match kind {
        VERDICT_ARBITER => [STORM_ARBITER, LIGHT_ARBITER],
        STORM_ARBITER => [LIGHT_ARBITER, VERDICT_ARBITER],
        LIGHT_ARBITER => [STORM_ARBITER, VERDICT_ARBITER],
        _ => unreachable!("registered Arbiter kind"),
    }
}

/// These script-owned charges are consumed by the actual different-kind spender,
/// not by every native PROC2_CAST_SUCCESSFUL event (including their applying cast).
pub(crate) fn owns_arbiter_charge(spell: u32) -> bool {
    matches!(spell, VERDICT_ARBITER | STORM_ARBITER | LIGHT_ARBITER)
}

fn consumed_purpose(spell: &SpellInfo) -> bool {
    spell
        .cast_charge_mods
        .iter()
        .flatten()
        .any(|modifier| modifier.aura_spell_id == DIVINE_PURPOSE && modifier.amount != 0.0)
}

pub(super) fn after_foreground_spender(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    target: Option<Entity>,
) {
    let Some(kind) = arbiter_kind(spell.id) else {
        return;
    };
    let consumed = consumed_purpose(spell);
    if consumed && auras::has_aura(world, caster, TWO_PIECE) {
        crate::spell_cast::cast_triggered(world, caster, DIVINE_POWER, Some(caster), 1);
    }
    if !auras::has_aura(world, caster, FOUR_PIECE) {
        return;
    }
    let opposite = opposite_kinds(kind);
    let pending = [kind, opposite[0], opposite[1]]
        .into_iter()
        .any(|buff| auras::has_aura(world, caster, buff));
    if consumed && !pending {
        crate::spell_cast::cast_triggered(world, caster, kind, Some(caster), 1);
    }
    if let Some(buff) = opposite
        .into_iter()
        .find(|&buff| auras::has_aura(world, caster, buff))
    {
        consume_opposite(world, caster, target, buff);
    }
}

fn consume_opposite(world: &mut World, caster: Entity, target: Option<Entity>, buff: u32) {
    let Some(target) = target else {
        warn!(
            ?caster,
            buff, "RET_TIER_PRIMARY_TARGET_UNAVAILABLE: no native cast target"
        );
        return;
    };
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == buff
    });
    if !crate::spell_cast::try_cast_triggered(world, caster, DIVINE_ARBITER, Some(target), 1) {
        warn!(
            ?caster,
            ?target,
            "Ret tier native Divine Arbiter1306923 refused"
        );
    }
}
