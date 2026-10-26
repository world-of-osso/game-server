//! SimC76f8118 sc_shaman.cpp2281–2287/7010–7038/10146–10149:
//! equipped MID2 pieces own Burning Core ticks and Fire Nova's recharge reward.
use bevy::prelude::*;

use super::combat;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::{self, AuraInstance};
use combat::spell_cast;
use combat::spell_info::SpellInfo;

const TWO_PIECE: u32 = 1296627;
const FOUR_PIECE: u32 = 1296628;
const BURNING_CORE: u32 = 1299975;
const FIRE_NOVA: u32 = 333977;
const SHORT_CIRCUIT: u32 = 1299991;
const CRASH_LIGHTNING: u32 = 187874;

pub(crate) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        BURNING_CORE,
        AuraScript {
            on_periodic: Some(tick_burning_core),
            ..Default::default()
        },
    );
}

pub(super) fn apply_burning_core(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !auras::has_aura(world, caster, TWO_PIECE) {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    spell_cast::cast_triggered(world, caster, BURNING_CORE, Some(target), 1);
}

fn tick_burning_core(world: &mut World, target: Entity, aura: &AuraInstance, index: u8) {
    if index == 0 {
        // Native1299975 effect0 supplies the tick period; effect1 supplies its caster damage modifier.
        spell_cast::cast_triggered(world, aura.caster, FIRE_NOVA, Some(target), 1);
    }
}

pub(super) fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if spell.id != FIRE_NOVA || !auras::has_aura(world, caster, FOUR_PIECE) {
        return;
    }
    // Once per explosion execution, not per area recipient, including background variants.
    let milliseconds = super::script_effect_value(world, caster, FOUR_PIECE, 0);
    super::reduce_cooldown(
        world,
        caster,
        CRASH_LIGHTNING,
        f64::from(milliseconds) / 1000.0,
    );
    spell_cast::cast_triggered(world, caster, SHORT_CIRCUIT, Some(caster), depth + 1);
}
