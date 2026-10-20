//! Native70448 Abundance: threshold from207383, one207640 buff; not old TC stacks.
use bevy::prelude::*;

use super::scripts::{AuraScript, AuraScripts};
use super::{ActiveAuras, AuraInstance, RemoveReason, has_aura, remove_auras};

const REJUVENATION: u32 = 774;
const TALENT: u32 = 207383;
const BUFF: u32 = 207640;

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        REJUVENATION,
        AuraScript {
            on_apply: Some(rejuvenation_changed),
            on_remove: Some(rejuvenation_removed),
            ..Default::default()
        },
    );
}

fn rejuvenation_changed(world: &mut World, _target: Entity, aura: &AuraInstance) {
    reconcile_threshold(world, aura.caster);
}

fn rejuvenation_removed(
    world: &mut World,
    _target: Entity,
    aura: &AuraInstance,
    _reason: RemoveReason,
) {
    reconcile_threshold(world, aura.caster);
}

fn read_native_threshold(world: &World, caster: Entity) -> Option<usize> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == TALENT)?
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .map(|effect| effect.amount as usize)
}

fn reconcile_threshold(world: &mut World, caster: Entity) {
    let Some(threshold) = read_native_threshold(world, caster) else {
        return;
    };
    let count = world
        .query::<&ActiveAuras>()
        .iter(world)
        .filter(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == REJUVENATION && aura.caster == caster)
        })
        .take(threshold)
        .count();
    if count >= threshold {
        if !has_aura(world, caster, BUFF) {
            crate::spell_cast::cast_triggered(world, caster, BUFF, Some(caster), 1);
        }
    } else {
        remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == BUFF && aura.caster == caster
        });
    }
}
