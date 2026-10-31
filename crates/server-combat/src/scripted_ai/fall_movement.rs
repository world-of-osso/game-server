//! TC a352b1fa MotionMaster1001–1046 and MoveSpline114–127: creature MoveFall.
//! Represented subset: static dry map floor, no transport or hover movement flag.
use bevy::prelude::*;
use shared::components::Position;

use super::{Hook, MotionType};
use crate::combat::DeathState;
use crate::ground::{GroundMap, GroundProbe};
use crate::maps::InMap;

const TERMINAL_SPEED: f32 = 60.148_003;
const GRAVITY: f32 = shared::skyriding::GRAVITY;

#[derive(Component)]
pub struct FallMovement {
    id: u32,
    from: Position,
    floor: f32,
    horizontal_target: Option<Vec3>,
    horizontal_duration: f32,
    elapsed_ms: u32,
    duration_ms: u32,
    started: std::time::Duration,
}

pub(crate) fn move_fall(world: &mut World, unit: Entity, id: u32) {
    start_fall(world, unit, id, None);
}

/// Begin gravity at the supported edge while moving horizontally into the fall column.
pub(crate) fn move_fall_towards(world: &mut World, unit: Entity, id: u32, landing: Vec3) {
    start_fall(world, unit, id, Some(landing));
}

fn start_fall(world: &mut World, unit: Entity, id: u32, landing: Option<Vec3>) {
    let Some((position, map)) = world
        .get::<Position>(unit)
        .copied()
        .zip(world.get::<InMap>(unit).copied())
    else {
        warn!(?unit, "MoveFall requires position and map");
        return;
    };
    if world
        .get::<crate::auras::ActiveAuras>(unit)
        .is_some_and(|auras| auras.is_stunned() || auras.has_type(crate::auras::handlers::MOD_ROOT))
    {
        return;
    }
    let search = landing.map_or(position, |target| Position {
        x: target.x,
        y: position.y,
        z: target.z,
    });
    let Some(floor) = query_fall_floor(world, unit, map, search) else {
        return;
    };
    if landing.is_some_and(|target| (target.y - floor).abs() > ::navigation::CELL_HEIGHT) {
        warn!(
            ?unit,
            floor,
            ?landing,
            "MoveFall target does not match collision floor"
        );
        return;
    }
    if (position.y - floor).abs() < 0.1 {
        return;
    }
    let duration_ms = (fall_seconds(position.y - floor) * 1000.0) as u32;
    if duration_ms == 0 {
        return;
    }
    let started = world.resource::<Time>().elapsed();
    let horizontal_duration = landing.map_or(0.0, |target| {
        Vec2::new(target.x - position.x, target.z - position.z).length()
            / crate::creature_ai::BASE_RUN_SPEED
    });
    super::movement::clear_point_movement(world, unit);
    world.entity_mut(unit).insert(FallMovement {
        id,
        from: position,
        floor,
        horizontal_target: landing,
        horizontal_duration,
        elapsed_ms: 0,
        duration_ms,
        started,
    });
}

fn query_fall_floor(
    world: &mut World,
    unit: Entity,
    map: InMap,
    position: Position,
) -> Option<f32> {
    let Some(mut ground) = world.get_resource_mut::<GroundMap>() else {
        warn!(?unit, "MoveFall requires map collision");
        return None;
    };
    let GroundProbe::Supported(floor) =
        ground.creature_fall_ground_in(map, Vec3::new(position.x, position.y, position.z))
    else {
        warn!(?unit, "MoveFall has no supported floor");
        return None;
    };
    Some(floor)
}

fn fall_seconds(drop: f32) -> f32 {
    if drop < 0.0 {
        return 0.0;
    }
    let terminal_time = TERMINAL_SPEED / GRAVITY;
    let terminal_drop = TERMINAL_SPEED * TERMINAL_SPEED / (2.0 * GRAVITY);
    if drop >= terminal_drop {
        (drop - terminal_drop) / TERMINAL_SPEED + terminal_time
    } else {
        (2.0 * drop / GRAVITY).sqrt()
    }
}

fn fall_elevation(seconds: f32) -> f32 {
    let terminal_time = TERMINAL_SPEED / GRAVITY;
    if seconds > terminal_time {
        TERMINAL_SPEED * (seconds - terminal_time) + GRAVITY * terminal_time * terminal_time * 0.5
    } else {
        seconds * (seconds * GRAVITY * 0.5)
    }
}

pub(crate) fn release_finished_falls(
    mut commands: Commands,
    falls: Query<(Entity, &FallMovement, Has<DeathState>)>,
) {
    for (unit, fall, dead) in &falls {
        if dead || fall.elapsed_ms == fall.duration_ms {
            commands.entity(unit).remove::<FallMovement>();
        }
    }
}

pub(crate) fn advance_falls(world: &mut World) {
    let now = world.resource::<Time>().elapsed();
    let delta_ms = world.resource::<Time>().delta().as_millis() as u32;
    for (unit, arrival) in update_fall_positions(world, now, delta_ms) {
        crate::interest::mark_interest_position_changed(
            world.get_resource::<crate::interest::InterestDirtyQueue>(),
            unit,
        );
        if let Some(queue) = world.get_resource::<lightyear::prelude::ReplicationDirtyQueue>() {
            queue.mark(unit);
        }
        if let Some(id) = arrival {
            super::dispatch(world, unit, Hook::MovementInform(MotionType::Effect, id));
        }
    }
}

fn update_fall_positions(
    world: &mut World,
    now: std::time::Duration,
    delta_ms: u32,
) -> Vec<(Entity, Option<u32>)> {
    let mut updates = Vec::new();
    let mut query =
        world.query_filtered::<(Entity, &mut FallMovement, &mut Position), Without<DeathState>>();
    for (unit, mut fall, mut position) in query.iter_mut(world) {
        // Launch initializes at t=0, never reuses the point-arrival tick's delta.
        if fall.started == now || fall.elapsed_ms == fall.duration_ms {
            continue;
        }
        fall.elapsed_ms = (fall.elapsed_ms + delta_ms).min(fall.duration_ms);
        let arrived = fall.elapsed_ms == fall.duration_ms;
        let seconds = fall.elapsed_ms as f32 / 1000.0;
        let fraction = if fall.horizontal_duration > 0.0 {
            (seconds / fall.horizontal_duration).min(1.0)
        } else {
            0.0
        };
        let target =
            fall.horizontal_target
                .unwrap_or(Vec3::new(fall.from.x, fall.from.y, fall.from.z));
        *position = Position {
            x: fall.from.x + (target.x - fall.from.x) * fraction,
            // TC computes elevation even when Done; integer-ms truncation can
            // leave the last sample slightly above the floor. Never force a snap.
            y: (fall.from.y - fall_elevation(seconds)).max(fall.floor),
            z: fall.from.z + (target.z - fall.from.z) * fraction,
        };
        updates.push((unit, arrived.then_some(fall.id)));
    }
    updates
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    #[ignore = "requires FOLLOWER_COMPONENT_ASSETS local Floodgate geometry"]
    fn move_fall_reaches_floodgate_floor_from_over_fifty_yards() {
        let assets =
            std::path::PathBuf::from(std::env::var_os("FOLLOWER_COMPONENT_ASSETS").unwrap());
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(GroundMap::new(&assets, HashMap::from([(2773, 5_839_271)])));
        app.update();
        let feet = Vec3::new(1524.0, 334.6686, 2746.0);
        let map = InMap::continent(2773);
        assert!(matches!(
            app.world_mut()
                .resource_mut::<GroundMap>()
                .creature_ground_in(map, feet),
            GroundProbe::Unsupported
        ));
        let unit = app
            .world_mut()
            .spawn((
                Position {
                    x: feet.x,
                    y: feet.y,
                    z: feet.z,
                },
                map,
            ))
            .id();
        move_fall(app.world_mut(), unit, 0);
        assert!(app.world().get::<FallMovement>(unit).is_some());
        for _ in 0..100 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(50));
            advance_falls(app.world_mut());
        }
        let landed = app.world().get::<Position>(unit).unwrap();
        assert!(
            (landed.y - 275.26523).abs() < 0.2,
            "actual landing {landed:?}"
        );
    }

    #[test]
    fn move_fall_finds_collision_floor_more_than_fifty_yards_below() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("los/777/m2");
        std::fs::create_dir_all(&dir).unwrap();
        let vertices = vec![
            Vec3::new(10.0, 0.0, 10.0),
            Vec3::new(10.0, 0.0, 30.0),
            Vec3::new(30.0, 0.0, 30.0),
            Vec3::new(30.0, 0.0, 10.0),
        ];
        let tile = shared::los::LosTile::build(32, 31, vertices, vec![[0, 1, 2], [0, 2, 3]]);
        std::fs::write(dir.join("32_31.los"), tile.to_bytes()).unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(GroundMap::new(root.path(), HashMap::new()));
        app.update();
        let feet = Vec3::new(20.0, 100.0, 20.0);
        let map = InMap::continent(777);
        assert!(
            !matches!(
                app.world_mut()
                    .resource_mut::<GroundMap>()
                    .creature_ground_in(map, feet),
                GroundProbe::Supported(_)
            ),
            "ordinary ground search remains bounded"
        );
        let unit = app
            .world_mut()
            .spawn((
                Position {
                    x: feet.x,
                    y: feet.y,
                    z: feet.z,
                },
                map,
            ))
            .id();
        move_fall(app.world_mut(), unit, 0);
        assert!(
            app.world().get::<FallMovement>(unit).is_some(),
            "fall must resolve the distant floor"
        );
        for _ in 0..120 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(50));
            advance_falls(app.world_mut());
        }
        let landed = app.world().get::<Position>(unit).unwrap();
        assert!(landed.y.abs() < 0.1, "actual landing {landed:?}");
    }
}
