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
                last_y: ground_y,
                epoch: 0,
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
