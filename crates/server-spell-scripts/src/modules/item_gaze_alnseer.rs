//! SimC76f8118 unique_gear_midnight.cpp2020–2098: equipped Gaze's hit-earned stat stacks.
use super::combat;
use bevy::prelude::*;
use combat::auras::procs;
use combat::auras::scripts::AuraScripts;
use combat::auras::{AuraApplication, AuraInstance, AuraSource, RemoveReason};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellStore;

const DRIVER: u32 = 1256896;
const ALNSIGHT: u32 = 1266686;
const STAT: u32 = 1266687;

#[derive(Component, Clone, Copy)]
enum BypassDeadline {
    AfterNextProc,
    Restore(f64),
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::MidAuraRegistration(register),
    SpellScriptHook::AfterAuraApplication(after_application),
];

fn register(scripts: &mut AuraScripts) {
    let entry = scripts.0.entry(ALNSIGHT).or_default();
    entry.check_proc = Some(is_simc_hit);
    entry.on_proc = Some(grant_stat);
    entry.on_remove = Some(clear_bypass);
}

fn is_simc_hit(_world: &World, _aura: &AuraInstance, event: &procs::AuraEvent) -> bool {
    // SimC emits no action hit for Alnsight activation or Permeating Chill's native helper.
    !matches!(event.spell_id, Some(ALNSIGHT | 370898))
}

fn after_application(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.spell_id != ALNSIGHT || aura.caster != unit {
        return;
    }
    let now = combat::spell_cast::now(world);
    let Some(deadline) = procs::proc_deadline(world, unit, aura.instance_id) else {
        return;
    };
    if deadline <= now {
        return;
    }
    world.entity_mut(unit).insert(BypassDeadline::AfterNextProc);
}

fn clear_bypass(world: &mut World, unit: Entity, _aura: &AuraInstance, _reason: RemoveReason) {
    world.entity_mut(unit).remove::<BypassDeadline>();
}

fn grant_stat(world: &mut World, unit: Entity, aura: &AuraInstance, _other: Option<Entity>) {
    match world.get::<BypassDeadline>(unit).copied() {
        Some(BypassDeadline::AfterNextProc) => {
            let deadline = procs::proc_deadline(world, unit, aura.instance_id)
                .expect("active Alnsight proc deadline");
            let now = combat::spell_cast::now(world);
            assert!(procs::set_proc_deadline(world, unit, aura.instance_id, now));
            world
                .entity_mut(unit)
                .insert(BypassDeadline::Restore(deadline));
        }
        Some(BypassDeadline::Restore(deadline)) => {
            assert!(procs::set_proc_deadline(
                world,
                unit,
                aura.instance_id,
                deadline
            ));
            world.entity_mut(unit).remove::<BypassDeadline>();
        }
        None => {}
    }
    let Some(item_level) = aura.cast_item_level else {
        error!(?unit, "Alnsight proc lacks its equipped item level");
        return;
    };
    let store = world.resource::<SpellStore>();
    let mut driver = store.get(DRIVER).expect("native Gaze driver").clone();
    driver.cast_item_level = Some(item_level);
    let effect = driver
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Gaze scaling effect");
    let value = combat::spell_cast::effects::modded_base_points(world, unit, &driver, effect);
    let mut stat = store.get(STAT).expect("native Alnscorned Essence").clone();
    stat.effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("native Gaze stat effect")
        .base_points = value;
    combat::auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: STAT,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
        &stat,
    );
}
