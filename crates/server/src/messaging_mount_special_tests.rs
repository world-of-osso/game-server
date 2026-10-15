use super::*;
use shared::components::{Mounted, PlayerMotion};

fn observer(app: &mut App, network_id: u64, name: &str, map: crate::maps::InMap) -> Entity {
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: name.into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            Position {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            map,
        ))
        .id();
    app.world_mut()
        .spawn((
            PlayerLink(player),
            Authenticated {
                account_id: network_id,
            },
            InWorld {
                character_id: network_id,
            },
            RemoteId(PeerId::Netcode(network_id)),
            Connected,
            HostClient { buffer: Vec::new() },
            MessageSender::<ChatMessage>::default(),
            MessageReceiver::<ChatMessage>::default(),
            MessageSender::<EmoteEvent>::default(),
            MessageReceiver::<EmoteEvent>::default(),
        ))
        .id()
}

fn mount_request(app: &mut App, connection: Entity) {
    app.world_mut()
        .get_mut::<MessageSender<ChatMessage>>(connection)
        .unwrap()
        .send::<ChatChannel>(ChatMessage {
            sender: "spoofed actor".into(),
            content: "/mountspecial".into(),
            channel: ChatType::Say,
        });
    for _ in 0..3 {
        app.update();
    }
}

fn events(app: &mut App, connection: Entity) -> Vec<EmoteEvent> {
    app.world_mut()
        .get_mut::<MessageReceiver<EmoteEvent>>(connection)
        .unwrap()
        .receive()
        .collect()
}

#[test]
fn mount_special_uses_authoritative_actor_and_observer_set_without_chat_or_criteria() {
    let (_file, mut app, player, connection, character) = emote_app();
    app.world_mut().entity_mut(connection).insert(InWorld {
        character_id: character,
    });
    let visible = observer(&mut app, 42, "Witness", crate::maps::InMap::continent(0));
    let hidden = observer(&mut app, 43, "Hidden", crate::maps::InMap::continent(0));
    let foreign = observer(&mut app, 44, "Foreign", crate::maps::InMap::continent(1));
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(visible);
    app.world_mut().entity_mut(player).insert((
        Mounted {
            mount_display_id: 2404,
            ..Default::default()
        },
        PlayerMotion::default(),
        Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        visibility,
    ));
    mount_request(&mut app, connection);
    let owner_events = events(&mut app, connection);
    let peer_events = events(&mut app, visible);
    assert_eq!(
        owner_events.len(),
        1,
        "one owner action, not an unknown slash chat"
    );
    assert_eq!(
        peer_events.len(),
        1,
        "visible peer observes the same action"
    );
    for event in owner_events.iter().chain(&peer_events) {
        assert_eq!(event.player_entity, player.to_bits());
        assert_eq!(event.sender, "Danceproof");
        assert_eq!(format!("{:?}", event.emote), "MountSpecial");
    }
    assert!(events(&mut app, hidden).is_empty());
    assert!(events(&mut app, foreign).is_empty());
    assert!(
        app.world()
            .resource::<ChatHistoryBuffer>()
            .entries
            .is_empty(),
        "no fabricated social line"
    );
    assert!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .is_empty()
    );
    for peer in [connection, visible, hidden, foreign] {
        assert!(
            app.world_mut()
                .get_mut::<MessageReceiver<ChatMessage>>(peer)
                .unwrap()
                .receive()
                .next()
                .is_none()
        );
    }
}

#[test]
fn mount_special_refuses_unmounted_moving_and_airborne_senders() {
    let (_file, mut app, player, connection, character) = emote_app();
    app.world_mut().entity_mut(connection).insert(InWorld {
        character_id: character,
    });
    app.world_mut().entity_mut(player).insert((
        PlayerMotion::default(),
        crate::interest::NetworkVisibility::default(),
    ));
    mount_request(&mut app, connection);
    assert!(events(&mut app, connection).is_empty());
    app.world_mut().entity_mut(player).insert(Mounted {
        mount_display_id: 2404,
        ..Default::default()
    });
    for flags in [
        PlayerMotion::FORWARD,
        PlayerMotion::STRAFE_LEFT,
        PlayerMotion::FALLING,
        PlayerMotion::FLYING,
        PlayerMotion::SWIMMING,
    ] {
        app.world_mut()
            .entity_mut(player)
            .insert(PlayerMotion(flags));
        mount_request(&mut app, connection);
        assert!(events(&mut app, connection).is_empty(), "flags {flags:#x}");
    }
}
