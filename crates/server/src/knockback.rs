//! Server-authoritative ballistic movement, reusing MovementControl ownership.
//! TC a352b1fa SpellEffects.cpp:4037 / Unit.cpp:12565; source gravity in shared::skyriding.
use crate::{
    ground::{GroundMap, GroundProbe},
    maps::InMap,
};
use bevy::prelude::*;
use shared::components::{
    Health, MovementControl, MovementSpeed, PlayerMotion, Position, Rotation,
};

#[derive(Component, Clone)]
pub(crate) struct KnockbackFlight {
    origin: Vec3,
    velocity: Vec3,
    elapsed: f32,
    peak: f32,
    map: InMap,
    epoch: u32,
}

impl KnockbackFlight {
    fn at(&self, elapsed: f32) -> Vec3 {
        self.origin + self.velocity * elapsed
            - Vec3::Y * (0.5 * shared::skyriding::GRAVITY * elapsed * elapsed)
    }
    fn vertical_speed(&self) -> f32 {
        self.velocity.y - shared::skyriding::GRAVITY * self.elapsed
    }
}

pub(crate) struct KnockbackPlugin;
impl Plugin for KnockbackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            update_knockbacks
                .after(crate::networking::process_player_inputs)
                .before(crate::networking::apply_terrain_gravity),
        );
    }
}

/// The spell handler owns immunity checks; scripted pipe pushes call this directly.
pub(crate) fn launch(
    world: &mut World,
    target: Entity,
    origin: Vec3,
    speed_xy: f32,
    speed_z: f32,
) -> Result<(), String> {
    let position = *world
        .get::<Position>(target)
        .ok_or("knockback target has no Position")?;
    let start = Vec3::new(position.x, position.y, position.z);
    let velocity = launch_velocity(world, target, start, origin, speed_xy, speed_z)?;
    launch_directed(world, target, velocity)
}

/// An explicit world-space impulse, including EffectPullTowards' angle override.
pub(crate) fn launch_directed(
    world: &mut World,
    target: Entity,
    mut velocity: Vec3,
) -> Result<(), String> {
    let position = *world
        .get::<Position>(target)
        .ok_or("knockback target has no Position")?;
    let map = *world
        .get::<InMap>(target)
        .ok_or("knockback target has no InMap")?;
    let start = Vec3::new(position.x, position.y, position.z);
    require_ground_and_collision(world, map, start)?;
    // MotionMaster makes NPC vertical speed positive; player impulses keep its sign.
    if world.get::<shared::components::Npc>(target).is_some() {
        velocity.y = velocity.y.abs();
    }
    let speed_xy = Vec2::new(velocity.x, velocity.z).length();
    let mut control = world
        .get::<MovementControl>(target)
        .copied()
        .unwrap_or_default();
    control.epoch = control.epoch.wrapping_add(1);
    control.controlled = true;
    world.entity_mut(target).insert((
        control,
        KnockbackFlight {
            origin: start,
            velocity,
            elapsed: 0.0,
            peak: start.y,
            map,
            epoch: control.epoch,
        },
    ));
    if let Some(mut speed) = world.get_mut::<MovementSpeed>(target) {
        speed.0 = speed_xy.abs();
    }
    if let Some(mut motion) = world.get_mut::<PlayerMotion>(target) {
        motion.0 |= PlayerMotion::FALLING;
    }
    crate::interest::mark_interest_position_changed(
        world.get_resource::<crate::interest::InterestDirtyQueue>(),
        target,
    );
    crate::spell_cast::mark_dirty(world, target);
    Ok(())
}

fn require_ground_and_collision(world: &mut World, map: InMap, feet: Vec3) -> Result<(), String> {
    let mut ground = world
        .get_resource_mut::<GroundMap>()
        .ok_or("knockback ground service unavailable")?;
    if ground.ground_in(map, feet) == GroundProbe::Unloaded {
        return Err(format!(
            "knockback ground assets unavailable on map{}",
            map.map_id
        ));
    }
    if world
        .get_resource::<crate::line_of_sight::LineOfSight>()
        .is_none()
    {
        return Err("knockback collision service unavailable".into());
    }
    Ok(())
}

fn launch_velocity(
    world: &World,
    target: Entity,
    start: Vec3,
    origin: Vec3,
    speed_xy: f32,
    speed_z: f32,
) -> Result<Vec3, String> {
    let offset = Vec3::new(start.x - origin.x, 0.0, start.z - origin.z);
    let direction = if offset.length_squared() > 0.0 {
        offset.normalize()
    } else {
        // Unit::KnockbackFrom at the same position uses target facing + pi.
        let rotation = world
            .get::<Rotation>(target)
            .ok_or("coincident knockback target has no Rotation")?;
        -Vec3::new(rotation.y.sin(), 0.0, rotation.y.cos())
    };
    Ok(direction * speed_xy + Vec3::Y * speed_z)
}

pub(crate) fn update_knockbacks(world: &mut World) {
    let delta = world.resource::<Time>().delta_secs();
    let targets: Vec<_> = world
        .query_filtered::<Entity, With<KnockbackFlight>>()
        .iter(world)
        .collect();
    for target in targets {
        advance_owned_flight(world, target, delta);
    }
}

fn advance_owned_flight(world: &mut World, target: Entity, delta: f32) {
    let mut flight = world
        .get::<KnockbackFlight>(target)
        .expect("active knockback")
        .clone();
    let control = *world
        .get::<MovementControl>(target)
        .expect("knockback owns movement control");
    if control.epoch != flight.epoch || !control.controlled {
        world.entity_mut(target).remove::<KnockbackFlight>();
        return;
    }
    if world.get::<InMap>(target) != Some(&flight.map)
        || world
            .get::<Health>(target)
            .is_some_and(|health| health.current <= 0.0)
    {
        cancel(world, target);
        return;
    }
    let current = *world.get::<Position>(target).expect("moving unit");
    flight.elapsed += delta;
    let predicted = flight.at(flight.elapsed);
    let next = crate::spell_cast::teleport::clamp_to_wall(
        world.resource::<crate::line_of_sight::LineOfSight>(),
        flight.map,
        Vec3::new(current.x, current.y, current.z),
        predicted,
    );
    flight.peak = flight.peak.max(next.y);
    advance_over_ground(world, target, flight, current, next);
}

fn advance_over_ground(
    world: &mut World,
    target: Entity,
    flight: KnockbackFlight,
    current: Position,
    mut next: Vec3,
) {
    let probe_feet = Vec3::new(next.x, current.y.max(next.y), next.z);
    let (probe, minimum) = {
        let mut ground = world.resource_mut::<GroundMap>();
        (
            ground.ground_in(flight.map, probe_feet),
            ground.min_height(flight.map.map_id, next),
        )
    };
    if probe == GroundProbe::Unloaded {
        error!(
            "Knockback on map{} entered unavailable ground assets; stopped at last authored position",
            flight.map.map_id
        );
        cancel(world, target);
        return;
    }
    let landed = matches!(probe, GroundProbe::Supported(height) if flight.vertical_speed() <= 0.0 && next.y <= height);
    if let GroundProbe::Supported(height) = probe
        && landed
    {
        next.y = height;
    }
    write_position(world, target, next);
    if minimum.is_some_and(|height| next.y < height) {
        cancel(world, target); // Normal player gravity owns the sourced void/death transition.
    } else if landed {
        apply_landing_damage(world, target, (flight.peak - next.y).max(0.0));
        cancel(world, target);
    } else {
        world.entity_mut(target).insert(flight);
    }
}

fn write_position(world: &mut World, target: Entity, point: Vec3) {
    *world.get_mut::<Position>(target).expect("moving unit") = Position {
        x: point.x,
        y: point.y,
        z: point.z,
    };
    crate::interest::mark_interest_position_changed(
        world.get_resource::<crate::interest::InterestDirtyQueue>(),
        target,
    );
    crate::spell_cast::mark_dirty(world, target);
}

fn apply_landing_damage(world: &mut World, target: Entity, distance: f32) {
    if world.get::<shared::components::Player>(target).is_none()
        || world
            .get::<shared::components::FeatherFall>(target)
            .is_some_and(|feather| feather.0)
    {
        return;
    }
    let Some(previous) = world.get::<Health>(target).copied() else {
        return;
    };
    let damage = previous.max
        * shared::movement::fall_damage_percent(distance)
        * crate::networking::fall_damage_multiplier(world.get::<crate::auras::ActiveAuras>(target));
    let log = crate::death::deal_environmental_damage(
        target,
        shared::protocol::EnvironmentalKind::Falling,
        &mut world.get_mut::<Health>(target).expect("player health"),
        damage,
    );
    if let Some(log) = log {
        crate::achievement::record_survived_fall(world, target, distance);
        crate::spell_cast::log_event(world, log);
    }
    let current = world.get::<Health>(target).expect("player health").current;
    if previous.current > 0.0 && current <= 0.0 {
        world.write_message(crate::death::PlayerDeathTransition { player: target });
    }
}

pub(crate) fn cancel(world: &mut World, target: Entity) {
    let Some(flight) = world.get::<KnockbackFlight>(target).cloned() else {
        return;
    };
    world.entity_mut(target).remove::<KnockbackFlight>();
    let mut control = world
        .get_mut::<MovementControl>(target)
        .expect("knockback movement owner");
    if control.epoch != flight.epoch {
        return;
    }
    control.controlled = false;
    control.epoch = control.epoch.wrapping_add(1);
    if let Some(mut speed) = world.get_mut::<MovementSpeed>(target) {
        speed.0 = 0.0;
    }
    if let Some(mut motion) = world.get_mut::<PlayerMotion>(target) {
        motion.0 &= !PlayerMotion::FALLING;
    }
    let height = world.get::<Position>(target).expect("moving unit").y;
    if let Some(mut fall) = world.get_mut::<crate::networking::FallTracker>(target) {
        fall.restart_at(height);
    }
    crate::interest::mark_interest_position_changed(
        world.get_resource::<crate::interest::InterestDirtyQueue>(),
        target,
    );
    crate::spell_cast::mark_dirty(world, target);
}
