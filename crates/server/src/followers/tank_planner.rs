//! Shared planner owns candidate collection and movement, never interrupt casts.
use super::Follower;
use bevy::prelude::*;

/// The planner's admitted movement destination; kits must not replace it.
#[derive(Component, Clone, Copy)]
pub(super) struct AdmittedPosition(pub Vec3);

#[derive(Component)]
struct AppliedMovementAt(f64);

/// A budget-admitted existing-fight caster, consumed by ordinary native chase.
#[derive(Component)]
struct AdmittedCasterTarget(Entity);

pub(super) fn update_caster_goal(
    world: &mut World,
    tank: Entity,
    follower: &Follower,
    party: &[Entity],
) -> Option<Entity> {
    let Some(caster) = read_admitted_caster(world, tank, follower, party) else {
        clear_caster_goal(world, tank);
        return None;
    };
    world.entity_mut(tank).insert(AdmittedCasterTarget(caster));
    apply_admitted_caster_target(world, tank);
    Some(caster)
}

fn read_admitted_caster(
    world: &mut World,
    tank: Entity,
    follower: &Follower,
    party: &[Entity],
) -> Option<Entity> {
    if follower.profile.role != shared::group::GroupRole::Tank
        || world.get::<AdmittedPosition>(tank).is_some()
    {
        return None;
    }
    for caster in read_engaged_caster_candidates(world, tank, follower, party) {
        if super::hazards::target_is_hazardous(world, tank, caster) {
            continue;
        }
        match read_movement_admission(world, tank, caster, follower, None) {
            Ok(Some(true)) => return Some(caster),
            Ok(None) => break,
            Ok(Some(false)) => debug!(?tank, ?caster, "Caster collection held by pull budget"),
            Err(error) => debug!(?tank, ?caster, %error, "Caster collection admission unavailable"),
        }
    }
    None
}

fn clear_caster_goal(world: &mut World, tank: Entity) {
    let Some(AdmittedCasterTarget(caster)) = world.entity_mut(tank).take::<AdmittedCasterTarget>()
    else {
        return;
    };
    let pursuing = crate::creature_ai::CreatureState::Pursuing {
        target: caster.to_bits(),
    };
    if world.get::<crate::creature_ai::CreatureState>(tank) == Some(&pursuing) {
        world
            .entity_mut(tank)
            .insert(crate::creature_ai::CreatureState::Idle);
        if let Some(position) = world.get::<shared::components::Position>(tank).copied() {
            crate::creature_ai::move_follower(world, tank, position, 0.0);
        }
    }
}

pub(super) fn apply_admitted_caster_target(world: &mut World, tank: Entity) -> bool {
    let Some(&AdmittedCasterTarget(caster)) = world.get::<AdmittedCasterTarget>(tank) else {
        return false;
    };
    world
        .entity_mut(tank)
        .remove::<crate::creature_ai::FollowOwner>()
        .insert((
            crate::creature_ai::CreatureState::Pursuing {
                target: caster.to_bits(),
            },
            super::ai::FollowerReach(super::ai::MELEE_REACH),
            crate::networking::CombatTarget(Some(caster)),
        ));
    true
}

/// Refusal lives at the kit's final pursuit boundary, not an overwritten early guard.
/// In-place attacks still use the ordinary native cast/range/GCD checks.
pub(super) fn hold_unbudgeted_pursuit(world: &mut World, tank: Entity, target: Entity) -> bool {
    let Some(follower) = world.get::<Follower>(tank).cloned() else {
        return false;
    };
    if follower.profile.role != shared::group::GroupRole::Tank
        || !crate::spell_cast::targets::is_hostile(world, tank, target)
    {
        return false;
    }
    let owns_run = world
        .get::<super::party::FollowerRun>(follower.owner)
        .is_some()
        || world
            .get::<super::stockade_diagnostic::DiagnosticParty>(follower.owner)
            .is_some();
    if !owns_run {
        return false;
    }
    // Existing but incomplete run records still go through fail-closed admission.
    match read_movement_admission(world, tank, target, &follower, None) {
        Ok(Some(true)) => return false,
        Ok(None) => debug!(
            ?tank,
            ?target,
            "Tank pursuit awaits published admission route"
        ),
        Ok(Some(false)) => debug!(?tank, ?target, "Tank pursuit held by pull budget"),
        Err(error) => debug!(?tank, ?target, %error, "Tank pursuit admission unavailable"),
    }
    world
        .entity_mut(tank)
        .remove::<crate::creature_ai::FollowOwner>()
        .insert((
            crate::creature_ai::CreatureState::Idle,
            crate::networking::CombatTarget(Some(target)),
        ));
    if let Some(position) = world.get::<shared::components::Position>(tank).copied() {
        crate::creature_ai::move_follower(world, tank, position, 0.0);
    }
    true
}

pub(super) fn read_movement_admission(
    world: &mut World,
    tank: Entity,
    target: Entity,
    follower: &Follower,
    destination: Option<Vec3>,
) -> Result<Option<bool>, String> {
    let map = *world
        .get::<crate::maps::InMap>(tank)
        .ok_or("Tank movement lacks a map copy")?;
    let (admitted_map, members, _) = super::party::read_admission(world, follower.owner)
        .ok_or("Tank movement lacks an authoritative admitted party")?;
    if admitted_map != map || !members.contains(&tank) {
        return Err("Tank movement is outside its admitted copy or roster".into());
    }
    let mut party = members.to_vec();
    party.push(follower.owner);
    let history = super::pull_history::snapshot(world, follower.owner)
        .ok_or("Tank movement lacks current authoritative run history")?;
    let difficulty = world
        .get::<crate::zones::CreatureDifficulty>(target)
        .ok_or("Tank movement target lacks native creature difficulty")?
        .0;
    let difficulty = super::population_difficulty(difficulty);
    let target_guid = world
        .get::<crate::zones::CreatureOrigin>(target)
        .ok_or("Tank movement target lacks authored source identity")?
        .guid;
    let request = super::pull_route::Request {
        tank,
        target_guid,
        difficulty,
        cleared: &history.cleared_spawns,
    };
    let Some(route) = (match destination {
        Some(point) => super::pull_route::read_point_route(world, &request, point)?,
        None => super::pull_route::read_route(world, &request)?,
    }) else {
        return Ok(None);
    };
    if super::hazards::route_crosses_hostile_field(world, tank, &route.points) {
        return Ok(Some(false));
    }
    debug!(
        ?tank,
        corners = route.points.len(),
        packs = route.pack_count,
        "Tank movement route projected"
    );
    let sources =
        super::pull_sources::read_present_sources(world, map, difficulty, &history.cleared_spawns)?;
    let engaged = super::ai::engaged_enemies(world, follower, tank, &party);
    let mut engaged_packs = std::collections::BTreeSet::new();
    for (enemy, _) in engaged {
        let origin = world
            .get::<crate::zones::CreatureOrigin>(enemy)
            .ok_or("Engaged enemy lacks authored source identity")?;
        let source = sources
            .get(&origin.guid)
            .ok_or("Engaged source is not present")?;
        engaged_packs.insert(source.pack_guid);
    }
    // Moving only within already-engaged formations is defense, not another pull.
    if route.spawns.iter().all(|guid| {
        sources
            .get(guid)
            .is_some_and(|source| engaged_packs.contains(&source.pack_guid))
    }) {
        return Ok(Some(true));
    }
    Ok(Some(matches!(
        super::pull_admission::read_pull_budget(world, tank, &route.spawns, difficulty)?,
        super::pull_budget::Decision::Allow { .. }
    )))
}

/// Consume the same admitted point at both kit pursuit and idle regroup boundaries.
pub(super) fn apply_admitted_position(
    world: &mut World,
    unit: Entity,
    target: Option<Entity>,
) -> bool {
    let Some(goal) = world.get::<AdmittedPosition>(unit).copied() else {
        return false;
    };
    world
        .entity_mut(unit)
        .remove::<crate::creature_ai::FollowOwner>()
        .insert((
            crate::creature_ai::CreatureState::Idle,
            crate::networking::CombatTarget(target),
        ));
    let now = world.resource::<Time>().elapsed_secs_f64();
    if world
        .get::<AppliedMovementAt>(unit)
        .is_some_and(|at| at.0 == now)
    {
        return true;
    }
    let Some(from) = world.get::<shared::components::Position>(unit).copied() else {
        warn!(?unit, "Admitted tank movement lacks a position");
        return true;
    };
    crate::creature_ai::move_follower(world, unit, from, 0.0);
    let here = Vec3::new(from.x, from.y, from.z);
    if goal.0 == here {
        return true;
    }
    if let Err(error) = advance_admitted_once(world, unit, from, goal.0) {
        warn!(?unit, %error, "Admitted tank movement held");
    }
    true
}

fn advance_admitted_once(
    world: &mut World,
    unit: Entity,
    from: shared::components::Position,
    goal: Vec3,
) -> Result<(), String> {
    let now = world.resource::<Time>().elapsed_secs_f64();
    let casting = world
        .get::<shared::casting::CastState>(unit)
        .is_some_and(|cast| !cast.cast_while_moving);
    if casting {
        return Ok(());
    }
    if !goal.is_finite() {
        return Err("admitted destination is non-finite".into());
    }
    let map = *world
        .get::<crate::maps::InMap>(unit)
        .ok_or("admitted movement lacks map")?;
    if world
        .get_resource::<crate::navigation::Navigation>()
        .is_none()
        || world.get_resource::<crate::ground::GroundMap>().is_none()
    {
        return Err("admitted movement requires authored navigation and ground".into());
    }
    let to = shared::components::Position {
        x: goal.x,
        y: goal.y,
        z: goal.z,
    };
    match super::hazards::point_route_crosses_hostile_field(world, unit, to) {
        Some(true) => return Err("admitted route crosses an active hostile field".into()),
        None => return Ok(()),
        Some(false) => {}
    }
    world.entity_mut(unit).insert(AppliedMovementAt(now));
    super::hazards::advance_grounded_route(
        world,
        unit,
        map,
        from,
        to,
        crate::navigation::Goal::Point,
    );
    Ok(())
}

pub(super) fn read_engaged_caster_candidates(
    world: &mut World,
    unit: Entity,
    follower: &Follower,
    party: &[Entity],
) -> Vec<Entity> {
    super::ai::engaged_enemies(world, follower, unit, party)
        .into_iter()
        .filter(|(enemy, _)| {
            let interruptible = world
                .get::<shared::casting::CastState>(*enemy)
                .is_some_and(|cast| cast.interruptible);
            let protected = crate::auras::has_breakable_by_damage_cc(world, *enemy);
            interruptible && !protected
        })
        .map(|(enemy, _)| enemy)
        .collect()
}
