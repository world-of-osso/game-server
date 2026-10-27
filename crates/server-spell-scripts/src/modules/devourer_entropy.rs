//! SimC76f8118 sc_demon_hunter.cpp9873–9881,9959–9967: Entropy combat cadence.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::KnownSpells;
use combat::soul_fragments::{self, FragmentKind};
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;
use shared::components::ActiveSpec;

const ENTROPY: u32 = 1261684;
#[derive(Component)]
struct EntropyClock {
    next: f64,
    period: f64,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::CombatStatusChanged(combat_changed),
    SpellScriptHook::AfterQueuedTriggers(spawn_due),
];

fn combat_changed(world: &mut World, unit: Entity, active: bool) {
    if !active {
        world.entity_mut(unit).remove::<EntropyClock>();
        return;
    }
    if world.get::<ActiveSpec>(unit).map(|spec| spec.0) != Some(1480)
        || !world
            .get::<KnownSpells>(unit)
            .is_some_and(|known| known.contains(ENTROPY))
    {
        return;
    }
    let period = f64::from(
        world
            .resource::<SpellStore>()
            .get(ENTROPY)
            .expect("native Entropy spell")
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("native Entropy periodic effect")
            .aura_period_ms,
    ) / 1000.0;
    assert!(period > 0.0, "native Entropy period");
    soul_fragments::spawn(world, unit, FragmentKind::Lesser, false)
        .expect("native Entropy fragment");
    let next = spell_cast::now(world) + period;
    world.entity_mut(unit).insert(EntropyClock { next, period });
}

fn spawn_due(world: &mut World) {
    let now = spell_cast::now(world);
    let due: Vec<_> = world
        .query::<(Entity, &EntropyClock)>()
        .iter(world)
        .filter(|(_, clock)| clock.next <= now)
        .map(|(unit, _)| unit)
        .collect();
    for unit in due {
        let mut clock = world
            .get_mut::<EntropyClock>(unit)
            .expect("due Entropy clock");
        clock.next += clock.period;
        soul_fragments::spawn(world, unit, FragmentKind::Lesser, false)
            .expect("native Entropy fragment");
    }
}
