//! Formation and first-wave native assistance snapshots for a pull.
use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Component, PartialEq, Eq)]
struct LoggedPull {
    target: Entity,
    count: usize,
}

pub(super) fn log_admission(world: &mut World, tank: Entity, target: Entity) {
    let planned = members(world, target, tank);
    let count = planned.len();
    let current = LoggedPull { target, count };
    if world
        .get::<LoggedPull>(tank)
        .is_none_or(|previous| previous.target != target)
    {
        let mut member_ids: Vec<_> = planned.into_iter().map(Entity::to_bits).collect();
        member_ids.sort_unstable();
        debug!(target: "game_server::follower_pull", tank = tank.to_bits(), target = target.to_bits(), ?member_ids, "planned pull closure");
    }
    if count > 1 && world.get::<LoggedPull>(tank) != Some(&current) {
        info!(target: "game_server::follower_pull", ?tank, ?target, count, "planned pull includes formation or assistance");
    }
    world.entity_mut(tank).insert(current);
}

fn assistants(world: &World, target: Entity, victim: Entity) -> Vec<Entity> {
    let assistants = if let Some(snapshot) =
        world.get::<crate::creature_assistance::AssistanceSnapshot>(target)
    {
        snapshot.0.clone()
    } else if world
        .get::<crate::creature_assistance::AssistanceCalled>(target)
        .is_some()
    {
        Vec::new()
    } else {
        crate::creature_assistance::assistance_snapshot(world, target, victim)
    };
    assistants
        .into_iter()
        .filter(|unit| crate::spell_cast::targets::is_alive(world, *unit))
        .collect()
}

fn formation(world: &World, target: Entity) -> HashSet<Entity> {
    if !crate::spell_cast::targets::is_alive(world, target) {
        return HashSet::new();
    }
    let mut result = HashSet::from([target]);
    if let (Some(map), Some(units)) = (
        world.get::<crate::maps::InMap>(target),
        world.get_resource::<crate::maps::units::MapUnits>(),
    ) {
        result.extend(units.in_map(*map).filter(|&candidate| {
            crate::spell_cast::targets::is_alive(world, candidate)
                && super::positioning::in_target_formation(world, target, candidate)
        }));
    }
    result
}

pub(super) fn first_wave(world: &World, target: Entity, victim: Entity) -> HashSet<Entity> {
    let mut result = formation(world, target);
    for caller in result.clone() {
        result.extend(assistants(world, caller, victim));
    }
    result
}

pub(super) fn pending_helpers(world: &World, target: Entity, victim: Entity) -> HashSet<Entity> {
    let uncalled = world
        .get::<crate::creature_assistance::AssistanceCalled>(target)
        .is_none()
        .then_some(target);
    crate::creature_assistance::pending_callers(world)
        .chain(uncalled)
        .filter(|caller| {
            world.get::<crate::maps::InMap>(*caller) == world.get::<crate::maps::InMap>(victim)
        })
        .flat_map(|caller| assistants(world, caller, victim))
        .filter(|helper| {
            world
                .get::<crate::creature_assistance::AssistanceCalled>(*helper)
                .is_none()
        })
        .collect()
}

pub(super) fn committed_helpers(world: &World, victim: Entity) -> HashSet<Entity> {
    crate::creature_assistance::committed_assistants(world)
        .filter(|helper| {
            world.get::<crate::maps::InMap>(*helper) == world.get::<crate::maps::InMap>(victim)
        })
        .collect()
}

#[cfg(test)]
pub(super) fn risky_pending_helpers(
    world: &World,
    target: Entity,
    victim: Entity,
) -> HashSet<Entity> {
    let uncalled = world
        .get::<crate::creature_assistance::AssistanceCalled>(target)
        .is_none()
        .then_some(target);
    crate::creature_assistance::pending_callers(world)
        .chain(uncalled)
        .filter(|caller| {
            world.get::<crate::maps::InMap>(*caller) == world.get::<crate::maps::InMap>(victim)
        })
        .flat_map(|caller| {
            let first = first_wave(world, caller, victim);
            assistants(world, caller, victim)
                .into_iter()
                .filter(move |helper| {
                    world
                        .get::<crate::creature_assistance::AssistanceCalled>(*helper)
                        .is_none()
                        && first_wave(world, *helper, victim)
                            .iter()
                            .any(|member| !first.contains(member))
                })
        })
        .collect()
}

/// Boss sight envelopes for one party member, computed from native aggro geometry.
#[derive(Clone)]
struct BossPullUnit {
    boss: Entity,
    members: HashSet<Entity>,
    intended_packs: HashSet<u64>,
}

#[derive(Component, Default)]
struct BossPullUnits(Vec<BossPullUnit>);

#[cfg(test)]
pub(super) fn install_test_boss_unit(
    world: &mut World,
    victim: Entity,
    boss: Entity,
    members: &[Entity],
) {
    let intended_packs = members
        .iter()
        .map(|&actor| pack(world, actor).expect("test source pack"))
        .collect();
    world
        .entity_mut(victim)
        .insert(BossPullUnits(vec![BossPullUnit {
            boss,
            members: members.iter().copied().collect(),
            intended_packs,
        }]));
}

fn pack(world: &World, actor: Entity) -> Option<u64> {
    let guid = world.get::<crate::zones::CreatureOrigin>(actor)?.guid;
    let data = world.get_resource::<crate::world_data::WorldData>()?;
    Some(super::pack_identity::pack_guid(
        guid,
        data.formations.get(&guid).map(|entry| entry.leader_guid),
    ))
}

pub(super) fn merged_pack_count(world: &World, target: Entity, victim: Entity) -> usize {
    let selected = members(world, target, victim);
    let packs: HashSet<_> = world
        .get::<BossPullUnits>(victim)
        .into_iter()
        .flat_map(|units| &units.0)
        .filter(|unit| !selected.is_disjoint(&unit.members))
        .flat_map(|unit| unit.intended_packs.iter().copied())
        .collect();
    packs.len().max(1)
}

/// Refresh before planning: one boss and every formation whose melee attack point
/// enters its sight form one pull unit. The boss uses its live position.
pub(super) fn refresh_boss_units(world: &mut World, victim: Entity) -> Result<(), String> {
    let Some(map) = world.get::<crate::maps::InMap>(victim).copied() else {
        return Ok(());
    };
    let Some(script) = crate::scripted_ai::instance_script::instance_script(world, map) else {
        world.entity_mut(victim).insert(BossPullUnits::default());
        return Ok(());
    };
    let entries: HashSet<_> = script
        .boss_creatures
        .iter()
        .map(|(entry, _)| *entry)
        .collect();
    let actors: Vec<_> = world
        .resource::<crate::maps::units::MapUnits>()
        .in_map(map)
        .filter(|actor| {
            world.get::<crate::zones::CreatureOrigin>(*actor).is_some()
                && world
                    .get::<crate::creature_ai::SpawnPoint>(*actor)
                    .is_some()
                && crate::spell_cast::targets::is_alive(world, *actor)
                && crate::spell_cast::targets::relation_matches(
                    world,
                    victim,
                    *actor,
                    crate::spell_cast::targets::Relation::Enemy,
                )
        })
        .collect();
    let active_units: Vec<_> = world
        .get::<BossPullUnits>(victim)
        .into_iter()
        .flat_map(|units| &units.0)
        .filter(|unit| {
            crate::spell_cast::targets::is_alive(world, unit.boss)
                && world
                    .get::<shared::components::CombatStatus>(unit.boss)
                    .is_some_and(|status| status.0)
        })
        .cloned()
        .collect();
    let bosses: Vec<_> = actors
        .iter()
        .copied()
        .filter(|actor| {
            world
                .get::<shared::components::Npc>(*actor)
                .is_some_and(|npc| entries.contains(&npc.template_id))
                && (super::positioning::is_unpulled_hostile(world, victim, *actor)
                    || active_units.iter().any(|unit| unit.boss == *actor))
        })
        .collect();
    let units = crate::system_states::with_state::<crate::creature_aggro::AggroDistance, _>(
        world,
        |world, geometry| -> Result<Vec<BossPullUnit>, String> {
            let mut units = Vec::new();
            for boss in bosses {
                if let Some(active) = active_units.iter().find(|unit| unit.boss == boss) {
                    units.push(active.clone());
                    continue;
                }
                if let Some(unit) = read_boss_pull_unit(world, geometry, victim, boss, &actors)? {
                    units.push(unit);
                }
            }
            Ok(units)
        },
    )?;
    world.entity_mut(victim).insert(BossPullUnits(units));
    Ok(())
}

fn read_boss_pull_unit(
    world: &World,
    geometry: &mut bevy::ecs::system::SystemState<crate::creature_aggro::AggroDistance>,
    victim: Entity,
    boss: Entity,
    actors: &[Entity],
) -> Result<Option<BossPullUnit>, String> {
    let center =
        crate::spell_cast::targets::position(world, boss).ok_or("Boss sight lacks position")?;
    let radius = super::pull_route::read_native_radius(world, geometry, victim, boss)?;
    let sight = crate::line_of_sight::ProjectedSight::capture(world, boss, victim);
    let los = world.get_resource::<crate::line_of_sight::LineOfSight>();
    let mut members = first_wave(world, boss, victim);
    let mut intended_packs = HashSet::from([pack(world, boss).ok_or("Boss lacks pack identity")?]);
    for &actor in actors {
        if actor == boss {
            continue;
        }
        let home = world.get::<crate::creature_ai::SpawnPoint>(actor).unwrap();
        let home = Vec3::new(home.x, home.y, home.z);
        let melee = crate::scripted_ai::model_reach::melee_range(world, victim, actor)
            .ok_or("Merged actor lacks native melee geometry")?;
        let toward_boss = center - home;
        let point = home + toward_boss.normalize_or_zero() * melee.min(toward_boss.length());
        if center.distance_squared(point) <= radius * radius && sight.sees(los, point) {
            intended_packs.insert(pack(world, actor).ok_or("Merged actor lacks pack identity")?);
            members.extend(first_wave(world, actor, victim));
        }
    }
    Ok((intended_packs.len() > 1).then_some(BossPullUnit {
        boss,
        members,
        intended_packs,
    }))
}

pub(super) fn members(world: &World, target: Entity, victim: Entity) -> HashSet<Entity> {
    let mut result = first_wave(world, target, victim);
    if let Some(units) = world.get::<BossPullUnits>(victim) {
        loop {
            let before = result.len();
            for unit in &units.0 {
                if !result.is_disjoint(&unit.members) {
                    result.extend(
                        unit.members
                            .iter()
                            .copied()
                            .filter(|member| crate::spell_cast::targets::is_alive(world, *member)),
                    );
                }
            }
            if result.len() == before {
                break;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::components::Health;

    #[test]
    fn overlapping_boss_units_form_one_order_independent_pull() {
        let mut world = World::new();
        let mut spawn = || {
            world
                .spawn((
                    Health {
                        current: 100.0,
                        max: 100.0,
                    },
                    crate::creature_assistance::AssistanceCalled,
                ))
                .id()
        };
        let first = spawn();
        let bridge = spawn();
        let second = spawn();
        let outside = spawn();
        let victim = world
            .spawn(BossPullUnits(vec![
                BossPullUnit {
                    boss: second,
                    members: HashSet::from([bridge, second, outside]),
                    intended_packs: HashSet::from([2, 3]),
                },
                BossPullUnit {
                    boss: first,
                    members: HashSet::from([first, bridge]),
                    intended_packs: HashSet::from([1, 2]),
                },
            ]))
            .id();
        assert_eq!(
            members(&world, first, victim),
            HashSet::from([first, bridge, second, outside])
        );
        assert_eq!(
            members(&world, second, victim),
            members(&world, first, victim)
        );
        assert_eq!(merged_pack_count(&world, first, victim), 3);
    }
}
