//! Copy-local instance intros triggered by deaths with a source creature StringId.
use bevy::prelude::*;
use shared::components::{Npc, Position, UnitFlags};

use super::{instance_script, movement};
use crate::maps::InMap;
use crate::zones::CreatureStringIds;

struct IntroRule {
    map: u32,
    boss: u32,
    tag: &'static str,
    kills: u32,
    count_key: u32,
    state_key: u32,
    destination: Position,
    remove_aura: u32,
}

// TC 15e0ed76 instance_the_stonevault.cpp:54,100-121 and boss_edna.cpp:94,165-192.
const RULES: &[IntroRule] = &[IntroRule {
    map: 2652,
    boss: 210108,
    tag: "edna_intro_trash",
    kills: 5,
    count_key: 2_101_080,
    state_key: 2_101_081,
    destination: Position {
        x: 1.94097,
        y: 361.66537,
        z: -0.512153,
    },
    remove_aura: 447230,
}];
const INTRO_STARTED: u32 = 1;
const INTRO_DONE: u32 = 2;
const INTRO_POINT: u32 = 1;

// AVANOXX_INTRO_LOG_CORRELATED: the three named deaths precede the pull in
// XCmNaPcWdFA4kJ8D/9 and qLNnp4x613mv2Ctz/28. LittleWigs records a 17.3 s
// emote-to-nameplate interval; the death trigger and timer origin are inferred.
const AVANOXX_MAP: u32 = 2660;
const AVANOXX: u32 = 213179;
const AVANOXX_ATTENDANTS: [u32; 3] = [217531, 218324, 217533];
const AVANOXX_DEATHS_KEY: u32 = 2_131_790;
const AVANOXX_STATE_KEY: u32 = 2_131_791;
const AVANOXX_DELAY: f64 = 17.3;
const AVANOXX_BLOCKED: u32 = UnitFlags::IMMUNE_TO_PC | UnitFlags::NOT_SELECTABLE;

#[derive(Component)]
struct AvanoxxIntro(f64);

#[derive(Component, Clone, Copy)]
enum IntroMovement {
    Waiting { rule: usize, until: f64 },
    Walking { rule: usize },
    Finishing { rule: usize, until: f64 },
}

/// `InstanceScript::OnUnitDeath`: one count per actual death, within the copy.
pub(crate) fn on_unit_death(world: &mut World, creature: Entity) {
    let Some(map) = world.get::<InMap>(creature).copied() else {
        return;
    };
    record_avanoxx_death(world, creature, map);
    let Some(tags) = world.get::<CreatureStringIds>(creature) else {
        return;
    };
    let matches: Vec<usize> = RULES
        .iter()
        .enumerate()
        .filter_map(|(index, rule)| (rule.map == map.map_id && tags.has(rule.tag)).then_some(index))
        .collect();
    for index in matches {
        let rule = &RULES[index];
        if instance_script::get_data(world, map, rule.state_key) != 0 {
            continue;
        }
        let count = instance_script::get_data(world, map, rule.count_key) + 1;
        instance_script::set_data(world, map, rule.count_key, count);
        if count < rule.kills {
            continue;
        }
        instance_script::set_data(world, map, rule.state_key, INTRO_STARTED);
        let boss = world
            .query::<(Entity, &Npc, &InMap)>()
            .iter(world)
            .find_map(|(entity, npc, copy)| {
                (*copy == map && npc.template_id == rule.boss).then_some(entity)
            });
        let Some(boss) = boss else {
            error!(
                ?map,
                entry = rule.boss,
                "tagged-trash intro boss is missing"
            );
            continue;
        };
        let until = world.resource::<Time>().elapsed_secs_f64() + 3.0;
        world
            .entity_mut(boss)
            .insert(IntroMovement::Waiting { rule: index, until });
    }
}

fn record_avanoxx_death(world: &mut World, creature: Entity, map: InMap) {
    if map.map_id != AVANOXX_MAP || instance_script::get_data(world, map, AVANOXX_STATE_KEY) != 0 {
        return;
    }
    let Some(npc) = world.get::<Npc>(creature) else {
        return;
    };
    let Some(index) = AVANOXX_ATTENDANTS
        .iter()
        .position(|entry| *entry == npc.template_id)
    else {
        return;
    };
    let deaths = instance_script::get_data(world, map, AVANOXX_DEATHS_KEY) | (1 << index);
    instance_script::set_data(world, map, AVANOXX_DEATHS_KEY, deaths);
    if deaths != (1 << AVANOXX_ATTENDANTS.len()) - 1 {
        return;
    }
    let boss = world
        .query::<(Entity, &Npc, &InMap)>()
        .iter(world)
        .find_map(|(entity, npc, copy)| {
            (*copy == map && npc.template_id == AVANOXX).then_some(entity)
        });
    let Some(boss) = boss else {
        error!(?map, entry = AVANOXX, "Avanoxx intro boss is missing");
        return;
    };
    instance_script::set_data(world, map, AVANOXX_STATE_KEY, INTRO_STARTED);
    let until = world.resource::<Time>().elapsed_secs_f64() + AVANOXX_DELAY;
    world.entity_mut(boss).insert(AvanoxxIntro(until));
}

/// Advance the source 3 s pre-walk and 1 s post-arrival tasks, and Avanoxx's inferred delay.
pub(super) fn advance(world: &mut World) {
    let now = world.resource::<Time>().elapsed_secs_f64();
    let ready: Vec<_> = world
        .query::<(Entity, &InMap, &AvanoxxIntro)>()
        .iter(world)
        .filter_map(|(boss, map, intro)| (now >= intro.0).then_some((boss, *map)))
        .collect();
    for (boss, map) in ready {
        release_avanoxx(world, boss);
        instance_script::set_data(world, map, AVANOXX_STATE_KEY, INTRO_DONE);
        world.entity_mut(boss).remove::<AvanoxxIntro>();
    }
    let pending: Vec<_> = world
        .query::<(Entity, &InMap, &IntroMovement)>()
        .iter(world)
        .map(|(entity, map, phase)| (entity, *map, *phase))
        .collect();
    for (boss, map, phase) in pending {
        match phase {
            IntroMovement::Waiting { rule, until } if now >= until => {
                remove_source_aura(world, boss, RULES[rule].remove_aura);
                movement::set_walk(world, boss, true);
                movement::move_point(world, boss, INTRO_POINT, RULES[rule].destination);
                world
                    .entity_mut(boss)
                    .insert(IntroMovement::Walking { rule });
            }
            IntroMovement::Finishing { rule, until } if now >= until => {
                release_boss(world, boss, RULES[rule].remove_aura);
                instance_script::set_data(world, map, RULES[rule].state_key, INTRO_DONE);
                world.entity_mut(boss).remove::<IntroMovement>();
            }
            _ => {}
        }
    }
}

pub(super) fn on_point_arrival(world: &mut World, boss: Entity, id: u32) {
    if id != INTRO_POINT {
        return;
    }
    let Some(IntroMovement::Walking { rule }) = world.get::<IntroMovement>(boss).copied() else {
        return;
    };
    let until = world.resource::<Time>().elapsed_secs_f64() + 1.0;
    world
        .entity_mut(boss)
        .insert(IntroMovement::Finishing { rule, until });
}

/// A respawned boss in a completed intro remains attackable in that copy.
pub(super) fn on_appeared(world: &mut World, boss: Entity) {
    let (Some(map), Some(npc)) = (world.get::<InMap>(boss).copied(), world.get::<Npc>(boss)) else {
        return;
    };
    let entry = npc.template_id;
    if map.map_id == AVANOXX_MAP
        && entry == AVANOXX
        && instance_script::get_data(world, map, AVANOXX_STATE_KEY) == INTRO_DONE
    {
        release_avanoxx(world, boss);
    }
    for rule in RULES
        .iter()
        .filter(|rule| rule.map == map.map_id && rule.boss == entry)
    {
        if instance_script::get_data(world, map, rule.state_key) == INTRO_DONE {
            release_boss(world, boss, rule.remove_aura);
        }
    }
}

fn release_avanoxx(world: &mut World, boss: Entity) {
    if let Some(mut flags) = world.get_mut::<UnitFlags>(boss) {
        flags.0 &= !AVANOXX_BLOCKED;
    }
}

fn release_boss(world: &mut World, boss: Entity, aura: u32) {
    remove_source_aura(world, boss, aura);
    if let Some(mut flags) = world.get_mut::<UnitFlags>(boss) {
        flags.0 &= !UnitFlags::IMMUNE_TO_PC;
    }
}

fn remove_source_aura(world: &mut World, boss: Entity, spell: u32) {
    let instances: Vec<_> = world
        .get::<crate::auras::ActiveAuras>(boss)
        .map(|auras| {
            auras
                .iter()
                .filter(|aura| aura.spell_id == spell)
                .map(|aura| aura.instance_id)
                .collect()
        })
        .unwrap_or_default();
    for instance in instances {
        crate::auras::remove_instance(world, boss, instance, crate::auras::RemoveReason::Cancelled);
    }
}
