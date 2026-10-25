//! SIMC_WW_UNBROKEN_RHYTHM_SCK_SNAPSHOT_CONSUMPTION:
//! SimC76f8118 sc_monk.cpp944/1526–1532/1660/1723–1727; native1297033 values.
use bevy::prelude::*;

use crate::auras::{self, ActiveAuras, AuraInstance, RemoveReason};

pub(super) const ROOT: u32 = 101546;
const BUFF: u32 = 1297033;
const PRODUCER: u32 = 1296624;
const FOF_DAMAGE: u32 = 117418;
const TICK_DEPTH: u8 = 1;

/// Actual parent aura identity and its earned damage scalar, not live buff stacks.
#[derive(Component)]
struct SckSnapshot {
    aura_instance: u32,
    multiplier: f32,
}

pub(crate) fn register_unbroken_rhythm(scripts: &mut auras::scripts::AuraScripts) {
    // SimC1723–1727 replaces the native unmasked aura42 producer with execute events.
    scripts.0.insert(
        PRODUCER,
        auras::scripts::AuraScript {
            check_proc: Some(|_, _, _| false),
            ..Default::default()
        },
    );
    scripts.0.insert(
        ROOT,
        auras::scripts::AuraScript {
            on_remove: Some(forget_snapshot),
            ..Default::default()
        },
    );
}

pub(super) fn after_launch(world: &mut World, caster: Entity, spell: u32) {
    match spell {
        FOF_DAMAGE => {
            // SimC1723–1727: once per regular/tier execute, not per impact or target.
            let equipped = world.get::<ActiveAuras>(caster).is_some_and(|auras| {
                auras
                    .iter()
                    .any(|aura| aura.spell_id == PRODUCER && aura.caster == caster)
            });
            if equipped {
                crate::spell_cast::cast_triggered(world, caster, BUFF, Some(caster), 1);
            }
        }
        // SimC944/parse_effects.cpp1593–1606 consumes after RSK-family damage.
        185099 | 468179 => consume_owned_rhythm(world, caster),
        _ => {}
    }
}

fn consume_owned_rhythm(world: &mut World, caster: Entity) {
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == BUFF && aura.caster == caster
    });
}

fn earned_multiplier(world: &World, caster: Entity) -> f32 {
    world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == BUFF && aura.caster == caster)
        })
        .map_or(1.0, |aura| {
            let percent = aura
                .effects
                .iter()
                .find(|effect| effect.index == 1)
                .expect("native Unbroken Rhythm retains effect1 damage percent")
                .amount;
            1.0 + f32::from(aura.stacks) * percent / 100.0
        })
}

/// The parent's single self hit follows aura application, before its first tick.
/// This seam also covers native triggered casts and refreshed parent auras.
pub(super) fn capture_and_consume(world: &mut World, caster: Entity, unit: Entity, _depth: u8) {
    let multiplier = earned_multiplier(world, caster);
    let parent = world.get::<ActiveAuras>(unit).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == ROOT && aura.caster == caster)
            .map(|aura| aura.instance_id)
    });
    if let Some(aura_instance) = parent {
        world.entity_mut(unit).insert(SckSnapshot {
            aura_instance,
            multiplier,
        });
    } else {
        world.entity_mut(unit).remove::<SckSnapshot>();
        error!(
            ?caster,
            ?unit,
            "SCK parent hit lacks its native aura snapshot context"
        );
    }
    consume_owned_rhythm(world, caster);
}

/// Only this parent's real periodic emission carries its frozen effectiveness.
/// Other107270 emitters (including322729) retain their native default dispatch.
pub(super) fn cast_tick(world: &mut World, unit: Entity, aura: &AuraInstance, index: u8) {
    let Some(snapshot) = world
        .get::<SckSnapshot>(unit)
        .filter(|snapshot| snapshot.aura_instance == aura.instance_id)
    else {
        error!(
            ?unit,
            instance = aura.instance_id,
            "SCK tick lacks its matching parent snapshot"
        );
        return;
    };
    let multiplier = snapshot.multiplier;
    let child_id = aura
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .expect("native SCK periodic effect remains on its parent aura")
        .trigger_spell;
    let Some(mut child) = super::super::load_triggered_spell(world, child_id, TICK_DEPTH) else {
        return;
    };
    child.cast_damage_multiplier = Some(multiplier);
    super::super::launch_triggered_spell(world, aura.caster, &child, Some(unit), TICK_DEPTH);
}

fn forget_snapshot(world: &mut World, unit: Entity, aura: &AuraInstance, _reason: RemoveReason) {
    let owned = world
        .get::<SckSnapshot>(unit)
        .is_some_and(|snapshot| snapshot.aura_instance == aura.instance_id);
    if owned {
        world.entity_mut(unit).remove::<SckSnapshot>();
    }
}
