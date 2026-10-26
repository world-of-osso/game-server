//! Native Killing Spree's periodic dummy dispatches its two independent hand attacks.
use super::combat;
use bevy::prelude::*;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::{AuraInstance, RemoveReason};
use combat::random::Rng;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::{self, targets};
use combat::spell_info::SpellStore;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EarlyAuraRegistration(register)];

const KILLING_SPREE: u32 = 51690;
const HANDS: &[u32] = &[57841, 57842];
const ENERGIZE: u32 = 1235074;

#[derive(Component, Default)]
struct SpreeTicks(u32);

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        KILLING_SPREE,
        AuraScript {
            on_apply: Some(begin),
            on_periodic: Some(tick),
            on_remove: Some(end),
            ..Default::default()
        },
    );
}

fn begin(world: &mut World, unit: Entity, _: &AuraInstance) {
    world.entity_mut(unit).insert(SpreeTicks::default());
}

fn end(world: &mut World, unit: Entity, _: &AuraInstance, _: RemoveReason) {
    world.entity_mut(unit).remove::<SpreeTicks>();
}

fn tick(world: &mut World, unit: Entity, aura: &AuraInstance, effect: u8) {
    if effect != 0 {
        return;
    }
    let initial = {
        let mut ticks = world.get_mut::<SpreeTicks>(unit).expect("active native Spree ticks");
        let initial = ticks.0 == 0;
        ticks.0 += 1;
        initial
    };
    // SimC76f8118 sc_rogue.cpp4822–4835: each hand independently picks
    // an enemy; tick0 deals damage but only subsequent ticks return native CP.
    for hand in HANDS {
        if let Some(target) = choose_target(world, aura.caster) {
            spell_cast::cast_triggered(world, aura.caster, *hand, Some(target), 1);
        }
    }
    if !initial {
        spell_cast::cast_triggered(world, aura.caster, ENERGIZE, Some(aura.caster), 1);
    }
}

fn choose_target(world: &mut World, caster: Entity) -> Option<Entity> {
    let center = targets::position(world, caster)?;
    let spell = world.resource::<SpellStore>().get(KILLING_SPREE).expect("native Killing Spree");
    let (_, radius) = targets::spell_range(spell, true);
    let enemies = targets::area_units(world, caster, center, radius, targets::Relation::Enemy, 0);
    if enemies.is_empty() {
        return None;
    }
    Some(enemies[combat::random::thread_rng().gen_range(0..enemies.len())])
}
