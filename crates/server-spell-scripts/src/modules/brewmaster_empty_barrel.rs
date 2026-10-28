//! SimC76f8118 sc_monk.cpp2613–2616: Keg Smash consumes Empty Barrel.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraSource, RemoveReason};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

const EMPTY_BARREL: u32 = 1265307;
const EMPTY_BARREL_DAMAGE: u32 = 1265133;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResourceValues(cast_hook)];

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    match spell {
        121253 => Some(consume_empty_barrel),
        322507 | 1241059 | 115203 | 120954 => Some(grant_empty_barrel),
        _ => None,
    }
}

fn grant_empty_barrel(world: &mut World, caster: Entity, _: &SpellInfo) {
    // sc_monk.cpp6921–6941: Bring Me Another's third entry grants the buff
    // on a successful Celestial or Fortifying Brew, independently of entry 1.
    if !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == 1265141)
    }) {
        return;
    }
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: EMPTY_BARREL,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn consume_empty_barrel(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !auras::has_aura(world, caster, EMPTY_BARREL) {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        error!(
            ?caster,
            "Keg Smash cannot consume Empty Barrel without a primary target"
        );
        return;
    };
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == EMPTY_BARREL && aura.caster == caster
    });
    spell_cast::cast_triggered(world, caster, EMPTY_BARREL_DAMAGE, Some(target), 1);
}
