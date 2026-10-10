//! Aura81 splits already-mitigated incoming damage, not a second offensive cast.
use bevy::prelude::*;
use shared::components::Health;

use super::{ActiveAuras, RemoveReason};
use crate::maps::InMap;
use crate::spell_info::SpellStore;

pub const SPLIT_DAMAGE_PCT: u32 = 81;

#[derive(Clone, Copy)]
pub(crate) struct Split {
    pub recipient: Entity,
    pub instance: u32,
    pub spell: u32,
    pub fraction: f32,
    pub floor_pct: f32,
    pub budget: f32,
    pub health_at_plan: f32,
}

/// Pure selection; callers validate living same-map recipients before applying it.
pub(crate) fn candidates(
    auras: Option<&ActiveAuras>,
    store: Option<&SpellStore>,
    school: u32,
) -> Vec<Split> {
    let mut result = Vec::new();
    for aura in auras.into_iter().flat_map(ActiveAuras::iter) {
        for effect in &aura.effects {
            if effect.aura_type != SPLIT_DAMAGE_PCT || effect.misc_values[0] as u32 & school == 0 {
                continue;
            }
            // Native6940s3 is its termination floor; no universal aura81 floor.
            let floor_pct = if aura.spell_id == 6940 {
                store
                    .and_then(|store| store.get(6940))
                    .and_then(|spell| spell.effects.iter().find(|effect| effect.index == 2))
                    .map_or(0.0, |effect| effect.base_points)
            } else {
                0.0
            };
            result.push(Split {
                recipient: aura.caster,
                instance: aura.instance_id,
                spell: aura.spell_id,
                fraction: (effect.amount * f32::from(aura.stacks) / 100.0).clamp(0.0, 1.0),
                floor_pct,
                budget: f32::INFINITY,
                health_at_plan: 0.0,
            });
        }
    }
    result
}

pub(crate) fn route(
    world: &mut World,
    attacker: Entity,
    victim: Entity,
    school: u32,
    mut amount: f32,
) -> f32 {
    let splits = candidates(
        world.get::<ActiveAuras>(victim),
        world.get_resource::<SpellStore>(),
        school,
    );
    for mut split in splits {
        if split.recipient == victim
            || world.get::<InMap>(split.recipient) != world.get::<InMap>(victim)
            || !crate::spell_cast::targets::is_alive(world, split.recipient)
            || super::is_immune(world, split.recipient, school)
        {
            continue;
        }
        let health = world.get::<Health>(split.recipient).unwrap();
        let multiplier = crate::level_scaling::UnitScaling::of(world, split.recipient)
            .health_multiplier(&crate::level_scaling::UnitScaling::of(world, attacker));
        split.budget =
            (health.current - health.max * split.floor_pct / 100.0).max(0.0) * multiplier;
        let share = (amount * split.fraction).min(split.budget);
        amount -= share;
        deliver(world, attacker, victim, school, split, share);
    }
    amount
}

/// No recursive routing: a split cannot bounce through a cycle of Sacrifice auras.
pub(crate) fn deliver(
    world: &mut World,
    attacker: Entity,
    victim: Entity,
    school: u32,
    split: Split,
    amount: f32,
) {
    if amount > 0.0 {
        let (remaining, absorbed) = super::absorb_damage(world, split.recipient, school, amount);
        crate::combat::deal_health_damage(
            world,
            attacker,
            split.recipient,
            remaining,
            school,
            Some(split.spell),
            absorbed,
        );
    }
    let reached_floor = world
        .get::<Health>(split.recipient)
        .is_some_and(|health| health.current <= health.max * split.floor_pct / 100.0);
    if split.floor_pct > 0.0 && reached_floor {
        super::remove_instance(world, victim, split.instance, RemoveReason::Consumed);
    }
}
