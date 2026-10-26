//! Native Familiar aura drives the source-defined hasted Arcane Assault cadence.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::networking::CombatTarget;
use combat::spell_cast::{
    self, effects::EffectCtx, script_registry::SpellScriptHook, scripts::script_effect_value,
};
use combat::spell_info::{SpellInfo, SpellStore};
use combat::stats::CharacterStats;
use shared::components::PowerType;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterQueuedTriggers(tick_familiars),
    SpellScriptHook::AfterElementalLaunch(energize_from_assault),
];

const FAMILIAR: u32 = 210126;
const ASSAULT: u32 = 225119;
const ENERGIZED: u32 = 452997;
const ENERGIZE: u32 = 454020;
const SURGE: u32 = 365362;
// SimC76f8118 sc_mage.cpp6363–6374: hasted3s ticks, extra assaults75ms apart.
const PERIOD_SECONDS: f64 = 3.0;
const BURST_SPACING_SECONDS: f64 = 0.075;

#[derive(Component)]
struct FamiliarTick {
    aura_instance: u32,
    next: f64,
}

#[derive(Component, Default)]
struct PendingAssaults(Vec<f64>);

fn period(world: &World, caster: Entity) -> f64 {
    let haste = world
        .get::<CharacterStats>(caster)
        .expect("Familiar caster stats")
        .haste_pct;
    PERIOD_SECONDS / (1.0 + f64::from(haste) / 100.0)
}

fn tick_familiars(world: &mut World) {
    let now = spell_cast::now(world);
    resolve_pending_assaults(world, now);
    let actors = world.query_filtered::<(Entity, Option<&ActiveAuras>, Option<&FamiliarTick>),
        Or<(With<ActiveAuras>, With<FamiliarTick>)>>().iter(world)
        .filter_map(|(caster, auras, timer)| {
            let aura = auras.and_then(|auras| auras.iter().find(|aura| aura.spell_id == FAMILIAR && aura.caster == caster));
            (aura.is_some() || timer.is_some()).then_some((caster, aura.map(|aura| aura.instance_id)))
        }).collect::<Vec<_>>();
    for (caster, instance) in actors {
        update_familiar(world, caster, instance, now);
    }
}

fn update_familiar(world: &mut World, caster: Entity, instance: Option<u32>, now: f64) {
    let Some(instance) = instance else {
        world.entity_mut(caster).remove::<FamiliarTick>();
        return;
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
        return;
    };
    if now < next {
        return;
    }
    world
        .get_mut::<FamiliarTick>(caster)
        .expect("existing Familiar timer")
        .next = next + interval;
    cast_current_target(world, caster);
    schedule_energized_assaults(world, caster, now);
}

fn cast_current_target(world: &mut World, caster: Entity) {
    if let Some(target) = world
        .get::<CombatTarget>(caster)
        .and_then(|target| target.0)
    {
        spell_cast::cast_triggered(world, caster, ASSAULT, Some(target), 1);
    }
}

fn schedule_energized_assaults(world: &mut World, caster: Entity, now: f64) {
    if !has_talent(world, caster, ENERGIZED) || !auras::has_aura(world, caster, SURGE) {
        return;
    }
    let total = script_effect_value(world, caster, ENERGIZED, 0) as u32;
    if total <= 1 {
        return;
    }
    if world.get::<PendingAssaults>(caster).is_none() {
        world.entity_mut(caster).insert(PendingAssaults::default());
    }
    let mut pending = world
        .get_mut::<PendingAssaults>(caster)
        .expect("pending Familiar assaults");
    for index in 1..total {
        pending
            .0
            .push(now + f64::from(index) * BURST_SPACING_SECONDS);
    }
}

fn resolve_pending_assaults(world: &mut World, now: f64) {
    let actors: Vec<_> = world
        .query_filtered::<Entity, With<PendingAssaults>>()
        .iter(world)
        .collect();
    for caster in actors {
        let mut pending = world
            .get_mut::<PendingAssaults>(caster)
            .expect("pending Familiar assaults");
        let before = pending.0.len();
        pending.0.retain(|due| *due > now);
        let count = before - pending.0.len();
        let empty = pending.0.is_empty();
        if empty {
            world.entity_mut(caster).remove::<PendingAssaults>();
        }
        for _ in 0..count {
            cast_current_target(world, caster);
        }
    }
}

fn energize_from_assault(world: &mut World, caster: Entity, spell: &SpellInfo, _: bool) {
    if spell.id != ASSAULT || !has_talent(world, caster, ENERGIZED) {
        return;
    }
    // sc_mage.cpp3052–3065: one native effect2 chance per Assault execution.
    let chance = f64::from(script_effect_value(world, caster, ENERGIZED, 1)) / 100.0;
    if chance <= 0.0 || combat::random::random::<f64>() >= chance {
        return;
    }
    let (_, max) =
        spell_cast::powers::power_value(world, caster, PowerType::Mana).expect("Familiar Mana");
    let amount = (max * script_effect_value(world, caster, ENERGIZE, 0) / 100.0).trunc();
    let source = world
        .resource::<SpellStore>()
        .get(ENERGIZE)
        .expect("native Familiar energize")
        .clone();
    let ctx = EffectCtx {
        caster,
        spell: &source,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    spell_cast::effects::energize_unit(world, &ctx, caster, PowerType::Mana, amount, false);
}
