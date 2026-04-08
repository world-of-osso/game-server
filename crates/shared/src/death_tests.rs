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
    assert_eq!(
        transition_state(DeathState::Alive, DeathState::Dead),
        Ok(DeathState::Dead)
    );
}

#[test]
fn dead_to_ghost() {
    assert_eq!(
        transition_state(DeathState::Dead, DeathState::Ghost),
        Ok(DeathState::Ghost)
    );
}

#[test]
fn dead_to_resurrecting() {
    assert_eq!(
        transition_state(DeathState::Dead, DeathState::Resurrecting),
        Ok(DeathState::Resurrecting)
    );
}

#[test]
fn ghost_to_resurrecting() {
    assert_eq!(
        transition_state(DeathState::Ghost, DeathState::Resurrecting),
        Ok(DeathState::Resurrecting)
    );
}

#[test]
fn resurrecting_to_alive() {
    assert_eq!(
        transition_state(DeathState::Resurrecting, DeathState::Alive),
        Ok(DeathState::Alive)
    );
}

#[test]
fn alive_to_alive_fails() {
    assert_eq!(
        transition_state(DeathState::Alive, DeathState::Alive),
        Err(DeathError::AlreadyAlive)
    );
}

#[test]
fn dead_to_dead_fails() {
    assert_eq!(
        transition_state(DeathState::Dead, DeathState::Dead),
        Err(DeathError::AlreadyDead)
    );
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
        id: 1,
        zone_id: 12,
        faction: 0,
        map_id: 0,
        x: 100.0,
        y: 200.0,
        z: 50.0,
    });
    reg.add(Graveyard {
        id: 2,
        zone_id: 12,
        faction: 1,
        map_id: 0, // Alliance only
        x: 500.0,
        y: 500.0,
        z: 50.0,
    });
    reg.add(Graveyard {
        id: 3,
        zone_id: 14,
        faction: 0,
        map_id: 1, // Different map
        x: 0.0,
        y: 0.0,
        z: 0.0,
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
    assert_eq!(
        release_spirit(DeathState::Alive, 0, 0.0, 0.0, 0.0, 0, &gys),
        Err(DeathError::NotDead)
    );
    assert_eq!(
        release_spirit(DeathState::Ghost, 0, 0.0, 0.0, 0.0, 0, &gys),
        Err(DeathError::NotDead)
    );
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
        GhostRestriction::Combat,
        GhostRestriction::Loot,
        GhostRestriction::Trade,
        GhostRestriction::QuestInteract,
        GhostRestriction::UseObject,
        GhostRestriction::Mount,
        GhostRestriction::Cast,
        GhostRestriction::Chat,
    ];
    for action in actions {
        assert!(is_ghost_restricted(DeathState::Ghost, action));
    }
}

#[test]
fn alive_no_restrictions() {
    assert!(!is_ghost_restricted(
        DeathState::Alive,
        GhostRestriction::Combat
    ));
    assert!(!is_ghost_restricted(
        DeathState::Alive,
        GhostRestriction::Trade
    ));
}

#[test]
fn dead_no_ghost_restrictions() {
    // Dead (not yet released) is a different state — ghost restrictions don't apply
    assert!(!is_ghost_restricted(
        DeathState::Dead,
        GhostRestriction::Combat
    ));
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
    Corpse {
        player: 100,
        map_id: 0,
        x: 100.0,
        y: 200.0,
        z: 50.0,
        created_at: 1000,
    }
}

#[test]
fn in_corpse_range_close() {
    let corpse = sample_corpse();
    assert!(in_corpse_range(105.0, 200.0, 50.0, &corpse)); // 5 yards away
}

#[test]
fn in_corpse_range_exact_boundary() {
    let corpse = sample_corpse();
    assert!(in_corpse_range(
        100.0 + CORPSE_RESURRECT_RANGE,
        200.0,
        50.0,
        &corpse
    ));
}

#[test]
fn in_corpse_range_too_far() {
    let corpse = sample_corpse();
    assert!(!in_corpse_range(
        100.0 + CORPSE_RESURRECT_RANGE + 0.1,
        200.0,
        50.0,
        &corpse
    ));
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

// --- Resurrection sickness ---

#[test]
fn spirit_healer_gives_sickness() {
    let sickness = apply_res_sickness(ResurrectSource::SpiritHealer, 60, 1000);
    assert!(sickness.is_some());
    let s = sickness.unwrap();
    assert_eq!(s.expires_at, 1000 + RES_SICKNESS_DURATION);
}

#[test]
fn corpse_run_no_sickness() {
    assert!(apply_res_sickness(ResurrectSource::CorpseRun, 60, 1000).is_none());
}

#[test]
fn player_spell_no_sickness() {
    assert!(apply_res_sickness(ResurrectSource::PlayerSpell, 60, 1000).is_none());
}

#[test]
fn low_level_no_sickness() {
    // Level 9 < min level 10 — no sickness even from spirit healer
    assert!(apply_res_sickness(ResurrectSource::SpiritHealer, 9, 1000).is_none());
}

#[test]
fn min_level_gets_sickness() {
    assert!(
        apply_res_sickness(ResurrectSource::SpiritHealer, RES_SICKNESS_MIN_LEVEL, 1000).is_some()
    );
}

#[test]
fn sickness_is_active() {
    let s = ResSickness { expires_at: 2000 };
    assert!(s.is_active(1500));
    assert!(!s.is_active(2000));
    assert!(!s.is_active(2500));
}

#[test]
fn sickness_remaining() {
    let s = ResSickness { expires_at: 2000 };
    assert_eq!(s.remaining(1500), 500);
    assert_eq!(s.remaining(2000), 0);
    assert_eq!(s.remaining(3000), 0);
}

#[test]
fn sickness_stat_penalty() {
    let s = ResSickness { expires_at: 2000 };
    // Active: 75% reduction (25% of base)
    assert_eq!(s.apply_penalty(100.0, 1500), 25.0);
    // Expired: full value
    assert_eq!(s.apply_penalty(100.0, 2500), 100.0);
}

#[test]
fn should_apply_sickness_checks() {
    assert!(should_apply_sickness(ResurrectSource::SpiritHealer, 60));
    assert!(!should_apply_sickness(ResurrectSource::CorpseRun, 60));
    assert!(!should_apply_sickness(ResurrectSource::PlayerSpell, 60));
    assert!(!should_apply_sickness(ResurrectSource::SpiritHealer, 5));
}

// --- Spirit healer ---

fn healer_ctx(
    state: DeathState,
    level: u8,
    player: (f32, f32, f32),
    healer: (f32, f32, f32),
) -> SpiritHealerContext {
    SpiritHealerContext {
        state,
        player_level: level,
        player_pos: player,
        healer_pos: healer,
        healer_map: 0,
        now: 1000,
    }
}

#[test]
fn spirit_healer_resurrects_ghost() {
    let ctx = healer_ctx(DeathState::Ghost, 60, (10.0, 10.0, 0.0), (15.0, 10.0, 0.0));
    let result = accept_spirit_healer(&ctx).unwrap();
    assert_eq!(result.x, 15.0);
    assert!(result.sickness.is_some());
    assert_eq!(
        result.sickness.unwrap().expires_at,
        1000 + RES_SICKNESS_DURATION
    );
}

#[test]
fn spirit_healer_low_level_no_sickness() {
    let ctx = healer_ctx(DeathState::Ghost, 5, (10.0, 10.0, 0.0), (15.0, 10.0, 0.0));
    assert!(accept_spirit_healer(&ctx).unwrap().sickness.is_none());
}

#[test]
fn spirit_healer_not_ghost_fails() {
    let ctx = healer_ctx(DeathState::Alive, 60, (0.0, 0.0, 0.0), (0.0, 0.0, 0.0));
    assert_eq!(accept_spirit_healer(&ctx), Err(DeathError::NotGhost));
}

#[test]
fn spirit_healer_out_of_range() {
    let ctx = healer_ctx(DeathState::Ghost, 60, (0.0, 0.0, 0.0), (100.0, 100.0, 0.0));
    assert_eq!(accept_spirit_healer(&ctx), Err(DeathError::NotGhost));
}

#[test]
fn spirit_healer_exact_range() {
    let ctx = healer_ctx(
        DeathState::Ghost,
        60,
        (0.0, 0.0, 0.0),
        (SPIRIT_HEALER_RANGE, 0.0, 0.0),
    );
    assert!(accept_spirit_healer(&ctx).is_ok());
}

// --- Durability loss ---

#[test]
fn death_durability_loss_10_percent() {
    let mut items = [
        Durability {
            current: 100,
            max: 100,
        },
        Durability {
            current: 50,
            max: 200,
        },
    ];
    let total = apply_death_durability_loss(&mut items);
    assert_eq!(items[0].current, 90); // 100 - 10% of 100
    assert_eq!(items[1].current, 30); // 50 - 10% of 200 = 50 - 20
    assert_eq!(total, 30); // 10 + 20
}

#[test]
fn spirit_healer_durability_loss_25_percent() {
    let mut items = [Durability {
        current: 100,
        max: 100,
    }];
    let total = apply_spirit_healer_durability_loss(&mut items);
    assert_eq!(items[0].current, 75);
    assert_eq!(total, 25);
}

#[test]
fn combined_death_and_spirit_healer_loss() {
    let mut items = [Durability {
        current: 100,
        max: 100,
    }];
    apply_death_durability_loss(&mut items); // 100 → 90
    apply_spirit_healer_durability_loss(&mut items); // 90 → 65
    assert_eq!(items[0].current, 65);
}

#[test]
fn durability_does_not_go_negative() {
    let mut items = [Durability {
        current: 5,
        max: 100,
    }];
    apply_death_durability_loss(&mut items); // 5 - 10 → 0
    assert_eq!(items[0].current, 0);
}

#[test]
fn broken_item() {
    let d = Durability {
        current: 0,
        max: 100,
    };
    assert!(d.is_broken());

    let d = Durability {
        current: 1,
        max: 100,
    };
    assert!(!d.is_broken());

    // No max = not breakable
    let d = Durability { current: 0, max: 0 };
    assert!(!d.is_broken());
}

#[test]
fn apply_loss_to_durability() {
    let mut d = Durability {
        current: 80,
        max: 200,
    };
    d.apply_loss(0.10);
    assert_eq!(d.current, 60); // 80 - 20
}

#[test]
fn empty_items_no_loss() {
    let mut items: [Durability; 0] = [];
    assert_eq!(apply_death_durability_loss(&mut items), 0);
}

// --- Player resurrect spells ---

#[test]
fn cast_resurrect_on_dead() {
    let result = cast_resurrect(
        DeathState::Dead,
        10000.0,
        5000.0,
        (100.0, 200.0, 50.0),
        (110.0, 200.0, 50.0),
    )
    .unwrap();
    assert_eq!(result.restored_hp, 3500.0);
    assert_eq!(result.restored_mana, 1750.0);
    assert_eq!(result.x, 100.0);
}

#[test]
fn cast_resurrect_on_ghost() {
    let result = cast_resurrect(
        DeathState::Ghost,
        8000.0,
        4000.0,
        (50.0, 50.0, 0.0),
        (55.0, 50.0, 0.0),
    );
    assert!(result.is_ok());
}

#[test]
fn cast_resurrect_on_alive_fails() {
    assert_eq!(
        cast_resurrect(
            DeathState::Alive,
            10000.0,
            5000.0,
            (0.0, 0.0, 0.0),
            (0.0, 0.0, 0.0)
        ),
        Err(DeathError::AlreadyAlive)
    );
}

#[test]
fn cast_resurrect_out_of_range() {
    assert_eq!(
        cast_resurrect(
            DeathState::Dead,
            10000.0,
            5000.0,
            (0.0, 0.0, 0.0),
            (500.0, 500.0, 0.0)
        ),
        Err(DeathError::NotDead)
    );
}

#[test]
fn cast_resurrect_exact_range() {
    let result = cast_resurrect(
        DeathState::Dead,
        10000.0,
        5000.0,
        (0.0, 0.0, 0.0),
        (RESURRECT_SPELL_RANGE, 0.0, 0.0),
    );
    assert!(result.is_ok());
}

#[test]
fn player_resurrect_no_sickness() {
    // Verify ResurrectSource::PlayerSpell gives no sickness
    assert!(apply_res_sickness(ResurrectSource::PlayerSpell, 60, 1000).is_none());
}

// --- Corpse despawn ---

#[test]
fn corpse_despawn_timer_starts() {
    let mut tracker = CorpseDespawnTracker::new();
    tracker.start_timer(100, 1000);
    assert!(tracker.is_pending(100));
    assert_eq!(tracker.pending_count(), 1);
}

#[test]
fn corpse_despawns_after_5_minutes() {
    let mut tracker = CorpseDespawnTracker::new();
    tracker.start_timer(100, 1000);

    // Not yet
    assert!(
        tracker
            .collect_despawns(1000 + CORPSE_DESPAWN_SECS - 1)
            .is_empty()
    );
    assert!(tracker.is_pending(100));

    // Now
    let despawned = tracker.collect_despawns(1000 + CORPSE_DESPAWN_SECS);
    assert_eq!(despawned, vec![100]);
    assert!(!tracker.is_pending(100));
}

#[test]
fn multiple_corpse_despawns() {
    let mut tracker = CorpseDespawnTracker::new();
    tracker.start_timer(100, 1000);
    tracker.start_timer(200, 1100);

    let despawned = tracker.collect_despawns(1000 + CORPSE_DESPAWN_SECS);
    assert_eq!(despawned.len(), 1);
    assert_eq!(despawned[0], 100);
    assert_eq!(tracker.pending_count(), 1);
}

#[test]
fn cancel_corpse_despawn() {
    let mut tracker = CorpseDespawnTracker::new();
    tracker.start_timer(100, 1000);
    tracker.cancel(100);
    assert!(!tracker.is_pending(100));
    assert!(tracker.collect_despawns(9999).is_empty());
}

#[test]
fn restart_timer_replaces_old() {
    let mut tracker = CorpseDespawnTracker::new();
    tracker.start_timer(100, 1000);
    tracker.start_timer(100, 2000); // dies again, new timer

    // Old timer would have fired at 1300, new at 2300
    assert!(
        tracker
            .collect_despawns(1000 + CORPSE_DESPAWN_SECS)
            .is_empty()
    );
    let despawned = tracker.collect_despawns(2000 + CORPSE_DESPAWN_SECS);
    assert_eq!(despawned, vec![100]);
}
