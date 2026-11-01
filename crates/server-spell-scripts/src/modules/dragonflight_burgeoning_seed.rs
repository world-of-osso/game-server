//! SimC76f8118 unique_gear_dragonflight.cpp963–1006: Burgeoning Seed consumes
//! every Brimming Life-Pod into one Supernatural stack each. Its DB2 use spell
//! has only dummy auras, so the consumption is script-owned.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::AfterCastHook;
use combat::spell_cast::{self, TriggeredSpellOverrides};
use combat::spell_info::SpellInfo;

const CONSUME: u32 = 384636;
const LIFE_POD: u32 = 384646;
const SUPERNATURAL: u32 = 384658;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::NativeAfterCast(hook)];

fn hook(spell: u32) -> Option<AfterCastHook> {
    (spell == CONSUME).then_some(consume_pods)
}

fn read_pod_stacks(world: &World, caster: Entity) -> Option<u8> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == LIFE_POD && aura.caster == caster)
        .map(|aura| aura.stacks)
}

fn consume_pods(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let Some(stacks) = read_pod_stacks(world, caster) else {
        return;
    };
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == LIFE_POD && aura.caster == caster
    });
    let item_level = spell
        .cast_item_level
        .expect("Burgeoning Seed use carries its item level");
    if !spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        SUPERNATURAL,
        Some(caster),
        1,
        TriggeredSpellOverrides {
            item_level: Some(item_level),
            aura_stacks: Some(stacks),
            ..Default::default()
        },
    ) {
        warn!(?caster, stacks, "Supernatural cast was refused");
    }
}
