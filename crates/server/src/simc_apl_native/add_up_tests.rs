//! SimC76f8118 raid_event.cpp2440–2443/3068–3108: event activity, not enemy count.
use super::*;

fn world_at(seconds: u64, expires_at: Option<u64>) -> (World, Entity) {
    let mut world = World::new();
    let mut clock: Time = Time::default();
    clock.advance_to(Duration::from_secs(seconds));
    world.insert_resource(clock);
    let actor = world
        .spawn(FixtureAdds {
            next_spawn_at: Duration::from_secs(50),
            expires_at: expires_at.map(Duration::from_secs),
            interval: Duration::from_secs(30),
            lifetime: Duration::from_secs(10),
            count: 3,
        })
        .id();
    (world, actor)
}

fn read(world: &World, actor: Entity, property: &str) -> Result<f64, String> {
    let bindings = Bindings::default();
    let facts = NativeFacts::new(
        world,
        actor,
        &bindings,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: FightStyle::Patchwerk,
        },
        1,
    );
    facts.value(&format!("raid_event.adds.{property}"))
}

#[test]
fn native_add_up_missing_event_is_zero() {
    let (mut world, actor) = world_at(5, None);
    world.entity_mut(actor).remove::<FixtureAdds>();
    assert_eq!(read(&world, actor, "exists").unwrap(), 0.0);
    assert_eq!(read(&world, actor, "up").unwrap(), 0.0);
}

#[test]
fn native_add_up_configured_waiting_event_is_not_active() {
    let (world, actor) = world_at(5, None);
    assert_eq!(read(&world, actor, "exists").unwrap(), 1.0);
    assert_eq!(read(&world, actor, "count").unwrap(), 3.0);
    assert_eq!(read(&world, actor, "up").unwrap(), 0.0);
}

#[test]
fn native_add_up_active_and_respawned_waves_are_active() {
    for (now, expires) in [(10, 20), (30, 40)] {
        let (world, actor) = world_at(now, Some(expires));
        assert_eq!(read(&world, actor, "remains").unwrap(), 10.0);
        assert_eq!(read(&world, actor, "up").unwrap(), 1.0);
    }
}

#[test]
fn native_add_up_expiration_boundary_and_retirement_are_not_active() {
    for (now, expires) in [(20, Some(20)), (21, Some(20)), (21, None)] {
        let (world, actor) = world_at(now, expires);
        assert_eq!(read(&world, actor, "exists").unwrap(), 1.0);
        assert_eq!(read(&world, actor, "remains").unwrap(), 0.0);
        assert_eq!(read(&world, actor, "up").unwrap(), 0.0);
    }
}

#[test]
fn native_add_up_existing_event_properties_keep_their_values() {
    let (world, actor) = world_at(10, Some(20));
    for (property, expected) in [
        ("exists", 1.0),
        ("count", 3.0),
        ("duration", 10.0),
        ("cooldown", 30.0),
        ("remains", 10.0),
        ("in", 40.0),
    ] {
        assert_eq!(read(&world, actor, property).unwrap(), expected);
    }
    assert!(read(&world, actor, "missing").is_err());
}
