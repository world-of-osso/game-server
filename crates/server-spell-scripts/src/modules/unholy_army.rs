//! SimC76f8118 sc_death_knight.cpp8805–8836/8942–9010: Army summons its real lesser ghouls over time.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

const ARMY: u32 = 42650;
const LESSER_GHOUL: u32 = 275430;
const SUMMON_INTERVAL: f64 = 0.5;

#[derive(Component)]
struct PendingArmy {
    remaining: u32,
    next_summon: f64,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::AfterCastUpdate(update),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == ARMY).then_some(schedule_army)
}

fn schedule_army(world: &mut World, caster: Entity, _: &SpellInfo) {
    if world.get::<ActiveSpec>(caster).map(|spec| spec.0) != Some(252) {
        return;
    }
    let count = spell_cast::scripts::script_effect_value(world, caster, ARMY, 0) as u32;
    assert!(count > 0, "native Army ghoul count must be positive");
    let next_summon = combat::auras::aura_now(world) + SUMMON_INTERVAL;
    world.entity_mut(caster).insert(PendingArmy {
        remaining: count,
        next_summon,
    });
}

fn update(world: &mut World) {
    let now = combat::auras::aura_now(world);
    let mut due = Vec::new();
    for (caster, mut pending) in world.query::<(Entity, &mut PendingArmy)>().iter_mut(world) {
        while pending.remaining > 0 && now >= pending.next_summon {
            due.push(caster);
            pending.remaining -= 1;
            pending.next_summon += SUMMON_INTERVAL;
        }
    }
    for caster in due {
        if !spell_cast::try_cast_triggered(world, caster, LESSER_GHOUL, Some(caster), 1) {
            warn!(?caster, "Native Army lesser-ghoul summon refused");
        }
    }
}
