//! Frost launch-time proc ownership; no live-aura guesses at missile impact.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use crate::auras::{self, ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::{self, effects::EffectCtx};
use crate::spell_info::{SpellInfo, SpellStore};

pub(super) const FINGERS: u32 = 44544;
pub(super) const THERMAL: u32 = 1247730;
const BRAIN_FREEZE: u32 = 190446;

fn is_frost(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 64)
}

pub(super) fn owns_proc(spell: u32) -> bool {
    matches!(spell, FINGERS | THERMAL | BRAIN_FREEZE)
}

pub(in crate::spell_cast) fn snapshot_launch_auras(
    world: &World,
    caster: Entity,
    spell: &mut SpellInfo,
) {
    if !is_frost(world, caster) {
        return;
    }
    // SimC76f8118 sc_mage.cpp4445–4449/4002–4015: launch snapshots
    // survive consumption, expiry and later applications while in flight.
    let sources: &[u32] = match spell.id {
        30455 => &[FINGERS, THERMAL],
        44614 => &[BRAIN_FREEZE],
        _ => return,
    };
    spell.cast_launch_auras = world
        .get::<ActiveAuras>(caster)
        .map_or_else(Vec::new, |auras| {
            auras
                .iter()
                .filter(|aura| aura.caster == caster && sources.contains(&aura.spell_id))
                .map(|aura| aura.spell_id)
                .collect()
        });
}

pub(super) fn launch_lance_payload(world: &mut World, ctx: &EffectCtx, target: Entity) {
    let Some(mut payload) = spell_cast::load_triggered_spell(world, 228598, ctx.depth + 1) else {
        return;
    };
    payload.cast_launch_auras = ctx.spell.cast_launch_auras.clone();
    spell_cast::launch_triggered_spell(world, ctx.caster, &payload, Some(target), ctx.depth + 1);
}

pub(super) fn after_cast_hook(spell: u32) -> Option<super::AfterCastHook> {
    matches!(spell, 84714 | 30455 | 44614).then_some(after_cast)
}

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !is_frost(world, caster) {
        return;
    }
    match spell.id {
        84714 => grant_orb_procs(world, caster, spell.cast_primary_target),
        // sc_mage.cpp4455–4462: spend at execution, not at impact.
        30455 => {
            for &aura in &spell.cast_launch_auras {
                spend_owned_stack(world, caster, aura);
            }
        }
        // sc_mage.cpp4012–4015: an actual earned Brain Freeze Flurry
        // consumes that proc and opens the selected native Thermal Void.
        44614 if spell.cast_launch_auras.contains(&BRAIN_FREEZE) => {
            spend_owned_stack(world, caster, BRAIN_FREEZE);
            if auras::has_aura(world, caster, 1247729) {
                apply_buff(world, caster, THERMAL);
            }
        }
        _ => {}
    }
}

fn apply_buff(world: &mut World, caster: Entity, spell: u32) {
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn grant_orb_procs(world: &mut World, caster: Entity, primary: Option<Entity>) {
    // sc_mage.cpp4186–4190: native selected Orb producers, independently
    // of the ground damage pulses and their recipients.
    if auras::has_aura(world, caster, 460590) {
        apply_buff(world, caster, 455122);
    }
    if auras::has_aura(world, caster, 385167) {
        let count = super::script_effect_value(world, caster, 385167, 0) as u32;
        for _ in 0..count {
            apply_buff(world, caster, FINGERS);
        }
        // sc_mage.cpp7457–7460: guaranteed Fingers is immediately reacted.
        crate::simc_apl_native::reaction::predict(world, caster, FINGERS);
    }
    if auras::has_aura(world, caster, 378406) {
        let chance = super::script_effect_value(world, caster, 378406, 0);
        if crate::random::random::<f32>() * 100.0 < chance {
            grant_brain_freeze(world, caster, primary);
        }
    }
}

fn grant_brain_freeze(world: &mut World, caster: Entity, primary: Option<Entity>) {
    complete_brain_freeze(world, caster, false);
    super::frost_mage_supply::after_brain_freeze(world, caster, primary);
}

pub(super) fn grant_random_brain_freeze(world: &mut World, caster: Entity) {
    if auras::has_aura(world, caster, BRAIN_FREEZE) {
        super::frost_mage_events::schedule_brain_freeze(world, caster);
    } else {
        complete_brain_freeze(world, caster, true);
    }
}

pub(super) fn refresh_random_brain_freeze(world: &mut World, caster: Entity) {
    complete_brain_freeze(world, caster, true);
}

fn complete_brain_freeze(world: &mut World, caster: Entity, restore_one: bool) {
    apply_buff(world, caster, BRAIN_FREEZE);
    // sc_mage.cpp7412–7433: guaranteed producers reset all charges;
    // random producers restore one, with a150ms delay if BF was already up.
    let category = world
        .resource::<SpellStore>()
        .get(44614)
        .and_then(|spell| spell.categories.as_ref())
        .expect("native Flurry charge category")
        .charge_category;
    let count = if restore_one { 1 } else { u8::MAX };
    spell_cast::restore_charges(world, caster, category, count);
}

fn spend_owned_stack(world: &mut World, caster: Entity, spell: u32) {
    let instance = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == spell && aura.caster == caster)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        auras::spend_stack(world, caster, instance);
    }
}
