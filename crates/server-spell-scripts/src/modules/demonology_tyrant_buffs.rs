//! Selected Tyrant's Oblation and Abyssal Dominion buff the owner on each Tyrant summon.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellStore;
use shared::components::ActiveSpec;

const TYRANT: u32 = 265187;
const OBLATION_TALENT: u32 = 1276746;
const OBLATION: u32 = 1276767;
const ABYSSAL_DOMINION_TALENT: u32 = 429581;
const ABYSSAL_DOMINION: u32 = 456323;
const REIGN_OF_TYRANNY: u32 = 1276748;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::OnCast(after_tyrant)];

// SimC76f8118 actions.cpp3641–3645: buffs.tyrants_oblation and
// buffs.abyssal_dominion trigger on every Summon Demonic Tyrant execution.
fn after_tyrant(world: &mut World, caster: Entity, spell_id: u32, depth: u8) {
    if spell_id != TYRANT
        || world
            .get::<ActiveSpec>(caster)
            .is_none_or(|spec| spec.0 != 266)
    {
        return;
    }
    if auras::has_aura(world, caster, OBLATION_TALENT) {
        grant(world, caster, OBLATION, depth);
    }
    if auras::has_aura(world, caster, ABYSSAL_DOMINION_TALENT)
        && grant(world, caster, ABYSSAL_DOMINION, depth)
    {
        extend_dominion(world, caster);
    }
}

fn grant(world: &mut World, caster: Entity, buff: u32, depth: u8) -> bool {
    let granted = spell_cast::try_cast_triggered(world, caster, buff, Some(caster), depth + 1);
    if !granted {
        warn!(?caster, buff, "Native Tyrant summon buff refused");
    }
    granted
}

/// init.cpp935–936 adds Reign of Tyranny effect1 explicitly; the native
/// duration modifier's class mask does not cover this buff.
fn extend_dominion(world: &mut World, caster: Entity) {
    if !auras::has_aura(world, caster, REIGN_OF_TYRANNY) {
        return;
    }
    let base_ms = world
        .resource::<SpellStore>()
        .get(ABYSSAL_DOMINION)
        .expect("native Abyssal Dominion")
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .expect("native Abyssal Dominion duration")
        .duration_ms;
    let reign_ms = spell_cast::scripts::script_effect_value(world, caster, REIGN_OF_TYRANNY, 0);
    let total_ms = (base_ms as f32 + reign_ms).round() as u32;
    assert!(auras::set_owned_duration(
        world,
        caster,
        caster,
        ABYSSAL_DOMINION,
        total_ms
    ));
}
