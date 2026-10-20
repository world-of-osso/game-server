//! TC a352b1fa boss_vazruden_the_herald.cpp535–555/575–610.
//! Ground hit-unit children and the registered source Liquid Fire GO AI only.
use bevy::prelude::*;
use shared::components::{Position, Rotation};

use crate::spell_info::SpellStore;
use crate::world_data::WorldData;

const TRIGGER: u32 = 21252;
const REPEAT_MS: u32 = 2000;
const TRIGGER_LIFETIME_MS: u32 = 1000;

pub(crate) fn source_spell(
    entry: u32,
    go_type: u8,
    ai_name: &str,
    script_name: &str,
    spell: i64,
) -> Option<u32> {
    let source = matches!(
        (entry, spell),
        (180125, 23972) | (181890, 30927) | (182533, 32492)
    );
    (source && go_type == 6 && ai_name.is_empty() && script_name == "go_vazruden_liquid_fire")
        .then_some(spell as u32)
}

pub(crate) fn unsupported_reason(world: &World, spell: u32) -> Option<String> {
    if world
        .get_resource::<WorldData>()
        .is_none_or(|data| !data.templates.contains_key(&TRIGGER))
    {
        return Some(format!("Liquid Fire missing source creature {TRIGGER}"));
    }
    world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(spell))
        .is_none()
        .then(|| format!("Liquid Fire missing source trap spell {spell}"))
}

pub(crate) fn fireball_hit(world: &mut World, _caster: Entity, target: Entity, depth: u8) {
    let map = *world
        .get::<crate::maps::InMap>(target)
        .expect("actual hit unit map");
    let spell = if crate::zones::map_difficulty_id(world, map) == 2 {
        30928
    } else {
        31706
    };
    crate::spell_cast::cast_triggered(world, target, spell, Some(target), depth + 1);
}

pub(crate) fn cone_hit(world: &mut World, _caster: Entity, target: Entity, depth: u8) {
    crate::spell_cast::cast_triggered(world, target, 23971, Some(target), depth + 1);
}

#[derive(Component)]
struct LiquidFire {
    spell: u32,
    elapsed_ms: u32,
    next_pulse_ms: u32,
}

pub(crate) fn arm(world: &mut World, object: Entity, spell: u32) {
    world.entity_mut(object).insert(LiquidFire {
        spell,
        elapsed_ms: 0,
        next_pulse_ms: 0,
    });
}

/// Gameobjects update after units. Reset's zero task consumes the first diff;
/// TaskContext::Repeat advances the previous scheduled deadline, not dispatch time.
pub(crate) fn update(world: &mut World) {
    let diff = world.resource::<Time<Fixed>>().delta().as_millis() as u32;
    let objects: Vec<_> = world
        .query_filtered::<Entity, With<LiquidFire>>()
        .iter(world)
        .collect();
    for object in objects {
        let spell = {
            let mut fire = world.get_mut::<LiquidFire>(object).expect("live source GO");
            fire.elapsed_ms += diff;
            if fire.elapsed_ms < fire.next_pulse_ms {
                continue;
            }
            fire.next_pulse_ms += REPEAT_MS;
            fire.spell
        };
        spawn_pulse(world, object, spell);
    }
}

fn spawn_pulse(world: &mut World, object: Entity, spell: u32) {
    let position = *world.get::<Position>(object).expect("source GO position");
    let orientation =
        world.get::<Rotation>(object).expect("source GO rotation").y - std::f32::consts::FRAC_PI_2;
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    let Some(trigger) =
        crate::zones::summon_creature(world, object, TRIGGER, [x, y, z, orientation])
    else {
        error!(?object, "Liquid Fire source21252 summon failed");
        return;
    };
    super::super::smart_ai::lifetimes::schedule_despawn(
        world,
        trigger,
        TRIGGER_LIFETIME_MS,
        0,
        false,
    );
    if let Err(reason) =
        crate::spell_cast::creature_cast(world, trigger, spell, Some(trigger), false)
    {
        error!(
            ?object,
            ?trigger,
            spell,
            ?reason,
            "Liquid Fire ordinary source self-cast rejected"
        );
    }
}
