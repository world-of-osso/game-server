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

// --- Ghost form ---

#[test]
fn ghost_restrictions_all_blocked() {
    let actions = [
        GhostRestriction::Combat, GhostRestriction::Loot,
        GhostRestriction::Trade, GhostRestriction::QuestInteract,
        GhostRestriction::UseObject, GhostRestriction::Mount,
        GhostRestriction::Cast, GhostRestriction::Chat,
    ];
    for action in actions {
        assert!(is_ghost_restricted(DeathState::Ghost, action));
    }
}

#[test]
fn alive_no_restrictions() {
    assert!(!is_ghost_restricted(DeathState::Alive, GhostRestriction::Combat));
    assert!(!is_ghost_restricted(DeathState::Alive, GhostRestriction::Trade));
}

#[test]
fn dead_no_ghost_restrictions() {
    // Dead (not yet released) is a different state — ghost restrictions don't apply
    assert!(!is_ghost_restricted(DeathState::Dead, GhostRestriction::Combat));
}

#[test]
fn living_sees_living() {
    assert!(ghost_visible(DeathState::Alive, DeathState::Alive, false));
}

#[test]
fn living_cannot_see_ghost() {
    assert!(!ghost_visible(DeathState::Alive, DeathState::Ghost, false));
}

#[test]
fn ghost_sees_ghost() {
    assert!(ghost_visible(DeathState::Ghost, DeathState::Ghost, false));
}

#[test]
fn ghost_sees_living() {
    assert!(ghost_visible(DeathState::Ghost, DeathState::Alive, false));
}

#[test]
fn ghost_sees_spirit_healer() {
    assert!(ghost_visible(DeathState::Ghost, DeathState::Alive, true));
}

#[test]
fn living_sees_spirit_healer() {
    // Spirit healers are visible to everyone
    assert!(ghost_visible(DeathState::Alive, DeathState::Alive, true));
}

// --- Corpse run ---

fn sample_corpse() -> Corpse {
    Corpse { player: 100, map_id: 0, x: 100.0, y: 200.0, z: 50.0, created_at: 1000 }
}

#[test]
fn in_corpse_range_close() {
    let corpse = sample_corpse();
    assert!(in_corpse_range(105.0, 200.0, 50.0, &corpse)); // 5 yards away
}

#[test]
fn in_corpse_range_exact_boundary() {
    let corpse = sample_corpse();
    assert!(in_corpse_range(100.0 + CORPSE_RESURRECT_RANGE, 200.0, 50.0, &corpse));
}

#[test]
fn in_corpse_range_too_far() {
    let corpse = sample_corpse();
    assert!(!in_corpse_range(100.0 + CORPSE_RESURRECT_RANGE + 0.1, 200.0, 50.0, &corpse));
}

#[test]
fn resurrect_at_corpse_success() {
    let corpse = sample_corpse();
    let result = resurrect_at_corpse(DeathState::Ghost, 105.0, 200.0, 50.0, 0, &corpse).unwrap();
    assert_eq!(result.x, 100.0);
    assert_eq!(result.y, 200.0);
    assert_eq!(result.z, 50.0);
    assert_eq!(result.map_id, 0);
}

#[test]
fn resurrect_at_corpse_not_ghost() {
    let corpse = sample_corpse();
    assert_eq!(
        resurrect_at_corpse(DeathState::Alive, 100.0, 200.0, 50.0, 0, &corpse),
        Err(DeathError::NotGhost)
    );
    assert_eq!(
        resurrect_at_corpse(DeathState::Dead, 100.0, 200.0, 50.0, 0, &corpse),
        Err(DeathError::NotGhost)
    );
}

#[test]
fn resurrect_at_corpse_too_far() {
    let corpse = sample_corpse();
    assert_eq!(
        resurrect_at_corpse(DeathState::Ghost, 500.0, 500.0, 50.0, 0, &corpse),
        Err(DeathError::NotGhost)
    );
}

#[test]
fn resurrect_at_corpse_wrong_map() {
    let corpse = sample_corpse();
    assert_eq!(
        resurrect_at_corpse(DeathState::Ghost, 100.0, 200.0, 50.0, 1, &corpse),
        Err(DeathError::NotGhost)
    );
}
