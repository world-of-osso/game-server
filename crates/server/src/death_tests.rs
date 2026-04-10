use super::*;
use shared::death::DeathResult;

fn death_result_at(x: f32, y: f32, z: f32) -> DeathResult {
    on_death(1, WORLD_MAP_ID, x, y, z, 5)
}

#[test]
fn build_graveyard_registry_uses_spirit_healer_spawns() {
    let world_data = WorldData {
        templates: [(
            5,
            crate::world_data::CreatureTemplate {
                entry: 5,
                min_level: 1,
                max_level: 1,
                faction: 0,
                npc_flags: shared::death::NPC_FLAG_SPIRIT_HEALER,
                speed_walk: 1.0,
                health_modifier: 1.0,
                models: vec![],
            },
        )]
        .into_iter()
        .collect(),
        spawns: vec![crate::world_data::CreatureSpawn {
            guid: 44,
            template_entry: 5,
            map: 0,
            zone_id: 12,
            position_x: -9000.0,
            position_y: 100.0,
            position_z: 50.0,
            wander_distance: 0.0,
            movement_type: 0,
        }],
        spawns_by_map: [(0, vec![0])].into_iter().collect(),
        waypoints: default(),
    };

    let registry = build_graveyard_registry(&world_data);

    assert_eq!(registry.len(), 1);
}

#[test]
fn lookup_corpse_graveyard_returns_nearest_match_for_death_location() {
    let corpse = death_result_at(90.0, 5.0, 0.0).corpse;
    let mut graveyards = GraveyardRegistry::new();
    graveyards.add(Graveyard {
        id: 1,
        zone_id: 12,
        faction: 0,
        map_id: WORLD_MAP_ID,
        x: 10.0,
        y: 0.0,
        z: 0.0,
    });
    graveyards.add(Graveyard {
        id: 2,
        zone_id: 12,
        faction: 0,
        map_id: WORLD_MAP_ID,
        x: 100.0,
        y: 0.0,
        z: 0.0,
    });

    let graveyard = lookup_corpse_graveyard(&corpse, 1, &graveyards).expect("nearest graveyard");

    assert_eq!(graveyard.id, 2);
    assert_eq!(graveyard.x, 100.0);
}

#[test]
fn release_spirit_uses_precomputed_graveyard_lookup() {
    let mut position = Position {
        x: 90.0,
        y: 5.0,
        z: 0.0,
    };
    let graveyard = Graveyard {
        id: 2,
        zone_id: 12,
        faction: 0,
        map_id: WORLD_MAP_ID,
        x: 100.0,
        y: 0.0,
        z: 0.0,
    };
    let mut death_state = PlayerDeathState {
        state: DeathState::Dead,
        corpse: Some(death_result_at(90.0, 5.0, 0.0).corpse),
        graveyard: Some(graveyard.clone()),
    };

    let message = release_spirit_for_player(
        &mut position,
        &mut death_state,
        1,
        &GraveyardRegistry::new(),
    )
    .expect("release spirit");

    assert_eq!(message, "released spirit");
    assert_eq!(death_state.state, DeathState::Ghost);
    assert_eq!(death_state.graveyard, Some(graveyard));
    assert_eq!(position.x, 100.0);
    assert_eq!(position.y, 0.0);
    assert_eq!(position.z, 0.0);
}

#[test]
fn corpse_resurrection_clears_death_state() {
    let mut position = Position {
        x: 10.0,
        y: 0.0,
        z: 10.0,
    };
    let mut health = Health {
        current: 0.0,
        max: 100.0,
    };
    let mut death_state = PlayerDeathState {
        state: DeathState::Ghost,
        corpse: Some(Corpse {
            player: 1,
            map_id: 0,
            x: 10.0,
            y: 0.0,
            z: 10.0,
            created_at: 5,
        }),
        graveyard: Some(Graveyard {
            id: 2,
            zone_id: 12,
            faction: 0,
            map_id: 0,
            x: 50.0,
            y: 0.0,
            z: 50.0,
        }),
    };

    let message = resurrect_player_at_corpse(&mut position, &mut health, None, &mut death_state)
        .expect("corpse resurrection");

    assert_eq!(message, "resurrected at corpse");
    assert_eq!(death_state.state, DeathState::Alive);
    assert!(death_state.corpse.is_none());
    assert!(health.current > 0.0);
}

#[test]
fn snapshot_flags_spirit_healer_proximity() {
    let position = Position {
        x: 5.0,
        y: 0.0,
        z: 0.0,
    };
    let death_state = PlayerDeathState {
        state: DeathState::Ghost,
        corpse: None,
        graveyard: Some(Graveyard {
            id: 1,
            zone_id: 12,
            faction: 0,
            map_id: 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }),
    };

    let snapshot = build_snapshot(&position, &death_state);

    assert_eq!(snapshot.state, DeathStateSnapshot::Ghost);
    assert!(snapshot.spirit_healer_available);
}

#[test]
fn stuck_escape_teleports_alive_player_to_nearest_graveyard() {
    let mut position = Position {
        x: 90.0,
        y: 5.0,
        z: 0.0,
    };
    let mut graveyards = GraveyardRegistry::new();
    graveyards.add(Graveyard {
        id: 1,
        zone_id: 12,
        faction: 0,
        map_id: WORLD_MAP_ID,
        x: 10.0,
        y: 0.0,
        z: 0.0,
    });
    graveyards.add(Graveyard {
        id: 2,
        zone_id: 12,
        faction: 0,
        map_id: WORLD_MAP_ID,
        x: 100.0,
        y: 0.0,
        z: 0.0,
    });

    let message = use_stuck_escape(&mut position, None, 1, &graveyards).expect("stuck escape");

    assert_eq!(message, "stuck escape used");
    assert_eq!(position.x, 100.0);
    assert_eq!(position.y, 0.0);
    assert_eq!(position.z, 0.0);
}

#[test]
fn stuck_escape_rejects_dead_players() {
    let mut position = Position {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };
    let death_state = PlayerDeathState {
        state: DeathState::Ghost,
        corpse: None,
        graveyard: None,
    };

    let error = use_stuck_escape(
        &mut position,
        Some(&death_state),
        1,
        &GraveyardRegistry::new(),
    )
    .expect_err("dead players cannot use stuck escape");

    assert_eq!(error, "stuck escape is unavailable while dead");
}
