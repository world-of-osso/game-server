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

// --- Release spirit / graveyards ---

fn sample_graveyards() -> GraveyardRegistry {
    let mut reg = GraveyardRegistry::new();
    reg.add(Graveyard {
        id: 1, zone_id: 12, faction: 0, map_id: 0,
        x: 100.0, y: 200.0, z: 50.0,
    });
    reg.add(Graveyard {
        id: 2, zone_id: 12, faction: 1, map_id: 0, // Alliance only
        x: 500.0, y: 500.0, z: 50.0,
    });
    reg.add(Graveyard {
        id: 3, zone_id: 14, faction: 0, map_id: 1, // Different map
        x: 0.0, y: 0.0, z: 0.0,
    });
    reg
}

#[test]
fn release_spirit_nearest_graveyard() {
    let gys = sample_graveyards();
    let result = release_spirit(DeathState::Dead, 0, 110.0, 210.0, 50.0, 0, &gys).unwrap();
    assert_eq!(result.graveyard_id, 1);
    assert_eq!(result.x, 100.0);
    assert_eq!(result.y, 200.0);
}

#[test]
fn release_spirit_faction_filter() {
    let gys = sample_graveyards();
    let result = release_spirit(DeathState::Dead, 0, 490.0, 490.0, 50.0, 1, &gys).unwrap();
    assert_eq!(result.graveyard_id, 2);
}

#[test]
fn release_spirit_horde_skips_alliance_gy() {
    let gys = sample_graveyards();
    let result = release_spirit(DeathState::Dead, 0, 490.0, 490.0, 50.0, 2, &gys).unwrap();
    assert_eq!(result.graveyard_id, 1); // Neutral, not Alliance-only
}

#[test]
fn release_spirit_different_map() {
    let gys = sample_graveyards();
    let result = release_spirit(DeathState::Dead, 1, 5.0, 5.0, 0.0, 0, &gys).unwrap();
    assert_eq!(result.graveyard_id, 3);
    assert_eq!(result.map_id, 1);
}

#[test]
fn release_spirit_not_dead_fails() {
    let gys = sample_graveyards();
    assert_eq!(release_spirit(DeathState::Alive, 0, 0.0, 0.0, 0.0, 0, &gys), Err(DeathError::NotDead));
    assert_eq!(release_spirit(DeathState::Ghost, 0, 0.0, 0.0, 0.0, 0, &gys), Err(DeathError::NotDead));
}

#[test]
fn release_spirit_no_graveyard_fails() {
    let gys = GraveyardRegistry::new();
    assert!(release_spirit(DeathState::Dead, 0, 0.0, 0.0, 0.0, 0, &gys).is_err());
}

#[test]
fn graveyard_registry_len() {
    let gys = sample_graveyards();
    assert_eq!(gys.len(), 3);
}
