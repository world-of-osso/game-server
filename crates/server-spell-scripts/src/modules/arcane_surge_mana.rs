//! Arcane Surge snapshots the mana-dependent damage multiplier before spending mana.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;
use shared::components::Mana;

const SURGE: u32 = 365350;
const MANA_BOMB: u32 = 457521;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::BeforeDamageSnapshot(snapshot_mana)];

fn snapshot_mana(world: &World, caster: Entity, spell: &mut SpellInfo) {
    if spell.id != SURGE {
        return;
    }
    let mana = world.get::<Mana>(caster).expect("Arcane Surge requires mana");
    let mana_fraction = (mana.current / mana.max).clamp(0.0, 1.0);
    let bomb = if combat::auras::has_aura(world, caster, MANA_BOMB) {
        1.0 + script_effect_value(world, caster, MANA_BOMB, 0) / 100.0
    } else {
        1.0
    };
    // SimC 76f8118 sc_mage.cpp3334–3342: interpolate from 1× at zero mana
    // to the native s2 multiplier, modified by selected Mana Bomb, at full mana.
    let maximum = script_effect_value(world, caster, SURGE, 1) * bomb;
    spell.cast_damage_multiplier = Some(
        spell.cast_damage_multiplier.unwrap_or(1.0) * (1.0 + mana_fraction * (maximum - 1.0)),
    );
}
