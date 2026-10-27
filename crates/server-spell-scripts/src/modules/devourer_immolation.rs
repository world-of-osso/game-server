//! Native Soul Immolation's source-defined Fury tick.
use super::combat;
use bevy::prelude::*;
use combat::auras::AuraInstance;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::soul_fragments::{self, FragmentKind};
use combat::spell_cast::{self, script_registry::SpellScriptHook};

const SOUL_IMMOLATION: u32 = 1241937;
const ENERGIZE: u32 = 1242475;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EarlyAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        SOUL_IMMOLATION,
        AuraScript {
            on_periodic: Some(tick),
            ..Default::default()
        },
    );
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
    let tick = combat::auras::periodic::paid_periodic_ticks(world, aura.caster, unit, SOUL_IMMOLATION);
    if tick % 2 == 1 {
        soul_fragments::spawn(world, aura.caster, FragmentKind::Lesser, false)
            .expect("native Soul Immolation fragment");
    }
}
