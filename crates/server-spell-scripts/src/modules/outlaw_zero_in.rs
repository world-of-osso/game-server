//! Snapshot and consume earned Zero In on Between the Eyes.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, RemoveReason};
use combat::spell_cast::script_registry::SpellScriptHook;

const BTE: u32 = 315341;
const ZERO_IN: u32 = 1259486;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SnapshotsModifier(snapshots_zero_in),
    SpellScriptHook::AfterNativeOnCast(consume_zero_in),
];

fn snapshots_zero_in(spell: u32) -> bool {
    spell == ZERO_IN
}

fn consume_zero_in(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if spell != BTE || depth != 0 {
        return;
    }
    // SimC76f8118 sc_rogue.cpp3871: BtE snapshots the earned modifiers
    // at launch and then expires all Zero In stacks, including on a miss.
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == ZERO_IN && aura.caster == caster
    });
}
