use std::time::Duration;

use bevy::ecs::system::RunSystemOnce;
use lightyear::connection::host::HostClient;
use lightyear::prelude::{
    AppMessageExt, Connected, MessageReceiver, MessageSender, PeerId, RemoteId,
};
use shared::death::DeathResult;

use super::*;

const WORLD_MAP_ID: u16 = 0;

fn counters(app: &App) -> &DeathTransitionCounters {
    app.world().resource::<DeathTransitionCounters>()
}

fn reset_counters(app: &mut App) {
    *app.world_mut().resource_mut::<DeathTransitionCounters>() = DeathTransitionCounters::default();
}

fn death_result_at(x: f32, y: f32, z: f32) -> DeathResult {
    on_death(1, WORLD_MAP_ID, x, y, z, 5)
}

fn setup_death_app() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<DeathStateUpdate>();
    app.add_plugins(DeathPlugin);
    app.insert_resource(ReplicationDirtyQueue::default());
    app.finish();
    app.cleanup();
    app
}

#[test]
fn idle_fixed_updates_do_not_visit_healthy_players() {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(DeathPlugin);

    let players = (1..=3)
        .map(|character_id| {
            app.world_mut()
                .spawn((
                    crate::maps::InMap::continent(0),
                    Player {
                        name: format!("Player{character_id}"),
                        race: 1,
                        class: 1,
                        appearance: shared::components::CharacterAppearance::default(),
                    },
                    Position {
                        x: character_id as f32,
                        y: 0.0,
                        z: 0.0,
                    },
                    Health {
                        current: 100.0,
                        max: 100.0,
                    },
                    crate::persistence::ClientId(character_id),
                ))
                .id()
        })
        .collect::<Vec<_>>();

    app.world_mut().run_schedule(FixedUpdate);
    reset_counters(&mut app);

    for _ in 0..3 {
        app.world_mut().run_schedule(FixedUpdate);
    }

    assert_eq!(
        counters(&app).players_visited,
        0,
        "idle fixed updates must not visit healthy players"
    );
    for player in players {
        let entity = app.world().entity(player);
        assert!(entity.get::<PlayerDeathState>().is_none());
        assert_eq!(entity.get::<Health>().unwrap().current, 100.0);
    }
}

fn spawn_death_connection(app: &mut App, peer_id: u64, player: Entity) -> Entity {
    app.world_mut()
        .spawn((
            RemoteId(PeerId::Local(peer_id)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated {
                account_id: peer_id,
            },
            PlayerLink(player),
            MessageSender::<DeathStateUpdate>::default(),
            MessageReceiver::<DeathStateUpdate>::default(),
        ))
        .id()
}

fn receive_death_updates(app: &mut App, connection: Entity) -> Vec<DeathStateUpdate> {
    app.world_mut()
        .entity_mut(connection)
        .get_mut::<MessageReceiver<DeathStateUpdate>>()
        .expect("connection should have a death update receiver")
        .receive()
        .collect()
}

#[test]
fn queued_player_death_transitions_apply_once_and_route_snapshot() {
    let mut app = setup_death_app();

    let player_position = Position {
        x: 12.0,
        y: 3.0,
        z: -7.0,
    };
    let player = app
        .world_mut()
        .spawn((
            crate::maps::InMap::continent(0),
            Player {
                name: "Elara".into(),
                race: 1,
                class: 2,
                appearance: shared::components::CharacterAppearance::default(),
            },
            player_position,
            Health {
                current: -5.0,
                max: 70.0,
            },
            crate::persistence::ClientId(1),
            crate::durability::EquippedDurability {
                slots: vec![crate::durability::DurabilitySlotState {
                    slot: shared::components::EquipmentVisualSlot::Head,
                    item_id: 0,
                    current: 70,
                    max: 70,
                }],
            },
        ))
        .id();
    let unrelated_player = app
        .world_mut()
        .spawn((
            crate::maps::InMap::continent(0),
            Player {
                name: "Theron".into(),
                race: 1,
                class: 3,
                appearance: shared::components::CharacterAppearance::default(),
            },
            Position {
                x: 100.0,
                y: 0.0,
                z: 100.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            crate::persistence::ClientId(2),
        ))
        .id();
    let player_connection = spawn_death_connection(&mut app, 1, player);
    let unrelated_connection = spawn_death_connection(&mut app, 2, unrelated_player);

    app.world_mut()
        .write_message(PlayerDeathTransition { player });
    app.world_mut()
        .write_message(PlayerDeathTransition { player });
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(Last);

    let player_entity = app.world().entity(player);
    assert_eq!(player_entity.get::<Health>().unwrap().current, 0.0);
    let death_state = player_entity
        .get::<PlayerDeathState>()
        .expect("lethal transition should insert player death state");
    assert_eq!(death_state.state, DeathState::Dead);
    let corpse = death_state
        .corpse
        .as_ref()
        .expect("death should create corpse");
    assert_eq!(corpse.player, 1);
    assert_eq!(corpse.x, player_position.x);
    assert_eq!(corpse.y, player_position.y);
    assert_eq!(corpse.z, player_position.z);
    assert_eq!(
        player_entity
            .get::<crate::durability::EquippedDurability>()
            .unwrap()
            .slots[0]
            .current,
        63
    );
    assert_eq!(counters(&app).replication_marks, 1);

    let updates = receive_death_updates(&mut app, player_connection);
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].message.as_deref(), Some("you died"));
    assert_eq!(updates[0].error, None);
    let snapshot = updates[0]
        .snapshot
        .as_ref()
        .expect("death update should contain a snapshot");
    assert_eq!(snapshot.state, DeathStateSnapshot::Dead);
    let corpse_snapshot = snapshot
        .corpse
        .as_ref()
        .expect("snapshot should contain corpse");
    assert_eq!(corpse_snapshot.map_id, WORLD_MAP_ID);
    assert_eq!(corpse_snapshot.x, player_position.x);
    assert_eq!(corpse_snapshot.y, player_position.y);
    assert_eq!(corpse_snapshot.z, player_position.z);
    assert!(receive_death_updates(&mut app, unrelated_connection).is_empty());

    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(Last);

    assert_eq!(counters(&app).replication_marks, 1);
    assert!(receive_death_updates(&mut app, player_connection).is_empty());
    assert!(receive_death_updates(&mut app, unrelated_connection).is_empty());
    assert_eq!(
        app.world()
            .entity(player)
            .get::<crate::durability::EquippedDurability>()
            .unwrap()
            .slots[0]
            .current,
        63
    );
}

#[test]
fn resurrection_allows_a_second_death_transition() {
    let mut app = setup_death_app();
    let player_position = Position {
        x: 12.0,
        y: 3.0,
        z: -7.0,
    };
    let player = app
        .world_mut()
        .spawn((
            crate::maps::InMap::continent(0),
            Player {
                name: "Elara".into(),
                race: 1,
                class: 2,
                appearance: shared::components::CharacterAppearance::default(),
            },
            player_position,
            Health {
                current: 0.0,
                max: 70.0,
            },
            crate::persistence::ClientId(1),
            crate::durability::EquippedDurability {
                slots: vec![crate::durability::DurabilitySlotState {
                    slot: shared::components::EquipmentVisualSlot::Head,
                    item_id: 0,
                    current: 70,
                    max: 70,
                }],
            },
        ))
        .id();
    let connection = spawn_death_connection(&mut app, 1, player);

    app.world_mut()
        .write_message(PlayerDeathTransition { player });
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(Last);
    assert_eq!(receive_death_updates(&mut app, connection).len(), 1);
    assert_eq!(
        app.world()
            .entity(player)
            .get::<crate::durability::EquippedDurability>()
            .unwrap()
            .slots[0]
            .current,
        63
    );

    let mut graveyards = GraveyardRegistry::new();
    graveyards.add(Graveyard {
        id: 1,
        zone_id: 1,
        faction: 0,
        map_id: WORLD_MAP_ID,
        x: player_position.x,
        y: player_position.y,
        z: player_position.z,
    });
    let release = app
        .world_mut()
        .run_system_once(
            move |mut players: Query<(&mut Position, &mut PlayerDeathState)>| {
                let (mut position, mut death_state) =
                    players.single_mut().expect("test player should be unique");
                release_spirit_for_player(&mut position, &mut death_state, 1, &graveyards)
            },
        )
        .expect("release helper should run")
        .expect("player should release spirit");
    assert_eq!(release, "released spirit");

    let resurrection = app
        .world_mut()
        .run_system_once(
            |mut players: Query<(&mut Position, &mut Health, &mut PlayerDeathState)>| {
                let (mut position, mut health, mut death_state) =
                    players.single_mut().expect("test player should be unique");
                resurrect_player_at_corpse(&mut position, &mut health, None, &mut death_state, 0)
            },
        )
        .expect("resurrection helper should run")
        .expect("player should resurrect at the corpse");
    assert_eq!(resurrection, "resurrected at corpse");
    let resurrected = app.world().entity(player);
    assert!(resurrected.get::<Health>().unwrap().current > 0.0);
    assert_eq!(
        resurrected.get::<PlayerDeathState>().unwrap().state,
        DeathState::Alive
    );
    assert!(
        resurrected
            .get::<PlayerDeathState>()
            .unwrap()
            .corpse
            .is_none()
    );

    let became_lethal = app
        .world_mut()
        .entity_mut(player)
        .get_mut::<Health>()
        .map(|mut health| {
            let damage = health.max;
            apply_health_damage(&mut health, damage)
        })
        .expect("resurrected player should have health");
    assert!(became_lethal);
    reset_counters(&mut app);
    app.world_mut()
        .write_message(PlayerDeathTransition { player });
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(Last);

    let player_entity = app.world().entity(player);
    assert_eq!(counters(&app).players_visited, 1);
    assert_eq!(counters(&app).replication_marks, 1);
    assert_eq!(
        player_entity
            .get::<PlayerDeathState>()
            .expect("second death should create state")
            .state,
        DeathState::Dead
    );
    assert!(
        player_entity
            .get::<PlayerDeathState>()
            .unwrap()
            .corpse
            .is_some()
    );
    assert_eq!(
        player_entity
            .get::<crate::durability::EquippedDurability>()
            .unwrap()
            .slots[0]
            .current,
        56
    );
    let updates = receive_death_updates(&mut app, connection);
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].message.as_deref(), Some("you died"));
    assert_eq!(
        updates[0].snapshot.as_ref().unwrap().state,
        DeathStateSnapshot::Dead
    );

    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(Last);
    assert_eq!(counters(&app).replication_marks, 1);
    assert_eq!(
        app.world()
            .entity(player)
            .get::<crate::durability::EquippedDurability>()
            .unwrap()
            .slots[0]
            .current,
        56
    );
    assert!(receive_death_updates(&mut app, connection).is_empty());
}

#[test]
fn despawned_player_death_transition_is_ignored() {
    let mut app = setup_death_app();
    let player = app
        .world_mut()
        .spawn((
            crate::maps::InMap::continent(0),
            Player {
                name: "Elara".into(),
                race: 1,
                class: 2,
                appearance: shared::components::CharacterAppearance::default(),
            },
            Position {
                x: 12.0,
                y: 3.0,
                z: -7.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            crate::persistence::ClientId(1),
        ))
        .id();
    let connection = spawn_death_connection(&mut app, 1, player);
    app.world_mut()
        .write_message(PlayerDeathTransition { player });
    assert!(app.world_mut().despawn(player));

    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(Last);

    assert_eq!(counters(&app).replication_marks, 0);
    assert!(receive_death_updates(&mut app, connection).is_empty());
}

/// One spawn of a creature template carrying `npcflag`.
fn world_data_with_one_spawn(npcflag: u32) -> WorldData {
    WorldData {
        templates: [(
            5,
            crate::world_data::CreatureTemplate {
                entry: 5,
                name: "Fixture creature".into(),
                subname: String::new(),
                creature_type: 7,
                family: 0,
                type_flags: 0,
                flags_extra: 0,
                min_level: 1,
                max_level: 1,
                faction: 0,
                npc_flags: u64::from(npcflag),
                speed_walk: 1.0,
                speed_run: 1.0,
                base_attack_time_ms: 2_000,
                health_modifier: 1.0,
                unit_class: 1,
                expansion: 0,
                rank: 0,
                experience_modifier: 1.0,
                gives_experience: true,
                script_name: String::new(),
                aggro_mode: crate::creature_aggro::AggroMode::Aggressive,
                trigger: false,
                unit_flags: 0,
                models: vec![],
                tuning: None,
                armor: 0.0,
                armor_modifier: 1.0,
                damage_modifier: 1.0,
                health_scaling_expansion: 0,
                mana_modifier: 1.0,
                regenerates_power: false,
                regenerates_health: true,
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
            orientation: 0.0,
            wander_distance: 0.0,
            movement_type: 0,
            spawntimesecs: 120,
            display_id: None,
            unit_flags: None,
            spawn_difficulties: vec![0],
            equipment_id: 0,
        }],
        spawns_by_map: [(0, vec![0])].into_iter().collect(),
        waypoints: default(),
        ..Default::default()
    }
}

/// Spirit Healer (creature_template entry 6491): npcflag 16385 = GOSSIP | SPIRITHEALER 0x4000.
const SPIRIT_HEALER_6491_NPCFLAG: u32 = 16385;
/// AzerothCore UNIT_NPC_FLAG_VENDOR_REAGENT.
const VENDOR_REAGENT_NPCFLAG: u32 = 0x0800;

#[test]
fn build_graveyard_registry_uses_spirit_healer_spawns() {
    let registry = build_graveyard_registry(&world_data_with_one_spawn(SPIRIT_HEALER_6491_NPCFLAG));

    assert_eq!(registry.len(), 1);
}

#[test]
fn build_graveyard_registry_ignores_reagent_vendor_spawns() {
    let registry = build_graveyard_registry(&world_data_with_one_spawn(VENDOR_REAGENT_NPCFLAG));

    assert_eq!(registry.len(), 0);
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

    let message = resurrect_player_at_corpse(&mut position, &mut health, None, &mut death_state, 0)
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

    let message = use_stuck_escape(&mut position, None, 1, 0, &graveyards).expect("stuck escape");

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
        0,
        &GraveyardRegistry::new(),
    )
    .expect_err("dead players cannot use stuck escape");

    assert_eq!(error, "stuck escape is unavailable while dead");
}

#[test]
fn releasing_the_spirit_moves_the_ghost_to_its_graveyard_and_bumps_the_teleport_epoch() {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(shared::protocol::ProtocolPlugin);
    app.add_plugins(DeathPlugin);
    app.insert_resource(ReplicationDirtyQueue::default());
    app.finish();
    app.cleanup();
    let graveyard = Graveyard {
        id: 2,
        zone_id: 12,
        faction: 0,
        map_id: WORLD_MAP_ID,
        x: 100.0,
        y: 0.0,
        z: 0.0,
    };
    let player = app
        .world_mut()
        .spawn((
            crate::maps::InMap::continent(0),
            Player {
                name: "Ghost".into(),
                race: 1,
                class: 1,
                appearance: shared::components::CharacterAppearance::default(),
            },
            Position {
                x: 90.0,
                y: 5.0,
                z: 0.0,
            },
            Health {
                current: 0.0,
                max: 100.0,
            },
            crate::persistence::ClientId(1),
            PlayerDeathState {
                state: DeathState::Dead,
                corpse: Some(death_result_at(90.0, 5.0, 0.0).corpse),
                graveyard: Some(graveyard),
            },
        ))
        .id();
    let connection = spawn_death_connection(&mut app, 1, player);
    app.world_mut().entity_mut(connection).insert((
        MessageSender::<ReleaseSpirit>::default(),
        MessageReceiver::<ReleaseSpirit>::default(),
    ));
    app.update();
    assert_eq!(
        app.world().get::<MovementControl>(player),
        Some(&MovementControl::default()),
        "every player carries MovementControl"
    );

    app.world_mut()
        .get_mut::<MessageSender<ReleaseSpirit>>(connection)
        .unwrap()
        .send::<DeathChannel>(ReleaseSpirit);
    for _ in 0..3 {
        app.update();
    }

    let position = *app.world().get::<Position>(player).unwrap();
    assert_eq!((position.x, position.y, position.z), (100.0, 0.0, 0.0));
    assert_eq!(app.world().get::<MovementControl>(player).unwrap().epoch, 1);
}
