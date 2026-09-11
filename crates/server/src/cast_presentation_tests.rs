use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use bevy::prelude::*;
use lightyear::connection::host::HostClient;
use lightyear::prelude::*;
use shared::casting::CastState;
use shared::components::{Health, MovementSpeed, Player, Position, SpellSchool};
use shared::protocol::{CombatChannel, ProtocolPlugin, SpellCastIntent, StopSpellCast};
use shared::spell_data::{SpellData, SpellTarget};
use std::time::Duration;

fn fixture() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    app.add_plugins(crate::combat::CombatPlugin);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(50),
    ));
    app.insert_resource(crate::spell_db::SpellDatabase::fixture(vec![SpellData {
        id: 133,
        name: "Presentation cast".into(),
        school: SpellSchool::Fire,
        cast_time: 1.0,
        cooldown: 0.0,
        cost: None,
        target: SpellTarget::Self_,
        range: 30.0,
        cast_while_moving: false,
        interruptible: true,
        effects: [None; 3],
    }]));
    app.finish();
    app.cleanup();
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Caster".into(),
                race: 1,
                class: 1,
                appearance: default(),
            },
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            MovementSpeed(0.0),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: 1 },
            InWorld { character_id: 1 },
            PlayerLink(player),
            MessageSender::<SpellCastIntent>::default(),
            MessageReceiver::<SpellCastIntent>::default(),
            MessageSender::<StopSpellCast>::default(),
            MessageReceiver::<StopSpellCast>::default(),
        ))
        .id();
    (app, player, connection)
}
fn advance(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}
fn start(app: &mut App, connection: Entity, id: u32) {
    app.world_mut()
        .get_mut::<MessageSender<SpellCastIntent>>(connection)
        .unwrap()
        .send::<CombatChannel>(SpellCastIntent {
            spell_id: Some(id),
            spell: String::new(),
            target_entity: None,
        });
    advance(app, 3);
}
#[test]
fn cast_presentation_receiver_starts_advances_and_expires_without_effects() {
    let (mut app, player, connection) = fixture();
    start(&mut app, connection, 133);
    let cast = app
        .world()
        .get::<CastState>(player)
        .expect("valid receiver request starts cast");
    assert_eq!(cast.spell_id, 133);
    assert!(cast.elapsed < cast.duration);
    let before = cast.elapsed;
    advance(&mut app, 3);
    assert!(app.world().get::<CastState>(player).unwrap().elapsed > before);
    advance(&mut app, 30);
    assert!(app.world().get::<CastState>(player).is_none());
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 100.0);
}
#[test]
fn cast_presentation_stop_and_movement_cancel_owner_cast() {
    let (mut app, player, connection) = fixture();
    start(&mut app, connection, 133);
    assert!(app.world().get::<CastState>(player).is_some());
    app.world_mut()
        .get_mut::<MessageSender<StopSpellCast>>(connection)
        .unwrap()
        .send::<CombatChannel>(StopSpellCast);
    advance(&mut app, 3);
    assert!(app.world().get::<CastState>(player).is_none());
    start(&mut app, connection, 133);
    app.world_mut().get_mut::<MovementSpeed>(player).unwrap().0 = 7.0;
    advance(&mut app, 2);
    assert!(app.world().get::<CastState>(player).is_none());
}
#[test]
fn cast_presentation_rejects_unknown_dead_moving_and_unowned_requests() {
    let (mut app, player, connection) = fixture();
    start(&mut app, connection, 999999);
    assert!(app.world().get::<CastState>(player).is_none());
    app.world_mut().get_mut::<Health>(player).unwrap().current = 0.0;
    start(&mut app, connection, 133);
    assert!(app.world().get::<CastState>(player).is_none());
    app.world_mut().get_mut::<Health>(player).unwrap().current = 100.0;
    app.world_mut().get_mut::<MovementSpeed>(player).unwrap().0 = 7.0;
    start(&mut app, connection, 133);
    assert!(app.world().get::<CastState>(player).is_none());
    app.world_mut().get_mut::<MovementSpeed>(player).unwrap().0 = 0.0;
    app.world_mut()
        .entity_mut(connection)
        .remove::<PlayerLink>();
    start(&mut app, connection, 133);
    assert!(app.world().get::<CastState>(player).is_none());
}
