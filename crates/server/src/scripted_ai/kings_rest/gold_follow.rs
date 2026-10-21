//! Animated Gold's TC a352b1fa unconstrained, non-pet follow.
//! Real baked paths only; near-point collision-angle search remains unconverted.
use bevy::prelude::*;
use shared::components::{CreatureMotion, MovementSpeed, Npc, Position, Rotation};

use crate::maps::InMap;
use crate::navigation::{Goal, MoveRequest, Navigation, Step};
use crate::scripted_ai::model_reach::{ModelReaches, combat_reach};
use crate::scripted_ai::{Hook, MotionType};

const FOLLOW_RANGE: f32 = 2.0;
const FOLLOW_RANGE_TOLERANCE: f32 = 1.0;
const CHECK_INTERVAL_MS: u32 = 100;
const BASE_WALK_SPEED: f32 = 2.5;

/// Serpent84202, Animated Gold88651 and invisible11686 (TDB1210 model info).
const GOLD_DISPLAYS: [u32; 3] = [11686, 84202, 88651];

/// The prepared model reaches, requiring the Kings' Rest follow displays.
pub(crate) fn load_model_reaches(conn: &rusqlite::Connection) -> rusqlite::Result<ModelReaches> {
    let reaches = crate::scripted_ai::model_reach::load_model_reaches(conn)?;
    if !GOLD_DISPLAYS
        .iter()
        .all(|display| reaches.source.contains_key(display))
    {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(reaches)
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
    let angle = delta.z.atan2(delta.x);
    let direction = Vec3::new(angle.cos(), 0.0, angle.sin());
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
    apply_path_step(world, actor, follow.target, speed, result)
}

fn apply_path_step(
    world: &mut World,
    actor: Entity,
    target: Entity,
    speed: f32,
    step: Step,
) -> bool {
    match step {
        Step::Moved { position, reached } => {
            replicate_step(world, actor, target, position, speed);
            reached
        }
        Step::Pending => false,
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

fn movement_prevented(world: &World, actor: Entity) -> bool {
    let immobile = world
        .get::<crate::auras::ActiveAuras>(actor)
        .is_some_and(|auras| {
            auras.is_stunned()
                || auras.has_type(crate::auras::handlers::MOD_ROOT)
                || auras.has_type(crate::auras::handlers::MOD_ROOT_2)
        });
    let casting = world
        .get::<shared::casting::CastState>(actor)
        .is_some_and(|cast| !cast.cast_while_moving);
    immobile || casting
}

fn pause_follow(world: &mut World, actor: Entity, mut follow: GoldFollow) {
    follow.last_target_position = None;
    follow.destination = None;
    crate::scripted_ai::movement::clear_point_movement(world, actor);
    world.entity_mut(actor).insert(follow);
}

fn position_check_due(follow: &mut GoldFollow, diff_ms: u32) -> bool {
    follow.check_ms += diff_ms;
    let due = follow.check_ms >= CHECK_INTERVAL_MS;
    if due {
        follow.check_ms = 0;
    }
    due
}

fn update_follow(world: &mut World, actor: Entity, mut follow: GoldFollow, diff_ms: u32) {
    let Some(positions) = follow_positions(world, actor, follow.target) else {
        clear(world, actor);
        crate::scripted_ai::movement::clear_point_movement(world, actor);
        return;
    };
    if movement_prevented(world, actor) {
        pause_follow(world, actor, follow);
        return;
    }
    let check = position_check_due(&mut follow, diff_ms);
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
