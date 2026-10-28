//! Sudden Doom from actual Dread Plague ticks.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    effects::EffectCtx, script_registry::SpellScriptHook, scripts::AfterCastHook,
};
use combat::spell_info::SpellInfo;

#[derive(Component, Default)]
struct SuddenDoomAttempts(u32);

#[derive(Component, Default)]
struct PendingSuddenDoomConsumption(Vec<f64>);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::PeriodicDamageFinished(roll_sudden_doom),
    SpellScriptHook::AuraRollAfterCast(after_cast),
    SpellScriptHook::AfterCastUpdate(consume_due),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 47541 | 1242174 | 207317 | 383269).then_some(schedule_consumption)
}

fn schedule_consumption(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !combat::auras::has_aura(world, caster, 81340) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp13583–13587: consume one earned
    // stack after the spender's damage, allowing the impact to use it.
    let delay = if matches!(spell.id, 47541 | 1242174) {
        0.210
    } else {
        0.100
    };
    let due = combat::auras::aura_now(world) + delay;
    if world.get::<PendingSuddenDoomConsumption>(caster).is_none() {
        world
            .entity_mut(caster)
            .insert(PendingSuddenDoomConsumption::default());
    }
    world
        .get_mut::<PendingSuddenDoomConsumption>(caster)
        .unwrap()
        .0
        .push(due);
}

fn consume_due(world: &mut World) {
    let now = combat::auras::aura_now(world);
    let mut due = Vec::new();
    for (caster, mut pending) in world
        .query::<(Entity, &mut PendingSuddenDoomConsumption)>()
        .iter_mut(world)
    {
        pending.0.retain(|&deadline| {
            if deadline <= now {
                due.push(caster);
                false
            } else {
                true
            }
        });
    }
    for caster in due {
        let instance = world
            .get::<combat::auras::ActiveAuras>(caster)
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|aura| aura.spell_id == 81340 && aura.caster == caster)
                    .map(|aura| aura.instance_id)
            });
        if let Some(instance) = instance {
            combat::auras::spend_stack(world, caster, instance);
        }
    }
}

fn roll_sudden_doom(world: &mut World, ctx: &EffectCtx<'_>) {
    let caster = ctx.caster;
    if ctx.spell.id != 1240996 || !combat::auras::has_aura(world, caster, 49530) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp7956–7957/14827–14830: Dread
    // Plague ticks use accumulated PRD with Harbinger and Ebon Fever.
    let value = |world: &World, id, index| {
        combat::spell_cast::scripts::script_effect_value(world, caster, id, index) as f64 / 100.0
    };
    let mut chance = value(world, 49530, 1);
    for (id, index) in [(276023, 1), (207269, 0)] {
        if combat::auras::has_aura(world, caster, id) {
            chance *= 1.0 + value(world, id, index);
        }
    }
    let constant = combat::random::accumulated_proc_constant(chance);
    if world.get::<SuddenDoomAttempts>(caster).is_none() {
        world
            .entity_mut(caster)
            .insert(SuddenDoomAttempts::default());
    }
    let mut attempts = world.get_mut::<SuddenDoomAttempts>(caster).unwrap();
    attempts.0 += 1;
    if combat::random::random::<f64>() >= constant * f64::from(attempts.0) {
        return;
    }
    attempts.0 = 0;
    combat::auras::apply_aura(
        world,
        &combat::auras::AuraApplication {
            caster,
            target: caster,
            spell_id: 81340,
            effect_mask: 15,
            source: combat::auras::AuraSource::Cast,
        },
    );
}
