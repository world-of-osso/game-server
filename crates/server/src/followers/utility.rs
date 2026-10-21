//! FOLLOWER_UTILITY_POLICY. Native spellbooks and execution, project decisions.
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{Health, Npc, Position};
use shared::group::GroupRole;
use std::collections::VecDeque;

use super::kit::{Entry, Slot, Target};
use super::{Follower, FollowerCatalog};
use crate::maps::InMap;
use crate::spell_info::SpellStore;

const DEATH_HORIZON: f64 = 3.0;
const BOSS_PULL_SECONDS: f64 = 10.0;
const DUNGEON_BOSS_FLAG: u32 = 0x1000_0000;
const WORLD_BOSS_FLAG: u32 = 0x4;
const RAID_BUFFS: &[u32] = &[1126, 1459, 21562, 6673, 462854, 364342];
const LUST: &[u32] = &[2825, 32182, 80353, 390386];
// Soulstone20707/6203 has no native trigger edge to a resurrection effect.
const RESURRECTIONS: &[u32] = &[20484, 391054];
const EXHAUSTION: &[u32] = &[57723, 57724, 80354, 264689, 390435];

#[derive(Component)]
struct HealthTrend(VecDeque<(f64, f32)>);

#[derive(Component)]
struct BossPull {
    boss: Entity,
    started: f64,
    lust_used: bool,
}

#[derive(Component)]
struct ResurrectionCast {
    caster: Entity,
    spell: u32,
}

pub(super) fn sample_health(world: &mut World, unit: Entity) {
    let Some(health) = world.get::<Health>(unit) else {
        return;
    };
    if health.max <= 0.0 {
        return;
    }
    let fraction = health.current / health.max;
    let now = world.resource::<Time>().elapsed_secs_f64();
    if world.get::<HealthTrend>(unit).is_none() {
        world.entity_mut(unit).insert(HealthTrend(VecDeque::new()));
    }
    let mut trend = world.get_mut::<HealthTrend>(unit).unwrap();
    while trend.0.len() > 1 && trend.0[1].0 <= now - DEATH_HORIZON {
        trend.0.pop_front();
    }
    if trend.0.back().is_none_or(|sample| sample.0 != now) {
        trend.0.push_back((now, fraction));
    }
}

pub(super) fn projected_death(world: &World, target: Entity) -> bool {
    let Some(trend) = world.get::<HealthTrend>(target) else {
        return false;
    };
    let (Some(&(start, before)), Some(&(now, current))) = (trend.0.front(), trend.0.back()) else {
        return false;
    };
    let elapsed = now - start;
    let losing_health = before > current && elapsed > 0.0 && current > 0.0;
    losing_health && f64::from(current) <= f64::from(before - current) / elapsed * DEATH_HORIZON
}

pub(super) fn is_raid_buff(spell: u32) -> bool {
    RAID_BUFFS.contains(&spell)
}

pub(super) fn active_defensive(world: &World, target: Entity) -> bool {
    let Some(follower) = world.get::<Follower>(target) else {
        return false;
    };
    world
        .resource::<FollowerCatalog>()
        .kits
        .iter()
        .filter(|entry| entry.spec_id == follower.profile.spec_id)
        .cloned()
        .map(normalize_entry)
        .any(|entry| {
            entry.slot == Slot::Defensive
                && entry.conditions.target == Target::SelfUnit
                && entry
                    .conditions
                    .missing_aura
                    .is_some_and(|aura| crate::auras::has_aura(world, target, aura))
        })
}

pub(super) fn normalize_entry(mut entry: Entry) -> Entry {
    let id = entry.spell_id;
    match id {
        642 | 23920 | 110959 | 414658 => {
            entry.slot = Slot::Defensive;
            entry.conditions.target = Target::SelfUnit;
            entry.conditions.missing_aura = Some(id);
        }
        1022 | 6940 | 204018 | 97462 => {
            entry.slot = Slot::Defensive;
            entry.conditions.target = Target::Party;
            entry.conditions.missing_aura = Some(id);
        }
        8122 | 6789 | 30283 | 115750 => {
            entry.slot = Slot::Control;
            entry.conditions.target = Target::Enemy;
            entry.conditions.requires_cast = true;
            entry.conditions.source_center = matches!(id, 8122 | 115750);
            entry.conditions.min_enemies =
                usize::from(entry.conditions.source_center || id == 30283);
        }
        _ => {}
    }
    if is_raid_buff(id) {
        entry.conditions.target = Target::Party;
    }
    if entry.slot == Slot::Defensive {
        entry.conditions.health_below = None;
        entry.conditions.attacked = false;
    }
    entry
}

/// Learned intervals remain authoritative; no scheduler spell grants.
fn learned(world: &World, unit: Entity, follower: &Follower, ids: &[u32]) -> Vec<u32> {
    let level = world
        .get::<shared::components::Level>(unit)
        .map_or(0, |level| level.0);
    world
        .resource::<FollowerCatalog>()
        .kits
        .iter()
        .filter(|entry| {
            entry.available(follower.profile.spec_id, level) && ids.contains(&entry.spell_id)
        })
        .map(|entry| entry.spell_id)
        .collect()
}

pub(super) fn try_support(
    world: &mut World,
    unit: Entity,
    follower: &Follower,
    party: &[Entity],
    enemies: &[(Entity, Option<Entity>)],
) -> bool {
    if enemies.is_empty() {
        world.entity_mut(follower.owner).remove::<BossPull>();
        return false;
    }
    if world
        .get::<CastState>(unit)
        .is_some_and(|cast| RESURRECTIONS.contains(&cast.spell_id))
    {
        return true;
    }
    try_battle_res(world, unit, follower, party) || try_lust(world, unit, follower, party, enemies)
}

fn role(world: &World, unit: Entity) -> Option<GroupRole> {
    if let Some(follower) = world.get::<Follower>(unit) {
        return Some(follower.profile.role);
    }
    let spec = world.get::<shared::components::ActiveSpec>(unit)?.0;
    world
        .get_resource::<crate::spell_info::ClassSpells>()?
        .classes
        .iter()
        .flat_map(|class| &class.specs)
        .find(|entry| entry.spec_id == spec)
        .map(|entry| match entry.role {
            0 => GroupRole::Tank,
            1 => GroupRole::Healer,
            _ => GroupRole::Dps,
        })
}

fn res_reserved(world: &World, target: Entity) -> bool {
    world
        .get::<ResurrectionCast>(target)
        .is_some_and(|reservation| {
            world
                .get::<CastState>(reservation.caster)
                .is_some_and(|cast| {
                    cast.spell_id == reservation.spell && cast.target == target.to_bits()
                })
        })
}

fn try_battle_res(world: &mut World, unit: Entity, follower: &Follower, party: &[Entity]) -> bool {
    let target = party
        .iter()
        .copied()
        .filter(|target| {
            world.get::<InMap>(*target) == world.get::<InMap>(unit)
                && world
                    .get::<Health>(*target)
                    .is_some_and(|health| health.current <= 0.0)
                && matches!(
                    role(world, *target),
                    Some(GroupRole::Healer | GroupRole::Tank)
                )
                && !res_reserved(world, *target)
                && world
                    .get::<crate::death::resurrection::PendingResurrection>(*target)
                    .is_none()
        })
        .min_by_key(|target| {
            (
                role(world, *target) != Some(GroupRole::Healer),
                target.to_bits(),
            )
        });
    let Some(target) = target else { return false };
    for spell in learned(world, unit, follower, RESURRECTIONS) {
        if cast_utility(world, unit, spell, target, true) {
            world.entity_mut(target).insert(ResurrectionCast {
                caster: unit,
                spell,
            });
            return true;
        }
    }
    false
}

fn is_boss(world: &World, unit: Entity) -> bool {
    let Some(npc) = world.get::<Npc>(unit) else {
        return false;
    };
    world
        .resource::<crate::world_data::WorldData>()
        .templates
        .get(&npc.template_id)
        .is_some_and(|template| {
            template.flags_extra & DUNGEON_BOSS_FLAG != 0
                || template.type_flags & WORLD_BOSS_FLAG != 0
        })
}

fn try_lust(
    world: &mut World,
    unit: Entity,
    follower: &Follower,
    party: &[Entity],
    enemies: &[(Entity, Option<Entity>)],
) -> bool {
    let Some(boss) = enemies
        .iter()
        .map(|(enemy, _)| *enemy)
        .find(|enemy| is_boss(world, *enemy))
    else {
        world.entity_mut(follower.owner).remove::<BossPull>();
        return false;
    };
    let now = world.resource::<Time>().elapsed_secs_f64();
    if world
        .get::<BossPull>(follower.owner)
        .is_none_or(|pull| pull.boss != boss)
    {
        world.entity_mut(follower.owner).insert(BossPull {
            boss,
            started: now,
            lust_used: false,
        });
    }
    let pull = world.get::<BossPull>(follower.owner).unwrap();
    let exhausted = party.iter().any(|member| {
        EXHAUSTION
            .iter()
            .chain(LUST)
            .any(|aura| crate::auras::has_aura(world, *member, *aura))
    });
    if pull.lust_used || now - pull.started > BOSS_PULL_SECONDS || exhausted {
        return false;
    }
    for spell in learned(world, unit, follower, LUST) {
        if cast_utility(world, unit, spell, unit, false) {
            world.get_mut::<BossPull>(follower.owner).unwrap().lust_used = true;
            return true;
        }
    }
    false
}

pub(super) fn try_movement(
    world: &mut World,
    unit: Entity,
    target: Entity,
    follower: &Follower,
    attack: u32,
) -> bool {
    let pursuing = matches!(world.get::<crate::creature_ai::CreatureState>(unit), Some(crate::creature_ai::CreatureState::Pursuing { target: moving }) if *moving == target.to_bits());
    if !pursuing || crate::auras::has_breakable_by_damage_cc(world, target) {
        return false;
    }
    let Some(distance) = distance(world, unit, target) else {
        return false;
    };
    let reach = if follower.profile.melee {
        super::ai::MELEE_REACH
    } else {
        world
            .resource::<SpellStore>()
            .get(attack)
            .and_then(|spell| spell.misc.as_ref())
            .and_then(|misc| misc.range)
            .map_or(f32::INFINITY, |range| range.max_hostile)
    };
    if distance <= reach {
        return false;
    }
    // Divine Steed190784 is an unimplemented dummy wrapper, not a speed aura.
    let ids = [100, 385952, 252216, 1850, 186257, 58875, 116841, 358267];
    for spell in learned(world, unit, follower, &ids) {
        if crate::auras::has_aura(world, unit, spell) {
            continue;
        }
        let recipient = if matches!(spell, 100 | 385952) {
            target
        } else {
            unit
        };
        if cast_utility(world, unit, spell, recipient, false) {
            return true;
        }
    }
    false
}

fn distance(world: &World, unit: Entity, target: Entity) -> Option<f32> {
    let a = world.get::<Position>(unit)?;
    let b = world.get::<Position>(target)?;
    Some(Vec3::new(a.x, a.y, a.z).distance(Vec3::new(b.x, b.y, b.z)))
}

fn cast_utility(
    world: &mut World,
    caster: Entity,
    spell: u32,
    target: Entity,
    urgent: bool,
) -> bool {
    let result = if urgent {
        crate::spell_cast::creature_cast_interrupting_self(world, caster, spell, Some(target))
    } else {
        crate::spell_cast::creature_cast(world, caster, spell, Some(target), false)
    };
    match result {
        Ok(()) => true,
        Err(error) => {
            debug!(
                ?caster,
                spell,
                ?target,
                ?error,
                "Follower utility unavailable"
            );
            false
        }
    }
}
