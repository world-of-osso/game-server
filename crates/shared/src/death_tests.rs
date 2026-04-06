use super::*;

// --- check_death ---

#[test]
fn check_death_at_zero() {
    assert!(check_death(0.0));
}

#[test]
fn check_death_below_zero() {
    assert!(check_death(-10.0));
}

#[test]
fn check_death_alive() {
    assert!(!check_death(1.0));
    assert!(!check_death(100.0));
}

// --- on_death ---

#[test]
fn on_death_creates_corpse() {
    let result = on_death(100, 0, 50.0, 60.0, 70.0, 1000);

    assert_eq!(result.player, 100);
    assert_eq!(result.corpse.player, 100);
    assert_eq!(result.corpse.map_id, 0);
    assert_eq!(result.corpse.x, 50.0);
    assert_eq!(result.corpse.y, 60.0);
    assert_eq!(result.corpse.z, 70.0);
    assert_eq!(result.corpse.created_at, 1000);
}

// --- State transitions ---

#[test]
fn alive_to_dead() {
    assert_eq!(transition_state(DeathState::Alive, DeathState::Dead), Ok(DeathState::Dead));
}

#[test]
fn dead_to_ghost() {
    assert_eq!(transition_state(DeathState::Dead, DeathState::Ghost), Ok(DeathState::Ghost));
}

#[test]
fn dead_to_resurrecting() {
    assert_eq!(transition_state(DeathState::Dead, DeathState::Resurrecting), Ok(DeathState::Resurrecting));
}

#[test]
fn ghost_to_resurrecting() {
    assert_eq!(transition_state(DeathState::Ghost, DeathState::Resurrecting), Ok(DeathState::Resurrecting));
}

#[test]
fn resurrecting_to_alive() {
    assert_eq!(transition_state(DeathState::Resurrecting, DeathState::Alive), Ok(DeathState::Alive));
}

#[test]
fn alive_to_alive_fails() {
    assert_eq!(transition_state(DeathState::Alive, DeathState::Alive), Err(DeathError::AlreadyAlive));
}

#[test]
fn dead_to_dead_fails() {
    assert_eq!(transition_state(DeathState::Dead, DeathState::Dead), Err(DeathError::AlreadyDead));
}

#[test]
fn alive_to_ghost_fails() {
    assert!(transition_state(DeathState::Alive, DeathState::Ghost).is_err());
}

#[test]
fn dead_to_alive_fails() {
    // Must go through Ghost or Resurrecting first
    assert!(transition_state(DeathState::Dead, DeathState::Alive).is_err());
}

#[test]
fn ghost_to_alive_fails() {
    // Must go through Resurrecting
    assert!(transition_state(DeathState::Ghost, DeathState::Alive).is_err());
}

// --- Full lifecycle ---

#[test]
fn full_death_ghost_resurrect_lifecycle() {
    let mut state = DeathState::Alive;
    state = transition_state(state, DeathState::Dead).unwrap();
    state = transition_state(state, DeathState::Ghost).unwrap();
    state = transition_state(state, DeathState::Resurrecting).unwrap();
    state = transition_state(state, DeathState::Alive).unwrap();
    assert_eq!(state, DeathState::Alive);
}

#[test]
fn death_direct_resurrect_lifecycle() {
    let mut state = DeathState::Alive;
    state = transition_state(state, DeathState::Dead).unwrap();
    // Direct res without releasing spirit
    state = transition_state(state, DeathState::Resurrecting).unwrap();
    state = transition_state(state, DeathState::Alive).unwrap();
    assert_eq!(state, DeathState::Alive);
}
