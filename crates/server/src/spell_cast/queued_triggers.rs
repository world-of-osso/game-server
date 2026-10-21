//! Queued native casts: effect64 costed triggers and effect160 normal forced casts.
//! TC a352b1fa SpellEffects.cpp691–723,851–892.
use std::collections::{BTreeMap, VecDeque};

use bevy::prelude::*;

const MILLIS_PER_SECOND: f64 = 1000.0;

#[derive(Resource, Default)]
pub(crate) struct PendingTriggers(BTreeMap<u64, VecDeque<QueuedTrigger>>);

enum QueuedCastKind {
    CostedTrigger,
    NormalForce,
}

struct QueuedTrigger {
    kind: QueuedCastKind,
    caster: Entity,
    target: Entity,
    destination: Option<[f32; 3]>,
    spell_id: u32,
    depth: u8,
    item_level: Option<u16>,
}

impl PendingTriggers {
    fn pop_due(&mut self, now_ms: u64) -> Option<QueuedTrigger> {
        let mut first = self.0.first_entry()?;
        if *first.key() > now_ms {
            return None;
        }
        let trigger = first.get_mut().pop_front().expect("nonempty due bucket");
        if first.get().is_empty() {
            first.remove();
        }
        Some(trigger)
    }
}

fn now_ms(world: &World) -> u64 {
    (super::now(world) * MILLIS_PER_SECOND) as u64
}

pub(super) fn schedule(
    world: &mut World,
    ctx: &super::EffectCtx,
    effect: &crate::spell_info::SpellEffect,
    target: Entity,
) {
    let due_ms = now_ms(world).wrapping_add_signed(i64::from(effect.misc_values[0]));
    let trigger = QueuedTrigger {
        kind: QueuedCastKind::CostedTrigger,
        caster: ctx.caster,
        target,
        destination: ctx.spell.cast_destination,
        spell_id: effect.trigger_spell,
        depth: ctx.depth + 1,
        item_level: ctx.spell.cast_item_level,
    };
    world
        .resource_mut::<PendingTriggers>()
        .0
        .entry(due_ms)
        .or_default()
        .push_back(trigger);
}

/// Effect160 reverses caster/target and uses ordinary cast time, costs and checks.
/// Its zero-delay callback follows the parent's SpellGo, never running inline.
pub(super) fn schedule_force_cast(
    world: &mut World,
    ctx: &super::EffectCtx,
    effect: &crate::spell_info::SpellEffect,
    target: Entity,
) {
    let due_ms = now_ms(world);
    let trigger = QueuedTrigger {
        kind: QueuedCastKind::NormalForce,
        caster: target,
        target: ctx.caster,
        destination: None,
        spell_id: effect.trigger_spell,
        depth: 0,
        item_level: None,
    };
    world
        .resource_mut::<PendingTriggers>()
        .0
        .entry(due_ms)
        .or_default()
        .push_back(trigger);
}

/// Remove one event before invoking it; nested zero-delay events follow it.
pub(crate) fn resolve_due(world: &mut World) {
    let now_ms = now_ms(world);
    loop {
        let Some(trigger) = world.resource_mut::<PendingTriggers>().pop_due(now_ms) else {
            return;
        };
        if world.get_entity(trigger.caster).is_err() || world.get_entity(trigger.target).is_err() {
            continue;
        }
        match trigger.kind {
            QueuedCastKind::CostedTrigger => {
                super::cast_costed_trigger_with_context(
                    world,
                    trigger.caster,
                    trigger.spell_id,
                    Some(trigger.target),
                    trigger.depth,
                    trigger.item_level,
                    trigger.destination,
                );
            }
            QueuedCastKind::NormalForce => {
                if let Err(failure) = super::creature::creature_cast(
                    world,
                    trigger.caster,
                    trigger.spell_id,
                    Some(trigger.target),
                    false,
                ) {
                    warn!("Forced spell {} skipped: {failure:?}", trigger.spell_id);
                }
            }
        }
    }
}
