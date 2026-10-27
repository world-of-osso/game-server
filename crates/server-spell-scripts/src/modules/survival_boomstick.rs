//! Boomstick uses earned Tip through the existing Hunter consumers.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::BeforeProcLaunch(spend_tip),
    SpellScriptHook::LateAuraRegistration(register),
];

const BOOMSTICK: u32 = 1261193;
const TIP: u32 = 260286;
const BOOMSTICK_TIP: u32 = 471536;

fn spend_tip(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // SimC76f8118 sc_hunter.cpp6304–6324: real channel execution spends
    // one earned use, grants its native buff and calls the shared consumers.
    if spell.id != BOOMSTICK || !owns_tip(world, caster) {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    super::cast_relationships::decrement(world, caster, TIP);
    spell_cast::cast_triggered(world, caster, BOOMSTICK_TIP, Some(caster), 1);
    super::survival_buffs::after_tip_impact(world, caster);
    super::marksmanship_sentinel::roll_survival_mark(world, caster, target);
    super::survival_pet::strike_after_tip(world, caster, target, 1);
}

fn owns_tip(world: &World, caster: Entity) -> bool {
    world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| aura.spell_id == TIP && aura.caster == caster)
    })
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(BOOMSTICK).or_default().on_remove = Some(clear_tip_buff);
}

fn clear_tip_buff(world: &mut World, unit: Entity, aura: &AuraInstance, _: RemoveReason) {
    // sc_hunter.cpp6344–6350: native buff lasts only through the channel.
    if aura.caster == unit {
        auras::remove_auras(world, unit, RemoveReason::Cancelled, |buff| {
            buff.spell_id == BOOMSTICK_TIP && buff.caster == unit
        });
    }
}
