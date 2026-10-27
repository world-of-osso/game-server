//! Native Soul Immolation's source-defined Fury tick.
use super::combat;
use bevy::prelude::*;
use combat::auras::AuraInstance;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::class_progression::KnownSpells;
use combat::random::Rng;
use combat::soul_fragments::{self, FragmentKind};
use combat::spell_cast::{self, script_registry::SpellScriptHook};

const SOUL_IMMOLATION: u32 = 1241937;
const ENERGIZE: u32 = 1242475;

#[derive(Component)]
struct PendingUndying(f64);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::AfterNativeCastUpdate(retrigger_undying),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        SOUL_IMMOLATION,
        AuraScript {
            on_periodic: Some(tick),
            on_natural_end: Some(schedule_undying),
            ..Default::default()
        },
    );
}

fn schedule_undying(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if unit != aura.caster
        || !world
            .get::<KnownSpells>(unit)
            .is_some_and(|known| known.contains(1272405))
    {
        return;
    }
    // SimC76f8118 cpp1626–1654,5962–5977: natural final tick can retrigger the HoT.
    let chance = spell_cast::scripts::script_effect_value(world, unit, 1272405, 1) / 100.0;
    if combat::random::thread_rng().gen_range(0.0..1.0) < chance {
        let at = spell_cast::now(world) + 0.001;
        world.entity_mut(unit).insert(PendingUndying(at));
    }
}

fn retrigger_undying(world: &mut World) {
    let now = spell_cast::now(world);
    let due: Vec<_> = world
        .query::<(Entity, &PendingUndying)>()
        .iter(world)
        .filter(|(_, pending)| pending.0 <= now)
        .map(|(unit, _)| unit)
        .collect();
    for unit in due {
        world.entity_mut(unit).remove::<PendingUndying>();
        if !spell_cast::try_cast_triggered(world, unit, SOUL_IMMOLATION, Some(unit), 1) {
            error!(?unit, "Undying Embers retrigger was refused");
        }
    }
}

fn tick(world: &mut World, unit: Entity, aura: &AuraInstance, effect: u8) {
    if effect != 0 || unit != aura.caster {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp5930–5958: each self-HoT tick executes
    // the native Fury child. The native aura owns the periodic clock.
    spell_cast::cast_triggered(world, aura.caster, ENERGIZE, Some(unit), 1);
    // SimC76f8118 sc_demon_hunter.cpp5950–5958 spawns every other tick,
    // starting with the first scheduled tick.
    let tick = combat::auras::paid_periodic_ticks(world, aura.caster, unit, SOUL_IMMOLATION);
    if tick % 2 == 1 {
        soul_fragments::spawn(world, aura.caster, FragmentKind::Lesser, false)
            .expect("native Soul Immolation fragment");
    }
}
