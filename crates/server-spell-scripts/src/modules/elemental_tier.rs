//! Native MID2 Flowing Elements and Overcharge lifecycle.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::AfterElementalLaunch(after_launch),
    SpellScriptHook::OwnsPolicyProc(owns_tier_proc),
];
const SET: u32 = 1296626;
const STORMKEEPER: u32 = 191634;
const ASCENDANCE: u32 = 1219480;
const FLOWING: u32 = 1300219;
const OVERCHARGE: u32 = 1300222;

#[derive(Component, Default)]
struct BufferedFlowing;

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(STORMKEEPER).or_default().on_remove = Some(stormkeeper_ended);
    scripts.0.entry(ASCENDANCE).or_default().on_remove = Some(ascendance_ended);
}

fn owns_tier_proc(spell: u32) -> bool {
    matches!(spell, FLOWING | OVERCHARGE)
}

fn equipped(world: &World, caster: Entity) -> bool {
    auras::has_aura(world, caster, SET)
}

fn stormkeeper_ended(
    world: &mut World,
    caster: Entity,
    aura: &AuraInstance,
    _reason: RemoveReason,
) {
    if caster != aura.caster || !equipped(world, caster) {
        return;
    }
    // SimC76f8118 sc_shaman.cpp12636–12651: overlap buffers the uses.
    if auras::has_aura(world, caster, ASCENDANCE) {
        world.entity_mut(caster).insert(BufferedFlowing);
    } else {
        grant_flowing(world, caster, false);
    }
}

fn ascendance_ended(world: &mut World, caster: Entity, aura: &AuraInstance, _reason: RemoveReason) {
    if caster != aura.caster || !equipped(world, caster) {
        return;
    }
    // SimC76f8118 sc_shaman.cpp12600–12608: expiry grants two or buffered four.
    let buffered = world.entity_mut(caster).take::<BufferedFlowing>().is_some();
    grant_flowing(world, caster, buffered);
}

fn grant_flowing(world: &mut World, caster: Entity, buffered: bool) {
    let count = spell_cast::scripts::script_effect_value(world, caster, SET, 0) as u8;
    if !spell_cast::try_cast_triggered(world, caster, FLOWING, Some(caster), 1) {
        warn!(?caster, "Native Flowing Elements grant refused");
        return;
    }
    auras::set_self_cast_stacks(
        world,
        caster,
        FLOWING,
        if buffered { count * 2 } else { count },
    );
}

fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo, foreground: bool) {
    if !foreground || spell.cast_parent_owns_modifier_charges || !equipped(world, caster) {
        return;
    }
    // The last Stormkeeper builder cannot spend the Flowing it just earned.
    if spell
        .cast_charge_mods
        .iter()
        .flatten()
        .any(|modifier| modifier.aura_spell_id == STORMKEEPER)
    {
        return;
    }
    let instance = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| {
                aura.spell_id == FLOWING
                    && aura.caster == caster
                    && aura.effects.iter().any(|effect| {
                        effect.index == 0
                            && auras::spell_mods::class_mask_affects(aura, effect, spell)
                    })
            })
            .map(|aura| aura.instance_id)
    });
    let Some(instance) = instance else { return };
    // SimC76f8118 sc_shaman.cpp12914–12924: spending a builder use earns the free spender.
    auras::spend_stack(world, caster, instance);
    if !spell_cast::try_cast_triggered(world, caster, OVERCHARGE, Some(caster), 1) {
        warn!(?caster, "Native Overcharge grant refused");
    }
}
