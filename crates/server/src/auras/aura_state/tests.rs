use super::*;
use shared::components::Health;

fn assert_health_state(state: u32, maximum: f32, cases: &[(f32, bool)]) {
    let mut world = World::new();
    let target = world
        .spawn(Health {
            current: maximum,
            max: maximum,
        })
        .id();
    for &(current, expected) in cases {
        world.get_mut::<Health>(target).unwrap().current = current;
        assert_eq!(
            read_computed_state(&world, target, state),
            Some(expected),
            "state{state}, native health{current}/{maximum}"
        );
    }
}

#[test]
fn health_aura_state_wounded_20_strict_boundary() {
    assert_health_state(
        2,
        100.0,
        &[
            (1.0, true),
            (19.0, true),
            (20.0, false),
            (21.0, false),
            (100.0, false),
        ],
    );
}

#[test]
fn health_aura_state_wounded_25_strict_boundary() {
    assert_health_state(
        6,
        100.0,
        &[(24.0, true), (25.0, false), (26.0, false), (100.0, false)],
    );
}

#[test]
fn health_aura_state_wounded_35_strict_boundary() {
    assert_health_state(
        13,
        100.0,
        &[(34.0, true), (35.0, false), (36.0, false), (100.0, false)],
    );
}

#[test]
fn health_aura_state_wound_health_20_80_has_both_strict_boundaries() {
    assert_health_state(
        21,
        100.0,
        &[
            (19.0, true),
            (20.0, false),
            (50.0, false),
            (80.0, false),
            (81.0, true),
            (100.0, true),
        ],
    );
}

#[test]
fn health_aura_state_healthy_75_strict_boundary() {
    assert_health_state(
        23,
        100.0,
        &[
            (1.0, false),
            (74.0, false),
            (75.0, false),
            (76.0, true),
            (100.0, true),
        ],
    );
}

#[test]
fn health_aura_state_wound_health_35_80_has_both_strict_boundaries() {
    assert_health_state(
        24,
        100.0,
        &[
            (34.0, true),
            (35.0, false),
            (50.0, false),
            (80.0, false),
            (81.0, true),
        ],
    );
}

#[test]
fn health_aura_state_wounded_50_strict_boundary() {
    assert_health_state(
        25,
        100.0,
        &[(49.0, true), (50.0, false), (51.0, false), (100.0, false)],
    );
}

#[test]
fn health_aura_state_thresholds_use_tc_integer_health_counts() {
    // TC Unit.h804–809 returns uint64 CountPctFromMaxHealth, not a floating ratio.
    assert_health_state(2, 101.0, &[(19.0, true), (20.0, false), (21.0, false)]);
    assert_health_state(23, 101.0, &[(75.0, false), (76.0, true)]);
    assert_health_state(21, 101.0, &[(80.0, false), (81.0, true)]);
    assert_health_state(6, 119.0, &[(28.0, true), (29.0, false), (30.0, false)]);
}

#[test]
fn health_aura_state_unmodeled_states_stay_unknown() {
    let mut world = World::new();
    let target = world
        .spawn(Health {
            current: 19.0,
            max: 100.0,
        })
        .id();
    for state in [
        0,
        1,
        3,
        4,
        5,
        7,
        8,
        9,
        10,
        11,
        12,
        14,
        16,
        17,
        18,
        19,
        20,
        22,
        26,
        u32::MAX,
    ] {
        assert_eq!(
            read_computed_state(&world, target, state),
            None,
            "unmodeled state{state}"
        );
    }
}

#[test]
fn health_aura_state_missing_native_health_stays_unknown() {
    let mut world = World::new();
    let target = world.spawn_empty().id();
    for state in [2, 6, 13, 21, 23, 24, 25] {
        assert_eq!(read_computed_state(&world, target, state), None);
    }
}
