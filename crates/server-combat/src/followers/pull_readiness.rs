//! Budget capability, not a cast command or a default-heal substitute.
use bevy::prelude::*;
use shared::group::GroupRole;

use super::{Follower, FollowerActionSpells};
use crate::spell_cast::{self, CastFailure};
use crate::spell_info::SpellStore;

#[derive(Debug)]
pub(super) struct ReadyHeal {
    pub healer: Entity,
    pub spell_id: u32,
    pub costs: Vec<(spell_cast::CostPool, f32)>,
}

pub(super) fn read_ready_heal(
    world: &mut World,
    tank: Entity,
    party: &[Entity],
) -> Result<Option<ReadyHeal>, String> {
    let candidates = read_heal_candidates(world, party)?;
    for (healer, spell_id) in candidates {
        if let Some(ready) = read_spell_readiness(world, healer, tank, spell_id)? {
            return Ok(Some(ready));
        }
    }
    Ok(None)
}

fn read_heal_candidates(world: &World, party: &[Entity]) -> Result<Vec<(Entity, u32)>, String> {
    let store = world
        .get_resource::<SpellStore>()
        .ok_or("Tank heal readiness requires the spell store")?;
    let mut candidates = Vec::new();
    for &unit in party {
        let Some(follower) = world.get::<Follower>(unit) else {
            continue;
        };
        if follower.profile.role != GroupRole::Healer {
            continue;
        }
        let actions = world.get::<FollowerActionSpells>(unit).ok_or_else(|| {
            format!("Tank heal readiness lacks action metadata for healer {unit}")
        })?;
        for &action in &actions.0 {
            let spell_id = match spell_cast::knowledge::resolve_actor_action(world, unit, action) {
                Ok(spell_id) => spell_id,
                Err(CastFailure::UnknownSpell | CastFailure::NotKnown) => continue,
                Err(error) => {
                    return Err(format!("Tank heal readiness action {action}: {error:?}"));
                }
            };
            let spell = store
                .get(spell_id)
                .ok_or_else(|| format!("Tank heal readiness lacks spell {spell_id}"))?;
            if has_healing_effect(store, spell, 0) {
                candidates.push((unit, spell_id));
            }
        }
    }
    Ok(candidates)
}

fn has_healing_effect(store: &SpellStore, spell: &crate::spell_info::SpellInfo, depth: u8) -> bool {
    spell.effects.iter().any(|effect| {
        effect.effect == 10
            || effect.aura == crate::auras::handlers::PERIODIC_HEAL
            || (depth < crate::spell_cast::effects::MAX_TRIGGER_DEPTH
                && effect.trigger_spell != 0
                && store
                    .get(effect.trigger_spell)
                    .is_some_and(|child| has_healing_effect(store, child, depth + 1)))
    })
}

fn read_spell_readiness(
    world: &mut World,
    healer: Entity,
    tank: Entity,
    spell_id: u32,
) -> Result<Option<ReadyHeal>, String> {
    let spell = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(spell_id))
        .cloned()
        .ok_or_else(|| format!("Tank heal readiness lacks sourced spell {spell_id}"))?;
    if !read_heal_capability(world, healer, tank, &spell)? {
        return Ok(None);
    }
    let costs = spell_cast::spell_costs(world, healer, &spell)
        .map_err(|error| format!("Tank heal readiness spell {spell_id}: {error:?}"))?;
    Ok(Some(ReadyHeal {
        healer,
        spell_id,
        costs,
    }))
}

fn read_heal_capability(
    world: &mut World,
    healer: Entity,
    tank: Entity,
    spell: &crate::spell_info::SpellInfo,
) -> Result<bool, String> {
    // Patient health priorities do not limit capability. Native GCD clocks stay intact.
    match spell_cast::read_tank_heal_capability(world, healer, spell, tank) {
        Ok(Some(target)) if target == tank => {}
        Err(CastFailure::Unsupported(reason)) => return Err(reason),
        _ => return Ok(false),
    }
    spell_cast::read_targets_include(world, healer, spell, tank)
        .map_err(|error| format!("Tank heal readiness spell {}: {error:?}", spell.id))
}
