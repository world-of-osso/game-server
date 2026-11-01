//! Native 70448 Rushing Wind Kick heals allies carrying the Monk's healing-over-time effects.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, handlers::PERIODIC_HEAL};
use combat::spell_cast::{
    self,
    script_registry::{AreaTargetFilter, SpellScriptHook},
    scripts::AfterCastHook,
};
use combat::spell_info::SpellInfo;

const KICK: u32 = 467_307;
const HEAL: u32 = 1_269_159;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AuraRollAfterCast(after_cast),
    SpellScriptHook::AreaFilter(area_filter),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == KICK).then_some(heal_misted_allies)
}

fn heal_misted_allies(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !world
        .get::<shared::components::ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 270)
    {
        return;
    }
    if !spell_cast::try_cast_triggered(world, caster, HEAL, Some(caster), 1) {
        warn!(?caster, "Rushing Wind Kick heal failed");
    }
}

fn area_filter(spell: u32, effect: u8) -> Option<AreaTargetFilter> {
    (spell == HEAL && effect == 0).then_some(select_misted_allies)
}

fn select_misted_allies(
    world: &World,
    _: &SpellInfo,
    caster: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    let Some(caster) = caster else {
        units.clear();
        return;
    };
    units.retain(|unit| {
        world.get::<ActiveAuras>(*unit).is_some_and(|auras| {
            auras.iter().any(|aura| {
                aura.caster == caster
                    && aura
                        .effects
                        .iter()
                        .any(|effect| effect.aura_type == PERIODIC_HEAL)
            })
        })
    });
    let count = spell_cast::scripts::script_effect_value(world, caster, KICK, 2).max(0.0) as usize;
    units.truncate(count);
}
