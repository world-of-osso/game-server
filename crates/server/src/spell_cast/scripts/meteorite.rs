//! Approved FIRE_PLAYER_METEORITE_NATIVE_DATA_BRIDGE and
//! FIRE_PLAYER_SPHERES_NATIVE_DATA_BRIDGE; native70448 supplies values/timers.
//! TC has no corresponding bindings. Combat Sphere acquisition is separate.
use bevy::prelude::*;
use shared::components::{ActiveSpec, CombatStatus, Position};

use crate::auras::procs::AuraEvent;
use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::auras::{ActiveAuras, AuraInstance, RemoveReason, has_aura};
use crate::spell_cast::effects::EffectCtx;
use crate::spell_info::{SpellEffect, SpellStore};

pub(super) const ROOT: u32 = 449559;
const MISSILE: u32 = 449560;
const PYROCOSM: u32 = 1260673;
const FIREBALL: u32 = 133;
const FIRE_BLAST: u32 = 108853;
const SPHERE_TALENT: u32 = 448601;
const SPHERE_DRIVER: u32 = 449849;
const SPHERE_BUFF: u32 = 448604;
const GLORIOUS: u32 = 449394;
const GLORIOUS_MARKER: u32 = 451073;
const FIRE_SPEC: u32 = 63;

pub(crate) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        PYROCOSM,
        AuraScript {
            check_proc: Some(check_fireball),
            on_damage_proc: Some(roll_pyrocosm),
            ..Default::default()
        },
    );
    scripts.0.insert(
        SPHERE_TALENT,
        AuraScript {
            on_apply: Some(start_ooc_spheres),
            on_remove: Some(stop_ooc_spheres),
            ..Default::default()
        },
    );
    scripts.0.insert(
        SPHERE_DRIVER,
        AuraScript {
            on_periodic: Some(generate_ooc_sphere),
            ..Default::default()
        },
    );
    scripts.0.insert(
        GLORIOUS_MARKER,
        AuraScript {
            check_proc: Some(check_fire_blast),
            on_damage_proc: Some(cast_glorious_storm),
            ..Default::default()
        },
    );
}

fn is_fire(world: &World, unit: Entity) -> bool {
    world
        .get::<ActiveSpec>(unit)
        .is_some_and(|spec| spec.0 == FIRE_SPEC)
}

fn check_fireball(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    #[cfg(test)]
    if event.spell_id == Some(FIREBALL) {
        eprintln!(
            "METEORITE_PROC_TRACE actor_mask={:?} damage={} actor={:?} aura_caster={:?}",
            event.actor_mask, event.damage, event.actor, aura.caster
        );
    }
    is_fire(world, event.actor)
        && event.actor == aura.caster
        && event.spell_id == Some(FIREBALL)
        && !event.periodic
        && event.damage > 0.0
}

fn roll_pyrocosm(world: &mut World, unit: Entity, aura: &AuraInstance, event: &AuraEvent) {
    let chance = aura
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("Pyrocosm native Fire chance effect1")
        .amount;
    if crate::random::random::<f32>() * 100.0 < chance {
        if let Some(target) = event.target {
            cast_root_at_target(world, unit, target);
        }
    }
}

fn cast_root_at_target(world: &mut World, caster: Entity, target: Entity) {
    let Some(position) = world.get::<Position>(target) else {
        return;
    };
    let destination = Vec3::new(position.x, position.y, position.z);
    crate::spell_cast::cast_triggered_at_destination(world, caster, ROOT, destination, 1);
}

pub(super) fn cast_missile(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect) {
    let Some(destination) = ctx.dest else {
        warn!("Meteorite449559 has no authored destination");
        return;
    };
    crate::spell_cast::cast_triggered_at_destination(
        world,
        ctx.caster,
        MISSILE,
        destination.position,
        ctx.depth + 1,
    );
}

fn start_ooc_spheres(world: &mut World, unit: Entity, _aura: &AuraInstance) {
    if is_fire(world, unit) {
        crate::spell_cast::cast_triggered(world, unit, SPHERE_DRIVER, Some(unit), 1);
    }
}

fn stop_ooc_spheres(world: &mut World, unit: Entity, _aura: &AuraInstance, _reason: RemoveReason) {
    crate::auras::remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
        aura.spell_id == SPHERE_DRIVER && aura.caster == unit
    });
}

fn generate_ooc_sphere(world: &mut World, unit: Entity, _aura: &AuraInstance, index: u8) {
    let in_combat = world
        .get::<CombatStatus>(unit)
        .is_some_and(|status| status.0);
    if index != 0 || !is_fire(world, unit) || in_combat || !has_aura(world, unit, SPHERE_TALENT) {
        return;
    }
    let cap = world
        .resource::<SpellStore>()
        .get(SPHERE_BUFF)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("native Spellfire Sphere stack cap")
        .cumulative_aura;
    let stacks = world
        .get::<ActiveAuras>(unit)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == SPHERE_BUFF && aura.caster == unit)
        })
        .map_or(0, |aura| u32::from(aura.stacks));
    if stacks >= cap {
        return;
    }
    crate::spell_cast::cast_triggered(world, unit, SPHERE_BUFF, Some(unit), 1);
    if has_aura(world, unit, GLORIOUS) {
        crate::spell_cast::cast_triggered(world, unit, GLORIOUS_MARKER, Some(unit), 1);
    }
}

fn check_fire_blast(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    is_fire(world, event.actor)
        && event.actor == aura.caster
        && event.spell_id == Some(FIRE_BLAST)
        && !event.periodic
}

fn cast_glorious_storm(world: &mut World, unit: Entity, aura: &AuraInstance, event: &AuraEvent) {
    let Some(target) = event.target else {
        return;
    };
    let count = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Glorious Meteorite count effect0")
        .amount as u32;
    for _ in 0..count {
        cast_root_at_target(world, unit, target);
    }
}
