//! Inner Demons starts its unhasted native summon cadence at combat entry.
use super::combat;
use bevy::prelude::*;
use combat::auras::has_aura;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellStore;

const TALENT: u32 = 267216;
const SUMMON: u32 = 279910;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::CombatStatusChanged(combat_changed),
    SpellScriptHook::AfterQueuedTriggers(summon_due),
];

#[derive(Component)]
struct InnerDemonsClock {
    next: f64,
    period: f64,
}

fn combat_changed(world: &mut World, unit: Entity, active: bool) {
    if !active {
        world.entity_mut(unit).remove::<InnerDemonsClock>();
        return;
    }
    if !has_aura(world, unit, TALENT) {
        return;
    }
    let period = f64::from(
        world.resource::<SpellStore>().get(TALENT).expect("Inner Demons native spell")
            .effects.iter().find(|effect| effect.index == 0)
            .expect("Inner Demons native period").aura_period_ms,
    ) / 1000.0;
    assert!(period > 0.0, "Inner Demons requires a native period");
    // SimC76f8118 init.cpp766–771/1722–1725: uniform first phase;
    // subsequent summons use the native period without haste.
    let next = combat::spell_cast::now(world) + combat::random::random::<f64>() * period;
    world.entity_mut(unit).insert(InnerDemonsClock { next, period });
}

fn summon_due(world: &mut World) {
    let now = combat::spell_cast::now(world);
    let due: Vec<_> = world.query::<(Entity, &InnerDemonsClock)>().iter(world)
        .filter(|(_, clock)| clock.next <= now).map(|(unit, _)| unit).collect();
    for unit in due {
        if !has_aura(world, unit, TALENT) {
            world.entity_mut(unit).remove::<InnerDemonsClock>();
            continue;
        }
        let mut clock = world.get_mut::<InnerDemonsClock>(unit).expect("due Inner Demons clock");
        clock.next += clock.period;
        if !combat::spell_cast::try_cast_triggered(world, unit, SUMMON, Some(unit), 1) {
            warn!(?unit, "Inner Demons native summon refused");
        }
    }
}
