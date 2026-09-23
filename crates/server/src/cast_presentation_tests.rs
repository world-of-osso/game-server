use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::spell_info::{
    SpellCastTime, SpellEffect, SpellInfo, SpellMisc, SpellPowerCost, SpellRange, SpellStore,
};
use bevy::prelude::*;
use lightyear::connection::host::HostClient;
use lightyear::prelude::*;
use shared::casting::CastState;
use shared::components::{Health, MovementSpeed, Player, Position};
use shared::protocol::{CombatChannel, ProtocolPlugin, SpellCastIntent, StopSpellCast};
use std::time::Duration;

/// 1 s Fire cast on self, 30 yd.
fn presentation_spell() -> SpellInfo {
    SpellInfo {
        id: 133,
        name: "Presentation cast".into(),
        misc: Some(SpellMisc {
            school_mask: 4,
            cast_time: Some(SpellCastTime {
                base_ms: 1000,
                minimum_ms: 0,
            }),
            range: Some(SpellRange {
                max_hostile: 30.0,
                max_friendly: 30.0,
                ..default()
            }),
            ..default()
        }),
        // TARGET_UNIT_CASTER
        effects: vec![SpellEffect {
            implicit_targets: [1, 0],
            ..default()
        }],
        ..default()
    }
}

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
    app.insert_resource(SpellStore::fixture(vec![presentation_spell()]));
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
    assert_eq!(cast.spell_name, "Presentation cast");
    let decoded: CastState = serde_json::from_slice(&serde_json::to_vec(cast).unwrap()).unwrap();
    assert_eq!(&decoded, cast);
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

#[test]
fn cast_presentation_stop_cannot_cancel_other_players() {
    let (mut app, player, connection) = fixture();
    let other = app
        .world_mut()
        .spawn((
            CastState::normal(133, 0, 10.0, true),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    start(&mut app, connection, 133);
    app.world_mut()
        .get_mut::<MessageSender<StopSpellCast>>(connection)
        .unwrap()
        .send::<CombatChannel>(StopSpellCast);
    advance(&mut app, 3);
    assert!(app.world().get::<CastState>(player).is_none());
    assert!(app.world().get::<CastState>(other).is_some());
    app.world_mut()
        .entity_mut(connection)
        .remove::<Authenticated>();
    start(&mut app, connection, 133);
    assert!(app.world().get::<CastState>(player).is_none());
}

#[test]
fn cast_presentation_validates_resource_cooldown_and_gcd() {
    use shared::casting::{GlobalCooldown, SpellCooldowns};
    use shared::components::Mana;
    let (mut app, player, connection) = fixture();
    let mut spell = presentation_spell();
    spell.power_costs = vec![SpellPowerCost {
        power_type: 0,
        mana_cost: 20,
        ..default()
    }];
    app.insert_resource(SpellStore::fixture(vec![spell]));
    app.world_mut().entity_mut(player).insert(Mana {
        current: 10.0,
        max: 100.0,
    });
    start(&mut app, connection, 133);
    assert!(app.world().get::<CastState>(player).is_none());
    app.world_mut().get_mut::<Mana>(player).unwrap().current = 100.0;
    let mut cooldowns = SpellCooldowns::default();
    cooldowns.start(133, 20.0);
    app.world_mut().entity_mut(player).insert(cooldowns);
    start(&mut app, connection, 133);
    assert!(app.world().get::<CastState>(player).is_none());
    app.world_mut()
        .entity_mut(player)
        .remove::<SpellCooldowns>();
    app.world_mut()
        .entity_mut(player)
        .insert(GlobalCooldown { remaining: 20.0 });
    start(&mut app, connection, 133);
    assert!(app.world().get::<CastState>(player).is_none());
    app.world_mut()
        .entity_mut(player)
        .remove::<GlobalCooldown>();
    start(&mut app, connection, 133);
    assert!(app.world().get::<CastState>(player).is_some());
    assert_eq!(app.world().get::<Mana>(player).unwrap().current, 100.0);
}

#[test]
fn cast_presentation_scales_percent_mana_cost_from_max_mana() {
    use shared::components::Mana;
    let (mut app, player, connection) = fixture();
    let mut spell = presentation_spell();
    // Fireball-style cost: 5% of base mana, no flat amount.
    spell.power_costs = vec![SpellPowerCost {
        power_type: 0,
        power_cost_pct: 5.0,
        ..default()
    }];
    app.insert_resource(SpellStore::fixture(vec![spell]));
    start(&mut app, connection, 133);
    assert!(
        app.world().get::<CastState>(player).is_none(),
        "percent cost with no mana pool must not cast"
    );
    app.world_mut().entity_mut(player).insert(Mana {
        current: 49.0,
        max: 1000.0,
    });
    start(&mut app, connection, 133);
    assert!(app.world().get::<CastState>(player).is_none());
    app.world_mut().get_mut::<Mana>(player).unwrap().current = 50.0;
    start(&mut app, connection, 133);
    assert!(app.world().get::<CastState>(player).is_some());
}
