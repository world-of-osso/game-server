//! Shared planner owns candidate collection and movement, never interrupt casts.
use super::Follower;
use bevy::prelude::*;

/// The planner's admitted movement destination; kits must not replace it.
#[derive(Component, Clone, Copy)]
pub(super) struct AdmittedPosition(pub Vec3);

#[derive(Component)]
struct AppliedMovementAt(f64);

/// The last hold reported for a tank; a persistent hold repeats once per interval.
#[derive(Component)]
struct ReportedHold {
    reason: &'static str,
    target: Option<Entity>,
    at: f64,
}

const HOLD_REPORT_INTERVAL_SECS: f64 = 10.0;

/// Report why the tank is held, at info level: on every new reason or target, then
/// once per interval while the same hold lasts. `detail` runs only when it logs.
pub(super) fn report_hold(
    world: &mut World,
    tank: Entity,
    target: Option<Entity>,
    reason: &'static str,
    detail: impl FnOnce(&World) -> Option<String>,
) {
    let now = world.resource::<Time>().elapsed_secs_f64();
    if world.get::<ReportedHold>(tank).is_some_and(|last| {
        last.reason == reason && last.target == target && now - last.at < HOLD_REPORT_INTERVAL_SECS
    }) {
        return;
    }
    let position = world.get::<shared::components::Position>(tank).copied();
    let detail = detail(world);
    info!(?tank, ?target, ?position, reason, detail, "Tank held");
    world.entity_mut(tank).insert(ReportedHold {
        reason,
        target,
        at: now,
    });
}

/// A budget-admitted existing-fight caster, consumed by ordinary native chase.
#[derive(Component)]
struct AdmittedCasterTarget(Entity);

pub(super) fn update_caster_goal(
    world: &mut World,
    tank: Entity,
    follower: &Follower,
    party: &[Entity],
    enemies: &[(Entity, Option<Entity>)],
) -> Option<Entity> {
    let Some(caster) = read_admitted_caster(world, tank, follower, party, enemies) else {
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
    enemies: &[(Entity, Option<Entity>)],
) -> Option<Entity> {
    if follower.profile.role != shared::group::GroupRole::Tank
        || world.get::<AdmittedPosition>(tank).is_some()
    {
        return None;
    }
    for caster in read_engaged_caster_candidates(world, tank, follower, party) {
        if !enemies.iter().any(|(enemy, _)| *enemy == caster)
            || super::hazards::target_is_hazardous(world, tank, caster)
        {
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
    let (reason, error) = match read_route_admission(world, tank, target, &follower, None, true) {
        Ok(Some((true, targets))) => {
            if targets.is_empty() {
                world
                    .entity_mut(follower.owner)
                    .remove::<super::AdmittedPull>();
            } else {
                world
                    .entity_mut(follower.owner)
                    .insert(super::AdmittedPull(targets));
            }
            return false;
        }
        Ok(None) => ("pursuit awaits published admission route", None),
        Ok(Some((false, _))) => ("pursuit held by pull budget", None),
        Err(error) => ("pursuit admission unavailable", Some(error)),
    };
    world
        .entity_mut(follower.owner)
        .remove::<super::AdmittedPull>();
    report_hold(world, tank, Some(target), reason, |_| error);
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
    Ok(
        read_route_admission(world, tank, target, follower, destination, false)?
            .map(|(allowed, _)| allowed),
    )
}

fn read_route_admission(
    world: &mut World,
    tank: Entity,
    target: Entity,
    follower: &Follower,
    destination: Option<Vec3>,
    pursuit: bool,
) -> Result<Option<(bool, Vec<Entity>)>, String> {
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
        None if pursuit => super::pull_route::read_pursuit_route(world, &request)?,
        None => super::pull_route::read_route(world, &request)?,
    }) else {
        return Ok(None);
    };
    if super::hazards::route_crosses_hostile_field(world, tank, &route.points)
        || super::positioning::route_enters_unpulled_sight(world, tank, target, &route.points)?
    {
        return Ok(Some((false, Vec::new())));
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
    let projected = super::pull_admission::expand_assistance(world, tank, &route.spawns, &sources)?;
    // Moving only within already-engaged formations is defense, not another pull.
    if projected.iter().all(|guid| {
        sources
            .get(guid)
            .is_some_and(|source| engaged_packs.contains(&source.pack_guid))
    }) {
        return Ok(Some((true, Vec::new())));
    }
    let allowed = matches!(
        super::pull_admission::read_pull_budget(world, tank, &projected, difficulty)?,
        super::pull_budget::Decision::Allow { .. }
    );
    let mut targets = vec![target];
    if allowed {
        for guid in projected {
            let actor = sources[&guid].actor.ok_or_else(|| {
                format!("Admitted pull source GUID {guid} lacks its native actor")
            })?;
            if actor != target {
                targets.push(actor);
            }
        }
    }
    Ok(Some((allowed, targets)))
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
            let interruptible = crate::spell_cast::can_interrupt_cast(world, unit, *enemy);
            let protected = crate::auras::has_breakable_by_damage_cc(world, *enemy);
            interruptible && !protected
        })
        .map(|(enemy, _)| enemy)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::followers::hazard_reaction_tests::TraceBuffer;
    use std::time::Duration;

    #[test]
    fn tank_hold_reports_at_info_on_change_and_once_per_interval() {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        let tank = world
            .spawn(shared::components::Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            })
            .id();
        let target = world.spawn_empty().id();
        let buffer = TraceBuffer::default();
        let subscriber = bevy::log::tracing_subscriber::fmt()
            .with_max_level(bevy::log::tracing::Level::INFO)
            .with_writer(buffer.clone())
            .without_time()
            .with_ansi(false)
            .finish();
        crate::test_support::tracing_capture::with_default(subscriber, || {
            let tick = |world: &mut World, millis: u64, reason| {
                world
                    .resource_mut::<Time>()
                    .advance_by(Duration::from_millis(millis));
                report_hold(world, tank, Some(target), reason, |_| {
                    Some("watcher (0, 0, 0) r=20".into())
                });
            };
            // 20 s of one hold at 20 ticks/s, then a new reason.
            tick(&mut world, 0, "pull-back has no admitted safe point");
            for _ in 0..400 {
                tick(&mut world, 50, "pull-back has no admitted safe point");
            }
            tick(&mut world, 50, "pursuit held by pull budget");
        });
        let text = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
        let held: Vec<_> = text
            .lines()
            .filter(|line| line.contains("Tank held"))
            .collect();
        assert_eq!(held.len(), 4, "{text}");
        assert!(held.iter().all(|line| line.contains("INFO")), "{text}");
        assert_eq!(
            held.iter()
                .filter(|line| line.contains("no admitted safe point"))
                .count(),
            3
        );
        assert!(held[3].contains("pursuit held by pull budget"));
        assert!(held[0].contains("x: 1.0") && held[0].contains("watcher (0, 0, 0) r=20"));
    }
}
