use super::*;

#[test]
fn start_pending_rejects_self_duel() {
    let mut state = DuelServerState::default();
    let player = Entity::from_bits(1);

    let result = state.start_pending(player, player);

    assert_eq!(result, Err("cannot duel yourself"));
}

#[test]
fn build_duel_boundary_uses_midpoint() {
    let boundary = build_duel_boundary(
        &Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        &Position {
            x: 20.0,
            y: 0.0,
            z: 10.0,
        },
    );

    assert_eq!(boundary.center_x, 10.0);
    assert_eq!(boundary.center_z, 5.0);
    assert_eq!(boundary.radius, DUEL_BOUNDARY_RADIUS);
}

#[test]
fn resolve_active_duel_detects_out_of_bounds_loss() {
    let challenger = Entity::from_bits(1);
    let target = Entity::from_bits(2);
    let resolution = resolve_duel_state(
        &Position {
            x: 100.0,
            y: 0.0,
            z: 0.0,
        },
        &Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        100.0,
        100.0,
        DuelBoundary {
            center_x: 0.0,
            center_z: 0.0,
            radius: 30.0,
        },
        challenger,
        target,
    );

    assert_eq!(
        resolution,
        Some(DuelResolution::WinLoss {
            winner: target,
            loser: challenger,
        })
    );
}

#[test]
fn resolve_active_duel_detects_health_loss() {
    let challenger = Entity::from_bits(1);
    let target = Entity::from_bits(2);
    let resolution = resolve_duel_state(
        &Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        &Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        0.0,
        50.0,
        DuelBoundary {
            center_x: 0.0,
            center_z: 0.0,
            radius: 30.0,
        },
        challenger,
        target,
    );

    assert_eq!(
        resolution,
        Some(DuelResolution::WinLoss {
            winner: target,
            loser: challenger,
        })
    );
}
