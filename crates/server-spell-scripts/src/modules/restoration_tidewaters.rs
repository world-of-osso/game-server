//! Tidewaters heals each Riptide recipient when its rain or totem is cast.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{
    self,
    script_registry::SpellScriptHook,
    scripts::AfterCastHook,
    targets::{self, Relation},
};
use combat::spell_info::{SpellInfo, SpellStore};

const TALENT: u32 = 462_424;
const HEAL: u32 = 462_425;
const RIPTIDE: u32 = 61_295;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::NativeAfterCast(after_cast)];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 73_920 | 444_995).then_some(heal_riptides)
}

fn heal_riptides(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if !combat::auras::has_aura(world, caster, TALENT) {
        return;
    }
    let range = world
        .resource::<SpellStore>()
        .get(HEAL)
        .expect("Tidewaters heal missing")
        .misc
        .as_ref()
        .and_then(|misc| misc.range)
        .expect("Tidewaters range missing")
        .max_friendly;
    let Some(center) = targets::position(world, caster) else {
        return;
    };
    let allies = targets::area_units(world, caster, center, range, Relation::Ally, 0);
    for ally in allies {
        let owned = world.get::<ActiveAuras>(ally).is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == RIPTIDE && aura.caster == caster)
        });
        if owned {
            spell_cast::cast_triggered(world, caster, HEAL, Some(ally), 1);
        }
    }
}
