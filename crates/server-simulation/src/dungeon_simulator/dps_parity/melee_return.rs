//! Shared melee intent after native movement abilities, not a replacement for their movement.
//! SimC76f8118 sc_demon_hunter.cpp1559–1611 returns over distance/run_speed.
use super::super::movement;
use crate::simc_apl_native::FixtureMovement;
use bevy::prelude::*;
use server_follower_apl::literal_rotation::melee_intent::{
    DisplacementReturn, ReturningToMelee, forced_movement,
};
use shared::components::Position;

struct WalkStep {
    target: Entity,
    position: Position,
    /// The melee edge this walk ends at.
    goal: Vec3,
    speed: f32,
    arrived: bool,
}

pub(super) fn advance(world: &mut World) {
    let delta = world.resource::<Time>().delta_secs();
    let actors = server::system_states::entities_with::<With<FixtureMovement>>(world);
    for actor in actors {
        advance_actor(world, actor, delta);
    }
}

fn advance_actor(world: &mut World, actor: Entity, delta: f32) {
    if world
        .get::<crate::simc_apl_native::AplMovementIntent>(actor)
        .is_some()
    {
        return;
    }
    // A charge can finish inside melee reach facing along its short travel,
    // away from the opponent. The fixture still owns the actor's melee intent.
    // Do not override active forced movement or turn an active channel.
    if forced_movement(world, actor) {
        if world.get::<crate::combat::AutoAttack>(actor).is_some() {
            world.entity_mut(actor).insert(DisplacementReturn);
        }
        return;
    }
    clear_displacement_return_at_melee(world, actor);
    face_melee_intent(world, actor);
    if matches!(
        world.get::<crate::creature_ai::CreatureState>(actor),
        Some(crate::creature_ai::CreatureState::Pursuing { .. })
    ) {
        let walking = read_walk_step(world, actor, delta).is_some();
        set_return_walking(world, actor, walking);
        return;
    }
    let step = read_walk_step(world, actor, delta);
    if step.is_none() && world.get::<ReturningToMelee>(actor).is_none() {
        return;
    }
    let walking = step.as_ref().is_some_and(|step| !step.arrived);
    set_return_walking(world, actor, walking);
    let goal = step
        .as_ref()
        .filter(|step| !step.arrived)
        .map(|step| step.goal);
    crate::creature_ai::publish_planned_destination(world, actor, goal);
    let Some(step) = step else {
        return;
    };
    crate::creature_ai::move_follower(world, actor, step.position, step.speed);
    super::face_actor_toward_primary(world, actor, step.target);
    // The encounter's position=back contract survives moving across the pack.
    let opponents =
        server::system_states::entities_with::<With<crate::creature_scripts::TrainingDummy>>(world);
    for target in opponents {
        super::face_dummy_away_from_actor(world, actor, target);
    }
}

fn clear_displacement_return_at_melee(world: &mut World, actor: Entity) {
    let reached = world
        .get::<crate::combat::AutoAttack>(actor)
        .is_none_or(|attack| {
            if world
                .get::<shared::components::Health>(attack.victim)
                .is_none_or(|health| health.current <= 0.0)
            {
                return true;
            }
            let Some(from) = world.get::<Position>(actor) else {
                return true;
            };
            let Some(target) = world.get::<Position>(attack.victim) else {
                return true;
            };
            let Some(range) = crate::scripted_ai::model_reach::melee_range(world, actor, attack.victim) else {
                error!(?actor, target = ?attack.victim, "Melee return requires sourced combat reaches");
                return false;
            };
            (target.x - from.x).hypot(target.z - from.z) <= range
        });
    if reached {
        world.entity_mut(actor).remove::<DisplacementReturn>();
    }
}

fn set_return_walking(world: &mut World, actor: Entity, walking: bool) {
    if walking {
        world.entity_mut(actor).insert(ReturningToMelee);
    } else {
        world.entity_mut(actor).remove::<ReturningToMelee>();
    }
    if let Some(mut moving) = world.get_mut::<crate::networking::Moving>(actor) {
        moving.0 = walking;
    }
}

fn face_melee_intent(world: &mut World, actor: Entity) {
    if world.get::<shared::casting::CastState>(actor).is_some() {
        return;
    }
    let target = world
        .get::<crate::combat::AutoAttack>(actor)
        .map(|attack| attack.victim);
    if let Some(target) = target.filter(|target| world.get_entity(*target).is_ok()) {
        super::face_actor_toward_primary(world, actor, target);
    }
}

fn read_walk_step(world: &World, actor: Entity, delta: f32) -> Option<WalkStep> {
    if forced_movement(world, actor) || !movement::can_move(world, actor) {
        return None;
    }
    let target = world.get::<crate::combat::AutoAttack>(actor)?.victim;
    if world.get::<shared::components::Health>(target)?.current <= 0.0 {
        return None;
    }
    let from = *world.get::<Position>(actor)?;
    let to = *world.get::<Position>(target)?;
    let range = crate::scripted_ai::model_reach::melee_range(world, actor, target)?;
    walk_toward_melee(
        target,
        from,
        to,
        range,
        movement::read_speed(world, actor),
        delta,
    )
}

fn walk_toward_melee(
    target: Entity,
    from: Position,
    to: Position,
    range: f32,
    speed: f32,
    delta: f32,
) -> Option<WalkStep> {
    let offset = Vec3::new(to.x - from.x, 0.0, to.z - from.z);
    let distance = offset.length();
    let remaining = distance - range;
    if remaining <= 0.0 || speed <= 0.0 {
        return None;
    }
    let step = (speed * delta).min(remaining);
    let here = Vec3::new(from.x, from.y, from.z);
    let position = here + offset * (step / distance);
    Some(WalkStep {
        target,
        position: Position {
            x: position.x,
            y: position.y,
            z: position.z,
        },
        goal: here + offset * (remaining / distance),
        speed,
        arrived: step >= remaining,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use server::simc_apl::Facts;
    use server::simc_apl_native::{Bindings, Encounter, MovementDestination, NativeFacts};
    use shared::components::{
        CharacterAppearance, CharacterVisual, CreatureMotion, Health, Rotation,
    };

    fn read_distance(world: &World, actor: Entity) -> Result<f64, String> {
        NativeFacts::new(
            world,
            actor,
            &Bindings::default(),
            Encounter::dungeon(0.0),
            1,
        )
        .value("movement.distance")
    }

    fn visual() -> CharacterVisual {
        CharacterVisual {
            race: 1,
            class: 1,
            appearance: CharacterAppearance::default(),
        }
    }

    fn tick(world: &mut World) {
        world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(500));
        advance(world);
    }

    #[test]
    fn melee_return_publishes_the_melee_edge_until_it_arrives() {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        let health = Health {
            current: 100.0,
            max: 100.0,
        };
        let target = world
            .spawn((
                Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                health,
                visual(),
            ))
            .id();
        let actor = world
            .spawn((
                Position {
                    x: 12.0,
                    y: 0.0,
                    z: 0.0,
                },
                Rotation {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                health,
                CreatureMotion::Still,
                FixtureMovement {
                    distance_to_move: 0.0,
                },
                crate::combat::AutoAttack::new(target),
                visual(),
            ))
            .id();
        assert_eq!(read_distance(&world, actor).unwrap(), 0.0, "idle");
        let range = crate::scripted_ai::model_reach::melee_range(&world, actor, target).unwrap();
        tick(&mut world);
        let edge = world.get::<MovementDestination>(actor).unwrap().0;
        assert!(edge.distance(Vec3::new(range, 0.0, 0.0)) < 1e-4, "{edge:?}");
        let x = world.get::<Position>(actor).unwrap().x;
        assert!((x - 8.5).abs() < 1e-4, "one 0.5s run stride: {x}");
        let distance = read_distance(&world, actor).unwrap();
        assert!((distance - f64::from(x - range)).abs() < 1e-5, "{distance}");
        for _ in 0..3 {
            tick(&mut world);
        }
        assert!(world.get::<ReturningToMelee>(actor).is_none());
        assert!(world.get::<MovementDestination>(actor).is_none());
    }
}
