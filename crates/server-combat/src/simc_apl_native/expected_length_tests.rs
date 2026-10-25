//! Configured fixture length, not elapsed time, remaining time or a fixed300.
use super::*;

fn fixture_world() -> (World, Entity) {
    let mut world = World::new();
    let mut clock = Time::<()>::default();
    clock.advance_by(Duration::from_secs(83));
    world.insert_resource(clock);
    let actor = world.spawn_empty().id();
    (world, actor)
}

fn fixture(duration: f64) -> Encounter {
    Encounter::Fixture {
        started_at: 11.0,
        duration,
        style: FightStyle::Patchwerk,
    }
}

#[test]
fn native_expected_combat_length_reads_configured_duration_not_elapsed_or_remaining() {
    let (world, actor) = fixture_world();
    let bindings = Bindings::default();
    for duration in [147.5, 300.0, 600.25] {
        let facts = NativeFacts::new(&world, actor, &bindings, fixture(duration), 0);
        assert_eq!(facts.value("time").unwrap(), 72.0);
        assert_eq!(facts.value("fight_remains").unwrap(), duration - 72.0);
        assert_eq!(facts.value("expected_combat_length").unwrap(), duration);
    }
}

#[test]
fn native_expected_combat_length_invalid_fixture_duration_has_no_default() {
    let (world, actor) = fixture_world();
    let bindings = Bindings::default();
    for duration in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let facts = NativeFacts::new(&world, actor, &bindings, fixture(duration), 0);
        let error = facts.value("expected_combat_length").unwrap_err();
        assert!(
            error.contains("EXPECTED_COMBAT_LENGTH_UNAVAILABLE"),
            "{duration}: {error}"
        );
    }
}

#[test]
fn native_expected_combat_length_dungeon_is_explicitly_unavailable() {
    let (world, actor) = fixture_world();
    let bindings = Bindings::default();
    let facts = NativeFacts::new(
        &world,
        actor,
        &bindings,
        Encounter::Dungeon { started_at: 11.0 },
        0,
    );
    let error = facts.value("expected_combat_length").unwrap_err();
    assert!(
        error.contains("EXPECTED_COMBAT_LENGTH_UNAVAILABLE"),
        "{error}"
    );
}

#[test]
fn native_expected_combat_length_action_context_uses_same_configured_fixture() {
    let (world, actor) = fixture_world();
    let bindings = Bindings::default();
    let facts = NativeFacts::new(&world, actor, &bindings, fixture(215.75), 0);
    let action = Action {
        source_row: None,
        name: "fireball".into(),
        target: None,
        options: Default::default(),
    };
    assert_eq!(
        facts.value_for(&action, "expected_combat_length").unwrap(),
        215.75
    );
}
