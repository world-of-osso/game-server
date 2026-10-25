use super::*;

#[test]
fn lethal_fall_transitions_death_in_same_fixed_step() {
    let ground_y = 50.0;
    let landing_position = Position {
        x: 90.0,
        y: ground_y,
        z: -90.0,
    };
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(crate::death::DeathPlugin);
    app.add_systems(FixedUpdate, apply_terrain_gravity);
    app.insert_resource(crate::terrain_height::test_helpers::flat_ground(
        100.0, 100.0, ground_y,
    ));
    let player = app
        .world_mut()
        .spawn((
            crate::maps::InMap::continent(0),
            Player {
                name: "Elara".into(),
                race: 1,
                class: 2,
                appearance: Default::default(),
            },
            landing_position,
            VerticalVelocity::default(),
            FallTracker {
                airborne: true,
                peak_y: ground_y + 70.0,
                ..FallTracker::grounded(ground_y)
            },
            Health {
                current: 5.0,
                max: 100.0,
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

    app.world_mut().run_schedule(FixedUpdate);

    let player_entity = app.world().entity(player);
    assert_eq!(player_entity.get::<Health>().unwrap().current, 0.0);
    let death_state = player_entity
        .get::<crate::death::PlayerDeathState>()
        .expect("lethal fall must transition the player in the same fixed step");
    assert_eq!(death_state.state, shared::death::DeathState::Dead);
    let corpse = death_state
        .corpse
        .as_ref()
        .expect("lethal fall must create a corpse");
    assert_eq!(corpse.player, 1);
    assert_eq!(corpse.x, landing_position.x);
    assert_eq!(corpse.y, landing_position.y);
    assert_eq!(corpse.z, landing_position.z);
    assert_eq!(
        player_entity
            .get::<crate::durability::EquippedDurability>()
            .unwrap()
            .slots[0]
            .current,
        63
    );

    app.world_mut().run_schedule(FixedUpdate);

    assert_eq!(
        app.world()
            .entity(player)
            .get::<crate::durability::EquippedDurability>()
            .unwrap()
            .slots[0]
            .current,
        63,
        "death durability loss must not repeat on later fixed steps"
    );
}

/// A 30 yd fall the player survives logs ENVIRONMENTAL_DAMAGE Falling to its owner: no
/// source, the damage dealt, no overkill, stamped with the server's wall clock.
#[test]
fn a_survived_fall_logs_falling_environmental_damage_to_the_owner() {
    use lightyear::connection::host::HostClient;
    use lightyear::prelude::{Connected, MessageReceiver, MessageSender, PeerId, RemoteId};
    use shared::protocol::{CombatLogEvent, CombatLogKind, EnvironmentalKind};

    let ground_y = 50.0;
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: std::time::Duration::from_millis(50),
    });
    app.add_plugins(shared::protocol::ProtocolPlugin);
    app.add_message::<crate::death::PlayerDeathTransition>();
    app.add_systems(FixedUpdate, apply_terrain_gravity);
    app.insert_resource(crate::terrain_height::test_helpers::flat_ground(
        100.0, 100.0, ground_y,
    ));
    app.finish();
    app.cleanup();
    let player = spawn_player(app.world_mut(), 90.0, ground_y, -90.0);
    app.world_mut().entity_mut(player).insert((
        shared::components::MovementControl::default(),
        FallTracker {
            airborne: true,
            peak_y: ground_y + 30.0,
            ..FallTracker::grounded(ground_y)
        },
    ));
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            PlayerLink(player),
            crate::auth::Authenticated { account_id: 1 },
            MessageSender::<CombatLogEvent>::default(),
            MessageReceiver::<CombatLogEvent>::default(),
        ))
        .id();
    let before_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(Last);

    let log: Vec<CombatLogEvent> = app
        .world_mut()
        .get_mut::<MessageReceiver<CombatLogEvent>>(connection)
        .unwrap()
        .receive()
        .collect();
    let damage = 100.0 * shared::movement::fall_damage_percent(30.0);
    assert!(damage >= 1.0, "a 30 yd fall hurts");
    assert_eq!(log.len(), 1, "{log:?}");
    assert_eq!(
        log[0].kind,
        CombatLogKind::Environmental(EnvironmentalKind::Falling)
    );
    assert_eq!(
        (log[0].source, log[0].target, log[0].spell_id),
        (None, Some(player.to_bits()), None)
    );
    assert_eq!((log[0].amount, log[0].overflow), (damage as i32, 0));
    assert!(log[0].timestamp_unix_ms >= before_ms);
    let health = app.world().get::<Health>(player).unwrap().current;
    assert_eq!(health, 100.0 - damage);
}
