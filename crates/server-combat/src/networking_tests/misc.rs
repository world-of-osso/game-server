use super::*;
#[test]
fn chat_message_sender_override() {
    let mut msg = ChatMessage {
        sender: "hacker".into(),
        content: "Hello".into(),
        channel: ChatType::Say,
    };
    msg.sender = "TrustedPlayer".to_string();
    assert_eq!(msg.sender, "TrustedPlayer");
    assert_eq!(msg.content, "Hello");
}

#[test]
fn set_target_updates_combat_target() {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);

    let target_entity = app.world_mut().spawn_empty().id();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    app.world_mut()
        .entity_mut(player)
        .insert(CombatTarget::default());

    let bits = target_entity.to_bits();
    let resolved = Entity::from_bits(bits);
    app.world_mut()
        .entity_mut(player)
        .get_mut::<CombatTarget>()
        .unwrap()
        .0 = Some(resolved);

    let ct = app.world().entity(player).get::<CombatTarget>().unwrap();
    assert_eq!(ct.0, Some(target_entity));
}

#[test]
fn known_elwynn_position_maps_to_existing_adt_tile() {
    assert_eq!(wow_to_tile(-8949.0, -132.0), (32, 48));
}

#[test]
fn deferred_player_despawn_ignores_entities_already_despawned() {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);

    let player = app.world_mut().spawn((ReplicationTag, VisibilityTag)).id();

    app.add_systems(Update, move |mut commands: Commands| {
        commands.entity(player).despawn();
        queue_player_despawn(&mut commands, player);
    });

    let result = catch_unwind(AssertUnwindSafe(|| app.update()));
    assert!(
        result.is_ok(),
        "queue_player_despawn should ignore entities already despawned before cleanup flushes"
    );
}
