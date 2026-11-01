//! Battlelord follows Overpower and is consumed by foreground Mortal Strike.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, RemoveReason, procs};
use combat::spell_cast::{SpellHistory, history_now, scripts::reduce_cooldown};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::ActiveSpec;

const BATTLELORD: u32 = 386630;
const BATTLELORD_BUFF: u32 = 386631;
const OVERPOWER: u32 = 7384;
const MORTAL_STRIKE: u32 = 12294;

pub(super) fn owns_proc(spell: u32) -> bool {
    spell == BATTLELORD
}

pub(super) fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo, foreground: bool) {
    if world
        .get::<ActiveSpec>(caster)
        .is_none_or(|spec| spec.0 != 71)
    {
        return;
    }
    match spell.id {
        OVERPOWER => roll_battlelord(world, caster),
        MORTAL_STRIKE if foreground => {
            auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
                aura.spell_id == BATTLELORD_BUFF && aura.caster == caster
            });
        }
        _ => {}
    }
}

fn roll_battlelord(world: &mut World, caster: Entity) {
    let Some(instance) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == BATTLELORD && aura.caster == caster)
            .map(|aura| aura.instance_id)
    }) else {
        return;
    };
    // SimC76f8118 sc_warrior.cpp5478–5487: each Overpower execution rolls
    // the native Battlelord chance, resets Mortal Strike, and grants its buff.
    if !procs::try_native_policy_proc(world, caster, instance) {
        return;
    }
    let now = history_now(world);
    let spell = world
        .resource::<SpellStore>()
        .get(MORTAL_STRIKE)
        .expect("native Mortal Strike");
    let remaining = world
        .get::<SpellHistory>(caster)
        .map_or(0, |history| history.remaining_cooldown_ms(spell, now));
    reduce_cooldown(
        world,
        caster,
        MORTAL_STRIKE,
        (remaining as f64 + 1.0) / 1000.0,
    );
    if !combat::spell_cast::try_cast_triggered(world, caster, BATTLELORD_BUFF, Some(caster), 1) {
        warn!(?caster, "Native Battlelord buff refused");
    }
}
