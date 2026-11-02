//! Release dead dungeon followers only after a complete native Follow route is proven.
use bevy::prelude::*;
use shared::components::{CombatStatus, Health, Mana, Position};

use super::{Follower, party};
use crate::maps::{InMap, MapStore, WorldLocation};
use crate::navigation::{Goal, Navigation};
use ::navigation::worker::{Output, Query};

/// A released follower must finish returning before the next pull.
#[derive(Component)]
pub(super) struct ReturningFromGraveyard;

#[derive(Component)]
pub(super) struct ResurrectionApproach;

pub(super) fn update(world: &mut World) {
    let approaching: Vec<_> = world
        .query_filtered::<Entity, With<ResurrectionApproach>>()
        .iter(world)
        .collect();
    for unit in approaching {
        world.entity_mut(unit).remove::<ResurrectionApproach>();
    }
    let returned: Vec<_> = world
        .query_filtered::<(Entity, &Follower, &Position), With<ReturningFromGraveyard>>()
        .iter(world)
        .filter_map(|(unit, follower, position)| {
            let owner = world.get::<Position>(follower.owner)?;
            crate::creature_ai::within_follow_range(*position, *owner).then_some(unit)
        })
        .collect();
    for unit in returned {
        world.entity_mut(unit).remove::<ReturningFromGraveyard>();
    }
    let returned_owners: Vec<_> = world
        .query_filtered::<(Entity, &Position), (
            With<ReturningFromGraveyard>,
            With<shared::components::Player>,
        )>()
        .iter(world)
        .filter_map(|(owner, position)| {
            let (_, members, _) = party::read_admission(world, owner)?;
            members
                .iter()
                .any(|member| {
                    crate::spell_cast::targets::is_alive(world, *member)
                        && world
                            .get::<Position>(*member)
                            .is_some_and(|member_position| {
                                crate::creature_ai::within_follow_range(*position, *member_position)
                            })
                })
                .then_some(owner)
        })
        .collect();
    for owner in returned_owners {
        world.entity_mut(owner).remove::<ReturningFromGraveyard>();
    }
    let owners: std::collections::BTreeSet<_> = world
        .query::<&Follower>()
        .iter(world)
        .map(|follower| follower.owner)
        .collect();
    for owner in owners {
        if world
            .get::<crate::death::PlayerDeathState>(owner)
            .is_some_and(|state| state.state == shared::death::DeathState::Dead)
        {
            request_resurrection(world, owner, owner);
        }
    }
    let dead: Vec<_> = world
        .query::<(Entity, &Follower, &crate::combat::Corpse)>()
        .iter(world)
        .map(|(unit, follower, _)| (unit, follower.owner))
        .collect();
    for (unit, owner) in dead {
        if crate::death::soulstone::use_follower_soulstone(world, unit) {
            continue;
        }
        if request_resurrection(world, unit, owner) {
            continue;
        }
        let (decision, reason, walk_seconds, error) = release_if_reachable(world, unit, owner);
        info!(follower = ?unit, owner = ?owner, decision, reason, walk_seconds = ?walk_seconds, error = ?error, "Follower release evaluation");
    }
}

fn request_resurrection(world: &mut World, target: Entity, owner: Entity) -> bool {
    if world
        .get::<crate::death::resurrection::PendingResurrection>(target)
        .is_some()
    {
        return true;
    }
    let Some((map, members, _)) = party::read_admission(world, owner) else {
        return false;
    };
    let members = members.to_vec();
    let boss_in_progress = crate::scripted_ai::instance_script::boss_in_progress(world, map);
    let in_combat = boss_in_progress
        || std::iter::once(owner)
            .chain(members.iter().copied())
            .any(|member| {
                world
                    .get::<CombatStatus>(member)
                    .is_some_and(|status| status.0)
            });
    if in_combat
        && (!boss_in_progress
            || crate::mythic_plus::combat_res_available(world, map) == Some(false))
    {
        return false;
    }
    let casters: Vec<_> = members
        .iter()
        .copied()
        .filter(|&unit| {
            unit != target
                && world.get::<super::AplControlled>(unit).is_some()
                && world.get::<InMap>(unit) == Some(&map)
                && crate::spell_cast::targets::is_alive(world, unit)
        })
        .collect();
    if casters.is_empty() {
        return false;
    }
    let store = world.resource::<crate::spell_info::SpellStore>();
    let candidates: Vec<_> = casters
        .into_iter()
        .flat_map(|unit| {
            world
                .get::<crate::class_progression::KnownSpells>(unit)
                .into_iter()
                .flat_map(move |known| {
                    known.iter().filter_map(move |spell| {
                        store
                            .get(spell)
                            .filter(|info| {
                                (!in_combat
                                    || info
                                        .misc
                                        .as_ref()
                                        .is_some_and(|misc| misc.attributes[0] & 0x1000_0000 == 0))
                                    && info
                                        .effects
                                        .iter()
                                        .any(crate::death::resurrection::is_resurrection)
                            })
                            .map(|_| (unit, spell))
                    })
                })
        })
        .collect();
    for (caster, spell) in candidates {
        match crate::spell_cast::creature_cast(world, caster, spell, Some(target), false) {
            Ok(())
            | Err(
                crate::spell_cast::CastFailure::InProgress
                | crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::Moving,
            ) => {
                world.entity_mut(caster).insert(ResurrectionApproach);
                return true;
            }
            Err(crate::spell_cast::CastFailure::OutOfRange) => {
                world.entity_mut(caster).insert((
                    ResurrectionApproach,
                    crate::creature_ai::FollowOwner::new(target),
                ));
                return true;
            }
            Err(error) => debug!(
                ?caster,
                ?target,
                spell,
                ?error,
                "Follower resurrection unavailable"
            ),
        }
    }
    false
}

type Evaluation = (&'static str, &'static str, Option<f32>, Option<String>);

fn wait(reason: &'static str) -> Evaluation {
    ("wait", reason, None, None)
}

fn release_if_reachable(world: &mut World, unit: Entity, owner: Entity) -> Evaluation {
    let Some((map, members, _)) = party::read_admission(world, owner) else {
        return wait("no_admitted_party");
    };
    if !members.contains(&unit) || world.get::<InMap>(unit) != Some(&map) {
        return wait("not_in_admitted_copy");
    }
    let in_combat = std::iter::once(owner)
        .chain(members.iter().copied())
        .any(|member| {
            world
                .get::<CombatStatus>(member)
                .is_some_and(|status| status.0)
        });
    if in_combat || crate::scripted_ai::instance_script::boss_in_progress(world, map) {
        if let Some(mut nav) = world.get_resource_mut::<Navigation>() {
            nav.cancel_query(unit, Goal::FollowerRelease);
        }
        return wait("party_in_combat");
    }
    let Some(destination) = release_location(world, unit, map) else {
        return wait("no_release_location");
    };
    if destination.map_id != map.map_id {
        return wait("release_outside_instance");
    }
    let Some(owner_position) = world.get::<Position>(owner).copied() else {
        return wait("owner_position_missing");
    };
    let start = destination.bevy_position();
    let point = |position: Position| Vec3::new(position.x, position.y, position.z);
    let Some(mut nav) = world.get_resource_mut::<Navigation>() else {
        return wait("navigation_unavailable");
    };
    let result = nav.query_deferred(
        unit,
        map.map_id,
        Goal::FollowerRelease,
        Query::CompleteRoute {
            from: point(start),
            to: point(owner_position),
        },
    );
    let route = match result {
        None => return wait("route_pending"),
        Some(Err(error)) => return ("wait", "route_error", None, Some(error)),
        Some(Ok(None)) => return wait("navigation_unbaked"),
        Some(Ok(Some(Output::CompleteRoute(route)))) => route,
        Some(Ok(Some(_))) => return wait("unexpected_route_result"),
    };
    if !route.complete {
        return ("wait", "route_incomplete", None, route.reason);
    }
    let speed = crate::creature_ai::BASE_RUN_SPEED
        * world
            .get::<crate::auras::AuraStatMods>(unit)
            .map_or(1.0, |mods| mods.movement_speed);
    if speed <= 0.0 {
        return wait("run_speed_zero");
    }
    let walk_seconds = route.distance / speed;
    world
        .resource_mut::<Navigation>()
        .cancel_query(unit, Goal::FollowerRelease);
    revive_at_release(world, unit, owner, &destination);
    ("release", "route_reachable", Some(walk_seconds), None)
}

fn release_location(world: &World, unit: Entity, map: InMap) -> Option<WorldLocation> {
    let position = world.get::<Position>(unit)?;
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    let location = WorldLocation {
        map_id: map.map_id,
        position: Vec3::new(x, y, z),
        orientation: 0.0,
    };
    let maps = world.get_resource::<MapStore>()?;
    crate::death::dungeon_release_location(&location, maps.map(map.map_id)?, maps)
}

fn revive_at_release(world: &mut World, unit: Entity, owner: Entity, destination: &WorldLocation) {
    crate::maps::transfer::teleport_creature_with_companions(world, unit, destination);
    crate::death::soulstone::clear_available(world, unit);
    if let Some(mut health) = world.get_mut::<Health>(unit) {
        health.current = (health.max * crate::death::GHOST_RESURRECT_HEALTH_PCT).max(1.0);
    }
    if let Some(mut mana) = world.get_mut::<Mana>(unit) {
        mana.current = mana.max * crate::death::GHOST_RESURRECT_MANA_PCT;
    }
    world
        .entity_mut(unit)
        .remove::<(crate::combat::DeathState, crate::combat::Corpse)>()
        .insert((
            crate::creature_ai::CreatureState::Idle,
            crate::creature_ai::FollowOwner::new(owner),
            ReturningFromGraveyard,
        ));
    crate::spell_cast::mark_dirty(world, unit);
}
