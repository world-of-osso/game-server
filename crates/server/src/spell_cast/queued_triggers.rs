//! Effect64 schedules a non-reentrant child (TC SpellEffects.cpp:691–723).
use std::collections::{BTreeMap, VecDeque};

use bevy::prelude::*;

const MILLIS_PER_SECOND: f64 = 1000.0;

#[derive(Resource, Default)]
pub(crate) struct PendingTriggers(BTreeMap<u64, VecDeque<QueuedTrigger>>);

struct QueuedTrigger {
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
}
