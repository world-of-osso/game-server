//! Native Familiar aura drives the source-defined hasted Arcane Assault cadence.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::networking::CombatTarget;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::stats::CharacterStats;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterQueuedTriggers(tick_familiars)];

const FAMILIAR: u32 = 210126;
const ASSAULT: u32 = 225119;
// SimC76f8118 sc_mage.cpp6363–6367: the Familiar's three-second tick is hasted.
const PERIOD_SECONDS: f64 = 3.0;

#[derive(Component)]
struct FamiliarTick {
    aura_instance: u32,
    next: f64,
}

fn period(world: &World, caster: Entity) -> f64 {
    let haste = world
        .get::<CharacterStats>(caster)
        .expect("Familiar caster stats")
        .haste_pct;
    PERIOD_SECONDS / (1.0 + f64::from(haste) / 100.0)
}

fn tick_familiars(world: &mut World) {
    let now = spell_cast::now(world);
    let actors = world.query_filtered::<(Entity, Option<&ActiveAuras>, Option<&FamiliarTick>), Or<(With<ActiveAuras>, With<FamiliarTick>)>>().iter(world)
        .filter_map(|(caster, auras, timer)| {
            let aura = auras.and_then(|auras| auras.iter().find(|aura| aura.spell_id == FAMILIAR && aura.caster == caster));
            (aura.is_some() || timer.is_some()).then_some((caster, aura.map(|aura| aura.instance_id)))
        }).collect::<Vec<_>>();
    for (caster, instance) in actors {
        let Some(instance) = instance else {
            world.entity_mut(caster).remove::<FamiliarTick>();
            continue;
        };
        let next = world
            .get::<FamiliarTick>(caster)
            .filter(|timer| timer.aura_instance == instance)
            .map(|timer| timer.next);
        let interval = period(world, caster);
        let Some(next) = next else {
            world.entity_mut(caster).insert(FamiliarTick {
                aura_instance: instance,
                next: now + interval,
            });
            continue;
        };
        if now < next {
            continue;
        }
        world
            .get_mut::<FamiliarTick>(caster)
            .expect("existing Familiar timer")
            .next = next + interval;
        if let Some(target) = world
            .get::<CombatTarget>(caster)
            .and_then(|target| target.0)
        {
            spell_cast::cast_triggered(world, caster, ASSAULT, Some(target), 1);
        }
    }
}
