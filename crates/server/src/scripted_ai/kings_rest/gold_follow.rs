//! Animated Gold's TC a352b1fa unconstrained, non-pet follow.
//! Real baked paths only; near-point collision-angle search remains unconverted.
use bevy::prelude::*;
use shared::components::{
    CreatureMotion, ModelDisplay, MovementSpeed, Npc, Position, Rotation, UnitScale,
};
use std::collections::HashMap;

use crate::maps::InMap;
use crate::navigation::{Goal, MoveRequest, Navigation, Step};
use crate::scripted_ai::{Hook, MotionType};

const FOLLOW_RANGE: f32 = 2.0;
const FOLLOW_RANGE_TOLERANCE: f32 = 1.0;
const CHECK_INTERVAL_MS: u32 = 100;
const BASE_WALK_SPEED: f32 = 2.5;

#[derive(Resource)]
pub(crate) struct GoldModelReaches(HashMap<u32, f32>);

pub(crate) fn load_model_reaches(
    conn: &rusqlite::Connection,
) -> rusqlite::Result<GoldModelReaches> {
    let mut statement = conn.prepare(
        "SELECT DisplayID,CombatReach FROM encounter_creature_model_info \
         WHERE DisplayID IN (11686,84202,88651)",
    )?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let reaches: HashMap<u32, f32> = rows.collect::<rusqlite::Result<_>>()?;
    if reaches.len() != 3 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(GoldModelReaches(reaches))
}

#[derive(Component, Clone)]
pub(crate) struct GoldFollow {
    target: Entity,
    check_ms: u32,
    last_target_position: Option<Position>,
    destination: Option<Position>,
}

pub(super) fn primary(world: &mut World, actor: Entity) -> Option<Entity> {
    let map = *world.get::<InMap>(actor)?;
    let candidates: Vec<_> = world
        .query::<(Entity, &Npc, &InMap)>()
        .iter(world)
        .filter_map(|(unit, npc, copy)| {
            (*copy == map && npc.template_id == super::golden_serpent::ENTRY).then_some(unit)
        })
        .collect();
    candidates.into_iter().find(|&unit| {
        crate::scripted_ai::instance_script::bound_boss(world, unit)
            .is_some_and(|boss| boss.boss_id == 0)
    })
}

pub(super) fn start(world: &mut World, actor: Entity, target: Entity) {
    crate::scripted_ai::movement::clear_point_movement(world, actor);
    crate::scripted_ai::movement::set_walk(world, actor, true);
    world.entity_mut(actor).insert(GoldFollow {
        target,
        check_ms: 0,
        last_target_position: None,
        destination: None,
    });
}

pub(super) fn clear(world: &mut World, actor: Entity) {
    world.entity_mut(actor).remove::<GoldFollow>();
}

fn combat_reach(world: &World, actor: Entity) -> Option<f32> {
    let model = world.get::<ModelDisplay>(actor)?;
    let reach = world
        .get_resource::<GoldModelReaches>()?
        .0
        .get(&model.display_id)?;
    Some(reach * world.get::<UnitScale>(actor).map_or(1.0, |scale| scale.0))
}

fn point(position: Position) -> Vec3 {
    Vec3::new(position.x, position.y, position.z)
}

struct FollowPositions {
    from: Position,
    target: Position,
    map: InMap,
    reach: f32,
}

fn follow_positions(world: &World, actor: Entity, target: Entity) -> Option<FollowPositions> {
    let map = *world.get::<InMap>(actor)?;
    if world.get::<InMap>(target) != Some(&map) {
        return None;
    }
    let reach = combat_reach(world, actor)? + combat_reach(world, target)? + FOLLOW_RANGE;
    Some(FollowPositions {
        from: *world.get::<Position>(actor)?,
        target: *world.get::<Position>(target)?,
        map,
        reach,
    })
}

fn near_point(from: Position, target: Position, reach: f32) -> Position {
    let delta = point(from) - point(target);
    let direction = Vec3::new(delta.x, 0.0, delta.z).normalize_or_zero();
    let near = point(target) + direction * reach;
    Position {
        x: near.x,
        y: near.y,
        z: near.z,
    }
}

fn update_destination(follow: &mut GoldFollow, positions: &FollowPositions) {
    if follow.last_target_position == Some(positions.target) {
        return;
    }
    follow.last_target_position = Some(positions.target);
    let distance = point(positions.from).distance(point(positions.target));
    if follow.destination.is_some() || distance >= positions.reach + FOLLOW_RANGE_TOLERANCE {
        follow.destination = Some(near_point(
            positions.from,
            positions.target,
            positions.reach,
        ));
    }
}

fn advance_path(
    world: &mut World,
    actor: Entity,
    follow: &GoldFollow,
    positions: &FollowPositions,
) -> bool {
    let Some(to) = follow.destination else {
        return false;
    };
    let speed = world
        .get::<crate::creature_ai::SpawnPoint>(actor)
        .expect("Gold spawn speeds")
        .speed_walk
        * BASE_WALK_SPEED;
    let distance = speed * world.resource::<Time>().delta_secs();
    let Some(mut navigation) = world.get_resource_mut::<Navigation>() else {
        return false;
    };
    let result = navigation.advance(
        actor,
        MoveRequest {
            map_id: positions.map.map_id,
            goal: Goal::Follow(follow.target.to_bits()),
            from: positions.from,
            to,
            distance,
            reach: 0.0,
        },
    );
    match result {
        Step::Moved { position, reached } => {
            replicate_step(world, actor, follow.target, position, speed);
            reached
        }
        Step::Unbaked | Step::Blocked => {
            world.entity_mut(actor).insert(CreatureMotion::Still);
            false
        }
    }
}

fn replicate_step(
    world: &mut World,
    actor: Entity,
    target: Entity,
    position: Position,
    speed: f32,
) {
    world
        .entity_mut(actor)
        .insert((position, MovementSpeed(speed), CreatureMotion::Walk));
    if let Some(rotation) = world.get::<Rotation>(target).copied() {
        world.entity_mut(actor).insert(rotation);
    }
    if let Some(queue) = world.get_resource::<lightyear::prelude::ReplicationDirtyQueue>() {
        queue.mark(actor);
    }
    if let Some(queue) = world.get_resource::<crate::interest::InterestDirtyQueue>() {
        crate::interest::mark_interest_position_changed(Some(queue), actor);
    }
}

fn update_follow(world: &mut World, actor: Entity, mut follow: GoldFollow, diff_ms: u32) {
    let Some(positions) = follow_positions(world, actor, follow.target) else {
        clear(world, actor);
        crate::scripted_ai::movement::clear_point_movement(world, actor);
        return;
    };
    follow.check_ms += diff_ms;
    let check = follow.check_ms >= CHECK_INTERVAL_MS;
    if check {
        follow.check_ms = 0;
    }
    let in_reach = point(positions.from).distance(point(positions.target)) < positions.reach;
    update_destination(&mut follow, &positions);
    if (check && in_reach) || advance_path(world, actor, &follow, &positions) {
        clear(world, actor);
        crate::scripted_ai::movement::clear_point_movement(world, actor);
        crate::scripted_ai::dispatch(world, actor, Hook::MovementInform(MotionType::Follow, 0));
    } else {
        world.entity_mut(actor).insert(follow);
    }
}

pub(crate) fn update_follows(world: &mut World) {
    let diff_ms = world.resource::<Time>().delta().as_millis() as u32;
    let movers: Vec<_> = world
        .query::<(Entity, &GoldFollow)>()
        .iter(world)
        .map(|(actor, follow)| (actor, follow.clone()))
        .collect();
    for (actor, follow) in movers {
        update_follow(world, actor, follow, diff_ms);
    }
}
