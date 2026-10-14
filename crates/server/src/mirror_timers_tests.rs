use std::path::PathBuf;
use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::{Connected, MessageReceiver, PeerId, RemoteId};
use shared::components::MovementControl;

use super::*;

const TICK_MS: i32 = 50;
const UNDER: Exposure = Exposure {
    under_water: true,
    dark_water: false,
};
const DRY: Exposure = Exposure {
    under_water: false,
    dark_water: false,
};

fn breath_start(value_ms: i32, scale: f32) -> TimerMessage {
    TimerMessage::Start(MirrorTimerStart {
        timer: MIRROR_TIMER_BREATH,
        value_ms,
        max_value_ms: BREATH_MAX_MS,
        scale,
        paused: false,
        spell_id: 0,
    })
}

fn breath_stop() -> TimerMessage {
    TimerMessage::Stop(MirrorTimerStop {
        timer: MIRROR_TIMER_BREATH,
    })
}

/// `ticks` fixed ticks of `exposure`; the tick index and outcome of every tick that sent or
/// dealt something.
fn run(
    timers: &mut MirrorTimers,
    exposure: Exposure,
    breathing: Breathing,
    ticks: usize,
) -> Vec<(usize, DrowningTick)> {
    (0..ticks)
        .map(|index| (index, handle_drowning(timers, exposure, breathing, TICK_MS)))
        .filter(|(_, tick)| *tick != DrowningTick::default())
        .collect()
}

#[test]
fn breath_starts_full_when_the_head_goes_under_and_drains_without_resending() {
    let mut timers = MirrorTimers::default();

    let sent = run(&mut timers, UNDER, Breathing::default(), 20 * 30);

    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0].1.messages, vec![breath_start(BREATH_MAX_MS, -1.0)]);
    // The activation tick does not count down; the 599 after it do.
    assert_eq!(timers.value[1], BREATH_MAX_MS - 599 * TICK_MS);
}

#[test]
fn surfacing_refills_at_ten_times_and_stops_when_full() {
    let mut timers = MirrorTimers::default();
    run(&mut timers, UNDER, Breathing::default(), 1 + 20 * 30);
    let under = timers.value[1];
    assert_eq!(under, 150_000);

    let sent = run(&mut timers, DRY, Breathing::default(), 20 * 5);

    assert_eq!(
        sent[0],
        (
            0,
            DrowningTick {
                messages: vec![breath_start(under + 10 * TICK_MS, 10.0)],
                damage: Vec::new(),
            }
        )
    );
    // 30 s of breath back at 10 ms per ms: 3 s, the 60th tick.
    assert_eq!(sent[1].0, 59);
    assert_eq!(sent[1].1.messages, vec![breath_stop()]);
    assert_eq!(sent.len(), 2);
    assert_eq!(timers.value[1], DISABLED_MIRROR_TIMER);
    assert!(run(&mut timers, DRY, Breathing::default(), 20).is_empty());
}

#[test]
fn diving_again_while_refilling_resends_the_draining_bar_from_its_value() {
    let mut timers = MirrorTimers::default();
    run(&mut timers, UNDER, Breathing::default(), 1 + 20 * 30);
    run(&mut timers, DRY, Breathing::default(), 20);
    let refilled = timers.value[1];
    assert_eq!(refilled, 160_000);

    let sent = run(&mut timers, UNDER, Breathing::default(), 20);

    assert_eq!(sent.len(), 1);
    assert_eq!(
        sent[0].1.messages,
        vec![breath_start(refilled - TICK_MS, -1.0)]
    );
}

#[test]
fn out_of_breath_drowns_every_second() {
    let mut timers = MirrorTimers::default();

    let sent = run(&mut timers, UNDER, Breathing::default(), 1 + 20 * 185);

    let drowned: Vec<usize> = sent
        .iter()
        .filter(|(_, tick)| tick.damage == vec![EnvironmentalKind::Drowning])
        .map(|(index, _)| *index)
        .collect();
    // Breath runs out 180 s after the activation tick, then `+= 1 * IN_MILLISECONDS`.
    assert_eq!(drowned, vec![3601, 3621, 3641, 3661, 3681]);
    assert!(
        sent.iter()
            .skip(1)
            .all(|(_, tick)| tick.messages.is_empty())
    );
}

#[test]
fn drowning_and_exhaustion_deal_a_fifth_of_maximum_health_plus_a_small_roll() {
    assert_eq!(
        environmental_damage(1_000, |max| {
            assert_eq!(max, 10);
            7
        }),
        207
    );
    assert_eq!(environmental_damage(100, |max| max), 21);
    assert_eq!(environmental_damage(4, |max| max), 1);
}

#[test]
fn water_breathing_keeps_the_breath_bar_off_and_stops_a_running_one() {
    let breathing = Breathing {
        water_breathing: true,
        ..Breathing::default()
    };
    let mut fresh = MirrorTimers::default();
    assert!(run(&mut fresh, UNDER, breathing, 20 * 200).is_empty());

    let mut diving = MirrorTimers::default();
    run(&mut diving, UNDER, Breathing::default(), 20);
    let sent = run(&mut diving, UNDER, breathing, 20);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].1.messages, vec![breath_stop()]);
    assert_eq!(diving.value[1], DISABLED_MIRROR_TIMER);
}

#[test]
fn breath_multiplier_scales_the_breath_bar() {
    let breathing = Breathing {
        breath_multiplier: 2.0,
        ..Breathing::default()
    };
    let mut timers = MirrorTimers::default();
    let sent = run(&mut timers, UNDER, breathing, 1);
    let TimerMessage::Start(start) = sent[0].1.messages[0] else {
        panic!("{sent:?}");
    };
    assert_eq!((start.value_ms, start.max_value_ms), (360_000, 360_000));
}

#[test]
fn deep_water_starts_fatigue_and_exhausts_after_a_minute() {
    let deep = Exposure {
        under_water: false,
        dark_water: true,
    };
    let mut timers = MirrorTimers::default();

    let sent = run(&mut timers, deep, Breathing::default(), 1 + 20 * 62);

    assert_eq!(
        sent[0].1.messages,
        vec![TimerMessage::Start(MirrorTimerStart {
            timer: MIRROR_TIMER_FATIGUE,
            value_ms: FATIGUE_MAX_MS,
            max_value_ms: FATIGUE_MAX_MS,
            scale: -1.0,
            paused: false,
            spell_id: 0,
        })]
    );
    let exhausted: Vec<usize> = sent
        .iter()
        .filter(|(_, tick)| tick.damage == vec![EnvironmentalKind::Fatigue])
        .map(|(index, _)| *index)
        .collect();
    assert_eq!(exhausted, vec![1201, 1221]);
}

#[test]
fn death_stops_every_running_timer() {
    let mut timers = MirrorTimers::default();
    let both = Exposure {
        under_water: true,
        dark_water: true,
    };
    run(&mut timers, both, Breathing::default(), 5);

    let stopped = stop_mirror_timers(&mut timers);

    assert_eq!(
        stopped,
        vec![
            TimerMessage::Stop(MirrorTimerStop {
                timer: MIRROR_TIMER_FATIGUE
            }),
            breath_stop(),
        ]
    );
    assert_eq!(timers, MirrorTimers::default());
    assert!(stop_mirror_timers(&mut timers).is_empty());
}

// -- Real terrain: the Godot swimming fixtures' deep-water spot ---------------------------

/// `world/maps/azeroth/azeroth.wdt`.
const AZEROTH_WDT_FDID: u32 = 775_971;
/// Azeroth 32,48 (Northshire lake), WoW X/Y: seabed 140.04, surface 143.99.
const SWIM_SPOT: (f32, f32) = (-8558.0, -490.0);
const SEABED: f32 = 140.04;
const SURFACE: f32 = 143.99;

fn client_ground() -> GroundMap {
    GroundMap::new(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data"),
        std::collections::HashMap::from([(0, AZEROTH_WDT_FDID)]),
    )
}

fn swim_spot(feet_y: f32) -> Vec3 {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(SWIM_SPOT.0, SWIM_SPOT.1, feet_y);
    Vec3::new(x, y, z)
}

#[test]
fn head_under_the_surface_is_under_water_and_floating_is_not() {
    let mut ground = client_ground();
    let exposure = |ground: &mut GroundMap, feet| exposure_at(ground, 0, swim_spot(feet));

    assert_eq!(exposure(&mut ground, SEABED), UNDER);
    // The Godot client floats with the feet 1.25 under the surface: the head is out.
    assert_eq!(exposure(&mut ground, SURFACE - 1.25), DRY);
    assert_eq!(
        exposure(&mut ground, SURFACE - DEFAULT_COLLISION_HEIGHT - 0.05),
        UNDER
    );
    assert_eq!(
        exposure(&mut ground, SURFACE - DEFAULT_COLLISION_HEIGHT + 0.05),
        DRY
    );
}

fn drowning_app(feet_y: f32) -> (App, Entity, Entity) {
    drowning_app_with(feet_y, |_| {})
}

fn drowning_app_with(feet_y: f32, extra: impl FnOnce(&mut App)) -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(shared::protocol::ProtocolPlugin);
    app.add_message::<crate::death::PlayerDeathTransition>();
    app.insert_resource(client_ground());
    app.add_systems(FixedUpdate, update_mirror_timers);
    extra(&mut app);
    app.finish();
    app.cleanup();
    let feet = swim_spot(feet_y);
    let player = app
        .world_mut()
        .spawn((
            Position {
                x: feet.x,
                y: feet.y,
                z: feet.z,
            },
            Health {
                current: 1_000.0,
                max: 1_000.0,
            },
            MirrorTimers::default(),
            MovementControl::default(),
            InMap {
                map_id: 0,
                instance_id: 1,
            },
            Player {
                name: "Diver".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            PlayerLink(player),
            MessageSender::<MirrorTimerStart>::default(),
            MessageReceiver::<MirrorTimerStart>::default(),
            MessageSender::<MirrorTimerStop>::default(),
            MessageReceiver::<MirrorTimerStop>::default(),
        ))
        .id();
    (app, player, connection)
}

fn tick(app: &mut App) {
    let mut time = Time::<()>::default();
    time.advance_by(Duration::from_millis(50));
    app.insert_resource(time);
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(Last);
}

fn received<M: lightyear::prelude::Message>(app: &mut App, connection: Entity) -> Vec<M> {
    app.world_mut()
        .get_mut::<MessageReceiver<M>>(connection)
        .unwrap()
        .receive()
        .collect()
}

/// `ticks` fixed ticks; the starts and stops the owner received over them (a receiver
/// holds only the last tick's messages).
fn run_ticks(
    app: &mut App,
    connection: Entity,
    ticks: usize,
) -> (Vec<MirrorTimerStart>, Vec<MirrorTimerStop>) {
    let (mut starts, mut stops) = (Vec::new(), Vec::new());
    for _ in 0..ticks {
        tick(app);
        starts.extend(received::<MirrorTimerStart>(app, connection));
        stops.extend(received::<MirrorTimerStop>(app, connection));
    }
    (starts, stops)
}

#[test]
fn achievements_criteria_drowning_after_a_creature_wound_never_invents_a_creature_killer() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState, load_achievement_catalog};
    use crate::auth::{Authenticated, InWorld};
    use crate::persistence::{self, ClientId, GameDatabase};
    let (mut app, player, connection) = drowning_app_with(SEABED, |app| {
        app.add_plugins((AchievementPlugin, crate::death::DeathPlugin));
    });
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    app.insert_resource(load_achievement_catalog(&conn).unwrap());
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "Woundeddiver", 1, 1).unwrap();
    app.insert_resource(GameDatabase::new(db));
    app.world_mut()
        .entity_mut(player)
        .insert((ClientId(character), PlayerAchievementState::default()));
    app.world_mut().entity_mut(connection).insert((
        Authenticated { account_id: 1 },
        InWorld {
            character_id: character,
        },
    ));
    let npc = app
        .world_mut()
        .spawn((
            shared::components::Npc {
                template_id: 448,
                name: "Hogger".into(),
            },
            Health {
                current: 1000.0,
                max: 1000.0,
            },
            InMap {
                map_id: 0,
                instance_id: 1,
            },
        ))
        .id();
    crate::combat::deal_direct_damage(app.world_mut(), npc, player, 100.0);
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 900.0);
    assert_eq!(
        app.world()
            .get::<crate::combat::LastDamager>(player)
            .unwrap()
            .0,
        npc
    );
    run_ticks(&mut app, connection, 20 * 186);
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 0.0);
    assert_eq!(
        app.world()
            .get::<crate::death::PlayerDeathState>(player)
            .unwrap()
            .state,
        shared::death::DeathState::Dead
    );
    let state = app.world().get::<PlayerAchievementState>(player).unwrap();
    assert_eq!(state.criteria_progress.get(&111), Some(&1));
    assert_eq!(
        state.criteria_progress.get(&147),
        Some(&1),
        "native drowning type26"
    );
    assert!(
        !state.criteria_progress.contains_key(&455),
        "a previous nonlethal attacker did not cause drowning"
    );
    assert!(!state.criteria_progress.contains_key(&5594));
    drop(app);
    let db = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&db, character);
    assert_eq!(saved.criteria_progress.get(&111), Some(&1));
    assert_eq!(saved.criteria_progress.get(&147), Some(&1));
    assert!(!saved.criteria_progress.contains_key(&455));
}

#[test]
fn a_player_on_the_seabed_gets_the_breath_bar_and_drowns_at_zero() {
    let (mut app, player, connection) = drowning_app(SEABED);

    let (starts, stops) = run_ticks(&mut app, connection, 20 * 180 + 2);

    assert_eq!(
        starts,
        vec![MirrorTimerStart {
            timer: MIRROR_TIMER_BREATH,
            value_ms: BREATH_MAX_MS,
            max_value_ms: BREATH_MAX_MS,
            scale: -1.0,
            paused: false,
            spell_id: 0,
        }]
    );
    assert!(stops.is_empty());
    let health = app.world().get::<Health>(player).unwrap().current;
    // One drowning hit of 200 + urand(0, 10) at 180 s.
    assert!((790.0..=800.0).contains(&health), "health {health}");

    // Surfacing refills the bar.
    app.world_mut().get_mut::<Position>(player).unwrap().y = SURFACE - 1.25;
    let (refill, _) = run_ticks(&mut app, connection, 1);
    assert_eq!(refill.len(), 1);
    assert_eq!(refill[0].scale, 10.0);
}

#[test]
fn death_under_water_stops_the_breath_bar() {
    let (mut app, player, connection) = drowning_app(SEABED);
    let (starts, _) = run_ticks(&mut app, connection, 1);
    assert_eq!(starts.len(), 1);

    app.world_mut().get_mut::<Health>(player).unwrap().current = 0.0;
    let (starts, stops) = run_ticks(&mut app, connection, 20);

    assert_eq!(
        stops,
        vec![MirrorTimerStop {
            timer: MIRROR_TIMER_BREATH
        }]
    );
    assert!(starts.is_empty());
}

#[test]
fn a_floating_swimmer_never_gets_the_breath_bar() {
    let (mut app, _, connection) = drowning_app(SURFACE - 1.25);
    let (starts, stops) = run_ticks(&mut app, connection, 40);
    assert!(starts.is_empty() && stops.is_empty());
}
