//! Party-wide scheduling is FOLLOWER_UTILITY_POLICY, not spell data.
use super::Follower;
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;
use shared::casting::CastState;
use std::collections::BTreeSet;

#[derive(Component)]
struct LastKick(Entity);

#[derive(Component)]
struct ReservedCast {
    spell: u32,
    target: u64,
    started: f64,
    expires: f64,
}

pub(super) fn spell_danger(world: &World, id: u32) -> u8 {
    let store = world.resource::<SpellStore>();
    let mut pending = vec![id];
    let mut visited = BTreeSet::new();
    let mut danger = 0;
    while let Some(id) = pending.pop() {
        if !visited.insert(id) {
            continue;
        }
        let Some(spell) = store.get(id) else { continue };
        danger = danger.max(direct_danger(spell));
        pending.extend(
            spell
                .effects
                .iter()
                .map(|effect| effect.trigger_spell)
                .filter(|child| *child != 0),
        );
    }
    danger
}

fn direct_danger(spell: &SpellInfo) -> u8 {
    spell
        .effects
        .iter()
        .map(|effect| {
            if crate::auras::is_aura_effect(effect)
                && matches!(effect.aura, 5 | 7 | 12 | 26 | 27 | 60)
            {
                4
            } else if matches!(effect.effect, 10 | 18 | 67 | 136 | 172) {
                3
            } else if matches!(effect.effect, 2 | 9 | 17 | 31 | 58 | 121)
                || matches!(effect.aura, 3 | 53 | 89)
            {
                2
            } else {
                1
            }
        })
        .max()
        .unwrap_or(0)
}

pub(super) fn ordered_casts(world: &World, enemies: &[(Entity, Option<Entity>)]) -> Vec<Entity> {
    let mut casts: Vec<_> = enemies
        .iter()
        .filter_map(|(enemy, _)| {
            let cast = world.get::<CastState>(*enemy)?;
            let protected =
                crate::auras::has_breakable_by_damage_cc(world, *enemy) || reserved(world, *enemy);
            (!protected).then_some((
                *enemy,
                spell_danger(world, cast.spell_id),
                cast.duration - cast.elapsed,
            ))
        })
        .collect();
    casts.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.2.total_cmp(&b.2))
            .then_with(|| a.0.to_bits().cmp(&b.0.to_bits()))
    });
    casts.into_iter().map(|(enemy, _, _)| enemy).collect()
}

pub(super) fn try_party(
    world: &mut World,
    owner: Entity,
    party: &[Entity],
    candidates: &[Entity],
) -> bool {
    let last = world.get::<LastKick>(owner).map(|last| last.0.to_bits());
    let mut members: Vec<_> = party
        .iter()
        .filter_map(|unit| {
            let follower = world.get::<Follower>(*unit)?;
            crate::spell_cast::targets::is_alive(world, *unit)
                .then_some((*unit, follower.profile.clone()))
        })
        .collect();
    members.sort_by_key(|(unit, _)| {
        (
            last.is_some_and(|last| unit.to_bits() <= last),
            unit.to_bits(),
        )
    });
    for (unit, profile) in members {
        if super::kit_interrupt::try_interrupt_candidates(world, unit, &profile, candidates) {
            return true;
        }
    }
    false
}

pub(super) fn reserved(world: &World, enemy: Entity) -> bool {
    let Some(reservation) = world.get::<ReservedCast>(enemy) else {
        return false;
    };
    let Some(cast) = world.get::<CastState>(enemy) else {
        return false;
    };
    let now = world.resource::<Time>().elapsed_secs_f64();
    let started = now - f64::from(cast.elapsed);
    reservation.spell == cast.spell_id
        && reservation.target == cast.target
        && (reservation.started - started).abs() < 0.01
        && now <= reservation.expires
}

pub(super) fn reserve(world: &mut World, unit: Entity, enemy: Entity, cast: &CastState) {
    let now = world.resource::<Time>().elapsed_secs_f64();
    world.entity_mut(enemy).insert(ReservedCast {
        spell: cast.spell_id,
        target: cast.target,
        started: now - f64::from(cast.elapsed),
        expires: now + f64::from(cast.duration - cast.elapsed).max(0.0),
    });
    if let Some(owner) = world.get::<Follower>(unit).map(|follower| follower.owner) {
        world.entity_mut(owner).insert(LastKick(unit));
    }
}

pub(super) fn dispel_harm(world: &World, dispeller: Entity, target: Entity, types: &[u32]) -> u8 {
    let hostile = crate::spell_cast::targets::is_hostile(world, dispeller, target);
    world
        .get::<crate::auras::ActiveAuras>(target)
        .into_iter()
        .flat_map(|auras| auras.iter())
        .filter(|aura| {
            types
                .iter()
                .any(|kind| crate::auras::aura_can_be_dispelled(aura, hostile, *kind))
        })
        .map(|aura| {
            if aura
                .effects
                .iter()
                .any(|effect| matches!(effect.aura_type, 5 | 7 | 12 | 26 | 27 | 60))
            {
                3
            } else if aura
                .effects
                .iter()
                .any(|effect| matches!(effect.aura_type, 3 | 53 | 89))
            {
                2
            } else {
                1
            }
        })
        .max()
        .unwrap_or(0)
}
