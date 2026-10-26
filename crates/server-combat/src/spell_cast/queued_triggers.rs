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
    FullTrigger,
    RemoveOwnedAura,
    ScaledFullTrigger(f32),
}

pub struct DestinationTrigger {
    pub target: Entity,
    pub spell_id: u32,
    pub delay_ms: u32,
    pub destination: [f32; 3],
}

pub struct ScaledTrigger {
    pub target: Entity,
    pub spell_id: u32,
    pub delay_ms: u32,
    pub multiplier: f32,
}

struct QueuedTrigger {
    kind: QueuedCastKind,
    caster: Entity,
    target: Entity,
    destination: Option<[f32; 3]>,
    spell_id: u32,
    depth: u8,
    item_level: Option<u16>,
    damage_multiplier: Option<f32>,
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
    let Some(child) = world
        .resource::<crate::spell_info::SpellStore>()
        .get(effect.trigger_spell)
    else {
        warn!("Queued spell {} not found", effect.trigger_spell);
        return;
    };
    let Some(target) = super::effects::trigger_recipient(ctx, child, target) else {
        return;
    };
    let due_ms = now_ms(world).wrapping_add_signed(i64::from(effect.misc_values[0]));
    let trigger = QueuedTrigger {
        kind: QueuedCastKind::CostedTrigger,
        caster: ctx.caster,
        target,
        destination: ctx.spell.cast_destination,
        spell_id: effect.trigger_spell,
        depth: ctx.depth + 1,
        item_level: ctx.spell.cast_item_level,
        damage_multiplier: super::scripts::frost_flurry_damage_multiplier(
            world, ctx.caster, ctx.spell, child.id,
        ),
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
        damage_multiplier: None,
    };
    world
        .resource_mut::<PendingTriggers>()
        .0
        .entry(due_ms)
        .or_default()
        .push_back(trigger);
}

/// Scripted FULL_MASK callback, sharing the ordinary owner/target lifetime checks.
pub fn schedule_full_trigger(
    world: &mut World,
    ctx: &super::EffectCtx,
    target: Entity,
    spell_id: u32,
    delay_ms: u32,
) {
    let due_ms = now_ms(world) + u64::from(delay_ms);
    let trigger = QueuedTrigger {
        kind: QueuedCastKind::FullTrigger,
        caster: ctx.caster,
        target,
        destination: None,
        spell_id,
        depth: ctx.depth + 1,
        item_level: ctx.spell.cast_item_level,
        damage_multiplier: None,
    };
    world
        .resource_mut::<PendingTriggers>()
        .0
        .entry(due_ms)
        .or_default()
        .push_back(trigger);
}

/// SimC76f8118 sc_mage.cpp5585–5605/7272–7293: apply or clear a
/// feedback buff after the current hit batch, retaining native aura payloads.
pub fn schedule_buff_outcome(world: &mut World, caster: Entity, spell_id: u32, grant: bool) {
    let due_ms = now_ms(world);
    let trigger = QueuedTrigger {
        kind: if grant {
            QueuedCastKind::FullTrigger
        } else {
            QueuedCastKind::RemoveOwnedAura
        },
        caster,
        target: caster,
        destination: None,
        spell_id,
        depth: 1,
        item_level: None,
        damage_multiplier: None,
    };
    world
        .resource_mut::<PendingTriggers>()
        .0
        .entry(due_ms)
        .or_default()
        .push_back(trigger);
}

/// Preserve the authored destination of a delayed full trigger.
pub fn schedule_destination_trigger(
    world: &mut World,
    ctx: &super::EffectCtx,
    request: DestinationTrigger,
) {
    let due_ms = now_ms(world) + u64::from(request.delay_ms);
    let trigger = QueuedTrigger {
        kind: QueuedCastKind::FullTrigger,
        caster: ctx.caster,
        target: request.target,
        destination: Some(request.destination),
        spell_id: request.spell_id,
        depth: ctx.depth + 1,
        item_level: ctx.spell.cast_item_level,
        damage_multiplier: None,
    };
    world
        .resource_mut::<PendingTriggers>()
        .0
        .entry(due_ms)
        .or_default()
        .push_back(trigger);
}

/// A source-qualified delayed multiplier changes no native child coefficients.
pub fn schedule_scaled_full_trigger(
    world: &mut World,
    ctx: &super::EffectCtx,
    request: ScaledTrigger,
) {
    if !request.multiplier.is_finite() || request.multiplier < 0.0 {
        error!(
            spell = request.spell_id,
            multiplier = request.multiplier,
            "Invalid delayed damage multiplier"
        );
        return;
    }
    let due_ms = now_ms(world) + u64::from(request.delay_ms);
    let trigger = QueuedTrigger {
        kind: QueuedCastKind::ScaledFullTrigger(request.multiplier),
        caster: ctx.caster,
        target: request.target,
        destination: None,
        spell_id: request.spell_id,
        depth: ctx.depth + 1,
        item_level: ctx.spell.cast_item_level,
        damage_multiplier: None,
    };
    world
        .resource_mut::<PendingTriggers>()
        .0
        .entry(due_ms)
        .or_default()
        .push_back(trigger);
}

/// Source script callback using default TRIGGERED_NONE, not FULL_MASK.
/// Unlike effect160, this preserves the source caster and chosen target.
pub(super) fn schedule_normal_cast(
    world: &mut World,
    ctx: &super::EffectCtx,
    target: Entity,
    spell_id: u32,
    delay_ms: u32,
) {
    let due_ms = now_ms(world) + u64::from(delay_ms);
    let trigger = QueuedTrigger {
        kind: QueuedCastKind::NormalForce,
        caster: ctx.caster,
        target,
        destination: None,
        spell_id,
        depth: 0,
        item_level: None,
        damage_multiplier: None,
    };
    world
        .resource_mut::<PendingTriggers>()
        .0
        .entry(due_ms)
        .or_default()
        .push_back(trigger);
}

fn resolve_costed_trigger(world: &mut World, trigger: QueuedTrigger) {
    let Some(mut spell) = super::load_triggered_spell(world, trigger.spell_id, trigger.depth)
    else {
        return;
    };
    spell.cast_item_level = trigger.item_level;
    spell.cast_destination = trigger.destination;
    spell.cast_damage_multiplier = trigger.damage_multiplier;
    // Retain Effect64's native payment/check path with its frozen launch value.
    if let Err(failure) = super::execute_costed_trigger(
        world,
        trigger.caster,
        &spell,
        Some(trigger.target),
        trigger.depth,
    ) {
        warn!(
            spell = trigger.spell_id,
            ?failure,
            "Queued native costed trigger refused"
        );
    }
}

/// Remove one event before invoking it; nested zero-delay events follow it.
pub(crate) fn resolve_due(world: &mut World) {
    super::scripts::resolve_frost_mage_events(world);
    let now_ms = now_ms(world);
    loop {
        let Some(trigger) = world.resource_mut::<PendingTriggers>().pop_due(now_ms) else {
            return;
        };
        if world.get_entity(trigger.caster).is_err() || world.get_entity(trigger.target).is_err() {
            continue;
        }
        match trigger.kind {
            QueuedCastKind::CostedTrigger => resolve_costed_trigger(world, trigger),
            QueuedCastKind::FullTrigger => {
                let Some(mut spell) =
                    super::load_triggered_spell(world, trigger.spell_id, trigger.depth)
                else {
                    continue;
                };
                spell.cast_item_level = trigger.item_level;
                spell.cast_destination = trigger.destination;
                if !super::launch_triggered_spell(
                    world,
                    trigger.caster,
                    &spell,
                    Some(trigger.target),
                    trigger.depth,
                ) {
                    warn!("Delayed scripted spell {} refused", trigger.spell_id);
                }
            }
            QueuedCastKind::ScaledFullTrigger(multiplier) => {
                let Some(mut spell) = world
                    .resource::<super::SpellStore>()
                    .get(trigger.spell_id)
                    .cloned()
                else {
                    warn!(
                        spell = trigger.spell_id,
                        "Delayed scaled spell missing native data"
                    );
                    continue;
                };
                spell.cast_item_level = trigger.item_level;
                spell.cast_damage_multiplier = Some(multiplier);
                if !super::launch_triggered_spell(
                    world,
                    trigger.caster,
                    &spell,
                    Some(trigger.target),
                    trigger.depth,
                ) {
                    warn!(spell = trigger.spell_id, "Delayed scaled spell refused");
                }
            }
            QueuedCastKind::RemoveOwnedAura => {
                crate::auras::remove_auras(
                    world,
                    trigger.target,
                    crate::auras::RemoveReason::Consumed,
                    |aura| aura.spell_id == trigger.spell_id && aura.caster == trigger.caster,
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
