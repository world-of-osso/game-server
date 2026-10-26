//! Source-qualified periodic counters; native aura clocks, stacks and values remain authoritative.
use super::combat;
use combat::auras::{ActiveAuras, AuraApplication, AuraInstance, AuraSource};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EarlyAuraRegistration(register)];
use bevy::prelude::*;
use shared::components::ActiveSpec;

struct PeriodicCounter {
    spec: u32,
    talent: u32,
    passive: u32,
    effect: u8,
    counter: u32,
    enabler: u32,
    random_initial_phase: bool,
}

// SimC76f8118 sc_mage.cpp5559–5581/7186–7200: hasted passive Icicles,
// random initial phase, and the actual counter cap enables Glacial Spike.
const COUNTERS: &[PeriodicCounter] = &[
    PeriodicCounter {
        spec: 64,
        talent: 1246832,
        passive: 1246832,
        effect: 0,
        counter: 205473,
        enabler: 1222865,
        random_initial_phase: true,
    },
    // sc_mage.cpp6468–6471: the earned Rapid Refreezing buff grants one
    // Icicle on each actual native200ms tick; its first tick is not randomized.
    PeriodicCounter {
        spec: 64,
        talent: 1246832,
        passive: 1310248,
        effect: 0,
        counter: 205473,
        enabler: 1222865,
        random_initial_phase: false,
    },
];

fn register(scripts: &mut combat::auras::scripts::AuraScripts) {
    for row in COUNTERS {
        let script = scripts.0.entry(row.passive).or_default();
        if row.random_initial_phase {
            script.on_apply = Some(randomize_initial_phase);
        }
        script.on_periodic = Some(advance_counter);
    }
}

fn selected(world: &World, unit: Entity, row: &PeriodicCounter) -> bool {
    world
        .get::<ActiveSpec>(unit)
        .is_some_and(|spec| spec.0 == row.spec)
        && world.get::<TraitLoadout>(unit).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == row.talent)
        })
}

fn randomize_initial_phase(world: &mut World, unit: Entity, aura: &AuraInstance) {
    for row in COUNTERS.iter().filter(|row| row.passive == aura.spell_id) {
        if selected(world, unit, row) {
            combat::auras::randomize_initial_periodic_phase(
                world,
                unit,
                aura.instance_id,
                row.effect,
            );
        }
    }
}

fn apply_owned(world: &mut World, caster: Entity, spell_id: u32) {
    combat::auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn advance_counter(world: &mut World, unit: Entity, aura: &AuraInstance, effect: u8) {
    for row in COUNTERS
        .iter()
        .filter(|row| row.passive == aura.spell_id && row.effect == effect)
    {
        if aura.caster != unit || !selected(world, unit, row) {
            continue;
        }
        apply_owned(world, unit, row.counter);
        let full = world.get::<ActiveAuras>(unit).is_some_and(|auras| {
            auras.iter().any(|counter| {
                counter.caster == unit
                    && counter.spell_id == row.counter
                    && counter.stacks >= counter.max_stacks.max(1)
            })
        });
        if full {
            apply_owned(world, unit, row.enabler);
        }
    }
}
