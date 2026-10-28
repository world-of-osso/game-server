//! Twisted Horror's Tendril native stat proc and first-target bonus.
use std::collections::BTreeSet;

use super::combat;
use bevy::prelude::*;
use combat::auras::procs::AuraEvent;
use combat::auras::scripts::AuraScripts;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraInstance, AuraSource};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellStore;

const DRIVER: u32 = 1310404;
const NEW_TARGET: u32 = 1310446;
const BUFF: u32 = 1310414;

#[derive(Component, Default)]
struct StruckTargets(BTreeSet<Entity>);

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(DRIVER).or_default().on_proc = Some(proc_driver);
    let script = scripts.0.entry(NEW_TARGET).or_default();
    script.check_proc = Some(is_new_target);
    script.on_proc = Some(proc_new_target);
}

fn is_new_target(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.target.is_some_and(|target| {
        target != aura.caster
            && world
                .get::<StruckTargets>(aura.caster)
                .is_none_or(|seen| !seen.0.contains(&target))
    })
}

fn proc_driver(world: &mut World, unit: Entity, driver: &AuraInstance, _: Option<Entity>) {
    apply_buff(world, unit, driver, 1);
}

fn proc_new_target(world: &mut World, unit: Entity, _: &AuraInstance, target: Option<Entity>) {
    let target = target.expect("new-target proc requires an actual target");
    world
        .entity_mut(unit)
        .entry::<StruckTargets>()
        .or_default()
        .into_mut()
        .0
        .insert(target);
    let driver = world
        .get::<ActiveAuras>(unit)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == DRIVER && aura.caster == unit)
                .cloned()
        })
        .expect("equipped Tendril retains its primary driver");
    // SimC76f8118 unique_gear_midnight.cpp4818–4875: first successful proc
    // on each target grants three stacks; the native driver owns the ICD.
    apply_buff(world, unit, &driver, 3);
}

fn apply_buff(world: &mut World, unit: Entity, driver: &AuraInstance, stacks: u8) {
    let amount = driver
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Tendril driver stat budget")
        .amount;
    let mut buff = world
        .resource::<SpellStore>()
        .get(BUFF)
        .expect("native Tendril stat buff")
        .clone();
    buff.cast_item_level = driver.cast_item_level;
    buff.cast_aura_stacks = Some(stacks);
    buff.effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("native Tendril rating effect")
        .base_points = amount;
    let mask = auras::aura_effect_mask(&buff);
    auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: BUFF,
            effect_mask: mask,
            source: AuraSource::Cast,
        },
        &buff,
    );
}
