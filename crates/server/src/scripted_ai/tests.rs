//! The Stockade boss scripts on real world.db data (TDB1210 spawns, creature_text,
//! BroadcastText, DB2 spells), with players whose connections receive the chat and
//! encounter messages. Skipped when data/world.db is absent.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::casting::{CastState, CastType};
use shared::components::{
    CombatStatus, CreatureMotion, Health, Mana, Player, Position, Rotation, UnitFactionTemplate,
    UnitFlags, UnitLevel,
};
use shared::protocol::{
    CastFailed, ChatMessage, ChatType, CombatLogEvent, CombatLogKind, EncounterDisengageUnit,
    EncounterEnd, EncounterEngageUnit, EncounterStart, ProtocolPlugin,
};

use super::creature_text::{CreatureTexts, load_creature_texts};
use super::instance_script::{EncounterState, get_boss_state};
use super::the_stockade::*;
use super::*;
use crate::auth::{Authenticated, InWorld};
use crate::level_scaling::CreatureSpellStats;
use crate::maps::{InMap, MapStore};
use crate::spell_cast::effects::{EffectCtx, deal_damage};
use crate::spell_info::SpellStore;
use crate::world_data::{CreatureSpawn, WorldData};

/// Install an observable fixture AI without pretending an unported script is registered.
pub(crate) fn install_fixture_ai(world: &mut World, actor: Entity, ai: Box<dyn CreatureAi>) {
    world.entity_mut(actor).insert(CreatureAiState {
        ai: Some(ai),
        pending: Default::default(),
    });
}

/// `SSCreatureIds` of the bosses.
const NPC_RANDOLPH_MOLOCH: u32 = 46383;

pub(super) mod cast_admission;
pub(crate) mod data_bosses;
mod dungeonach;
mod rainfire_visibility;
mod smart_combat;
mod wailing_caverns;
const NPC_LORD_OVERHEAT: u32 = 46264;
const NPC_HOGGER: u32 = 46254;

const COPY: InMap = InMap {
    map_id: 34,
    instance_id: 1,
};
const OTHER_COPY: InMap = InMap {
    map_id: 34,
    instance_id: 2,
};
/// Spells the scripts cast, the spell Wildly Stabbing triggers, and a player
/// Fireball (133) to hit bosses with, and Eye of Divinity's native type4 detection.
const SPELLS: [u32; 13] = [
    86604, 86620, 86736, 12466, 86633, 86636, 86726, 86727, 86729, 55964, 55966, 133, 23101,
];

struct Data {
    base_mana: std::collections::HashMap<(u8, u8), u32>,
    texts: CreatureTexts,
    spell_stats: CreatureSpellStats,
    spells: Vec<crate::spell_info::SpellInfo>,
    spawns: Vec<CreatureSpawn>,
    maps: MapStore,
}

/// The five Stockade script creatures' templates.
fn stockade_templates() -> std::collections::HashMap<u32, crate::world_data::CreatureTemplate> {
    let mut templates = crate::test_data::world_data().templates.clone();
    templates.retain(|entry, _| {
        [
            NPC_HOGGER,
            NPC_LORD_OVERHEAT,
            NPC_RANDOLPH_MOLOCH,
            NPC_WARDEN_THELWATER,
            NPC_MORTIMER_MOLOCH,
        ]
        .contains(entry)
    });
    templates
}

fn world_db_path() -> PathBuf {
    crate::test_data::world_db_path()
}

fn data() -> Option<&'static Data> {
    static DATA: OnceLock<Option<Data>> = OnceLock::new();
    DATA.get_or_init(|| {
        let path = world_db_path();
        let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let base_mana = conn
            .prepare("SELECT level, class, basemana FROM creature_classlevelstats")
            .unwrap()
            .query_map([], |row| Ok(((row.get(0)?, row.get(1)?), row.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let spawns = conn
            .prepare(
                "SELECT guid, id1, zoneId, position_x, position_y, position_z, orientation, spawntimesecs
                 FROM content_creature WHERE map = 34 AND id1 IN (46254, 46264, 46383)",
            )
            .unwrap()
            .query_map([], |row| {
                Ok(CreatureSpawn {
                    guid: row.get(0)?,
                    template_entry: row.get(1)?,
                    map: 34,
                    zone_id: row.get(2)?,
                    position_x: row.get(3)?,
                    position_y: row.get(4)?,
                    position_z: row.get(5)?,
                    orientation: row.get(6)?,
                    wander_distance: 0.0,
                    movement_type: 0,
                    spawntimesecs: row.get(7)?,
                    display_id: None,
                    unit_flags: None,
                    spawn_difficulties: vec![crate::maps::store::DIFFICULTY_NORMAL],
                    equipment_id: 0,
                })
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let all = crate::spell_cast::test_harness::world_db()?;
        let spells = SPELLS
            .iter()
            .map(|&id| all.store.get(id).expect("spell in world.db").clone())
            .collect();
        Some(Data {
            base_mana,
            texts: load_creature_texts(&conn).unwrap(),
            spell_stats: crate::world_data::load_creature_spell_stats(&conn),
            spells,
            spawns,
            maps: crate::maps::store::load_map_store(&conn).unwrap(),
        })
    })
    .as_ref()
}

#[test]
fn encounter_fixture_clocks_ignore_host_delays() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let (mut h, _) = Harness::new(NPC_RANDOLPH_MOLOCH, &[]).expect("world.db fixture");
    let ticks = Arc::new(AtomicUsize::new(0));
    let observed = ticks.clone();
    h.app.add_systems(FixedUpdate, move || {
        observed.fetch_add(1, Ordering::Relaxed);
    });
    let clocks = |h: &Harness| {
        let world = h.app.world();
        [
            world.resource::<Time<Real>>().elapsed(),
            world.resource::<Time<Virtual>>().elapsed(),
            world.resource::<Time<Fixed>>().elapsed(),
            world.resource::<Time>().elapsed(),
        ]
    };
    let before = clocks(&h);
    for _ in 0..3 {
        // Exceed Bevy's default250ms wall-delta clamp between simulated ticks.
        std::thread::sleep(Duration::from_millis(300));
        h.advance(0.05);
    }
    assert_eq!(ticks.load(Ordering::Relaxed), 3);
    for (start, end) in before.into_iter().zip(clocks(&h)) {
        assert_eq!(end - start, Duration::from_millis(150));
    }
}

#[derive(Default)]
struct Inbox {
    chat: Vec<ChatMessage>,
    starts: Vec<EncounterStart>,
    ends: Vec<EncounterEnd>,
    engages: Vec<EncounterEngageUnit>,
    disengages: Vec<EncounterDisengageUnit>,
    timeline: Vec<shared::protocol::EncounterTimelineEvent>,
    log: Vec<CombatLogEvent>,
    spell_gos: Vec<shared::protocol::SpellGo>,
    opened: Vec<shared::protocol::InteractionOpened>,
    closed: Vec<shared::protocol::InteractionClosed>,
}

/// Each app owns its seed; parallel encounters never share an RNG stream.
#[derive(Resource)]
pub(super) struct EncounterSeed(pub u64);

struct Harness {
    app: App,
    /// (player, connection, inbox) per player.
    players: Vec<(Entity, Entity, Inbox)>,
}

fn stockade_app(data: &'static Data) -> App {
    stockade_app_with(data, |_| {})
}

/// `extra` adds plugins before the app is finished.
fn stockade_app_with(data: &'static Data, extra: impl FnOnce(&mut App)) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    app.add_plugins((
        crate::combat::CombatPlugin,
        crate::spell_cast::SpellCastPlugin,
        crate::auras::AuraPlugin,
        crate::creature_ai::CreatureAiPlugin,
        crate::creature_aggro::CreatureAggroPlugin,
        ScriptedAiPlugin,
        crate::achievement::AchievementPlugin,
    ));
    extra(&mut app);
    app.insert_resource(bossmod::Catalog::default());
    app.insert_resource(EncounterSeed(0));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(50),
    ));
    app.insert_resource(Time::<Fixed>::from_duration(Duration::from_millis(50)));
    app.finish();
    app.cleanup();
    app.update();
    let all = crate::spell_cast::test_harness::world_db().unwrap();
    app.insert_resource(SpellStore::fixture(data.spells.clone()));
    app.insert_resource(crate::spell_cast::test_harness::power_table(all));
    app.insert_resource(crate::faction::world_db_faction_templates().unwrap());
    app.insert_resource(WorldData {
        templates: stockade_templates(),
        class_level_base_mana: data.base_mana.clone(),
        ..Default::default()
    });
    app.insert_resource(CreatureTexts {
        groups: data.texts.groups.clone(),
        display_gender: data.texts.display_gender.clone(),
    });
    app.insert_resource(data.spell_stats.clone());
    assert_eq!(
        data.maps.instance_scripts.get(&34).map(String::as_str),
        Some(INSTANCE_THE_STOCKADE)
    );
    // Copies 1 and 2 of the Stockade, Normal, created for two solo players.
    let mut instances = crate::maps::InstanceManager::default();
    for character_id in [1001, 1002] {
        let entrant = crate::maps::instances::InstanceEntrant {
            character_id,
            group: None,
            leader: character_id,
            difficulty_id: crate::maps::store::DIFFICULTY_NORMAL,
            ..Default::default()
        };
        let now = crate::maps::instances::Now {
            elapsed: 0.0,
            unix: 0,
        };
        instances.create_map(&data.maps, 34, entrant, now).unwrap();
    }
    assert!(instances.copy_difficulty(OTHER_COPY).is_some());
    app.insert_resource(instances);
    app.insert_resource(data.maps.clone());
    app
}

/// Run the real native stat producer once, separately from encounter health buffers.
fn load_native_player_stats() -> crate::stats::CharacterStats {
    use bevy::ecs::system::RunSystemOnce;
    static STATS: OnceLock<crate::stats::CharacterStats> = OnceLock::new();
    *STATS.get_or_init(|| {
        let mut world = World::new();
        let all = crate::spell_cast::test_harness::world_db().unwrap();
        world.insert_resource(crate::spell_cast::test_harness::class_spells(all));
        let player = world
            .spawn((
                Player {
                    name: "Stockade stat inputs".into(),
                    race: 1,
                    class: 1,
                    appearance: default(),
                },
                shared::components::Level(30),
                shared::components::UnitStats::default(),
                shared::components::CombatRatings::default(),
                shared::components::DerivedStats::default(),
                Health {
                    current: 100.0,
                    max: 100.0,
                },
            ))
            .id();
        world
            .run_system_once(crate::stats::update_character_stats)
            .unwrap();
        *world
            .get::<crate::stats::CharacterStats>(player)
            .expect("native player stats produced")
    })
}

/// A level30 Human with encounter health buffer, native stats and WoW pose.
fn spawn_player(app: &mut App, map: InMap, wow: [f32; 4], id: u64) -> (Entity, Entity) {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(wow[0], wow[1], wow[2]);
    let stats = load_native_player_stats();
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: format!("Fixshout{id}"),
                race: 1,
                class: 1,
                appearance: default(),
            },
            Position { x, y, z },
            Rotation {
                x: 0.0,
                y: wow[3] + std::f32::consts::FRAC_PI_2,
                z: 0.0,
            },
            map,
            Health {
                current: 1_000_000.0,
                max: 1_000_000.0,
            },
            // Actual producer output; encounter buffers are intentionally independent.
            shared::components::Level(30),
            stats.primary,
            stats.ratings,
            stats.derived(),
            stats,
            UnitLevel(30),
            UnitFactionTemplate(1),
            CombatStatus(false),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(id)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: id },
            InWorld { character_id: id },
            crate::networking::PlayerLink(player),
            (
                MessageSender::<ChatMessage>::default(),
                MessageReceiver::<ChatMessage>::default(),
                MessageSender::<shared::protocol::EncounterTimelineEvent>::default(),
                MessageReceiver::<shared::protocol::EncounterTimelineEvent>::default(),
                MessageSender::<EncounterStart>::default(),
                MessageReceiver::<EncounterStart>::default(),
                MessageSender::<EncounterEnd>::default(),
                MessageReceiver::<EncounterEnd>::default(),
                MessageSender::<EncounterEngageUnit>::default(),
                MessageReceiver::<EncounterEngageUnit>::default(),
                MessageSender::<EncounterDisengageUnit>::default(),
                MessageReceiver::<EncounterDisengageUnit>::default(),
            ),
            (
                MessageSender::<shared::protocol::InteractNpc>::default(),
                MessageReceiver::<shared::protocol::InteractNpc>::default(),
                MessageSender::<shared::protocol::UseGameObject>::default(),
                MessageReceiver::<shared::protocol::UseGameObject>::default(),
                MessageSender::<shared::protocol::InteractionOpened>::default(),
                MessageReceiver::<shared::protocol::InteractionOpened>::default(),
                MessageSender::<shared::protocol::InteractionClosed>::default(),
                MessageReceiver::<shared::protocol::InteractionClosed>::default(),
                MessageSender::<shared::protocol::InteractionFailed>::default(),
                MessageReceiver::<shared::protocol::InteractionFailed>::default(),
            ),
            (
                MessageSender::<CombatLogEvent>::default(),
                MessageReceiver::<CombatLogEvent>::default(),
                MessageSender::<CastFailed>::default(),
                MessageReceiver::<CastFailed>::default(),
                MessageSender::<shared::protocol::SpellGo>::default(),
                MessageReceiver::<shared::protocol::SpellGo>::default(),
            ),
        ))
        .id();
    (player, connection)
}

/// The boss `entry` at its TDB1210 spawn in `map`.
fn spawn_boss(app: &mut App, data: &Data, entry: u32, map: InMap) -> Entity {
    let spawn = data
        .spawns
        .iter()
        .find(|spawn| spawn.template_entry == entry)
        .expect("boss spawn in world.db");
    let world_data = app.world().resource::<WorldData>();
    let template = world_data.templates.get(&entry).unwrap();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    let boss = crate::zones::spawn_creature(
        &mut commands,
        world_data,
        template,
        &mut crate::zones::CreatureSpawnContext {
            map: map,
            difficulty_id: crate::zones::map_difficulty_id(app.world(), map),
            ground: Some(&mut crate::test_data::ground_map()),
            temporary: false,
        },
        false,
        spawn,
        &mut rand::thread_rng(),
    )
    .expect("authored boss level context");
    queue.apply(app.world_mut());
    boss
}

impl Harness {
    /// `entry` at its spawn in copy 1, a player 3 yards from it (and `extra`
    /// players at WoW positions), all out of combat.
    fn new(entry: u32, extra: &[(InMap, [f32; 4])]) -> Option<(Self, Entity)> {
        let data = data()?;
        let mut app = stockade_app(data);
        let boss = spawn_boss(&mut app, data, entry, COPY);
        let spawn = data
            .spawns
            .iter()
            .find(|spawn| spawn.template_entry == entry)
            .unwrap();
        let near = [
            spawn.position_x + 3.0,
            spawn.position_y,
            spawn.position_z,
            std::f32::consts::PI,
        ];
        let mut players = Vec::new();
        let (player, connection) = spawn_player(&mut app, COPY, near, 1);
        players.push((player, connection, Inbox::default()));
        for (index, &(map, position)) in extra.iter().enumerate() {
            let (player, connection) = spawn_player(&mut app, map, position, index as u64 + 2);
            players.push((player, connection, Inbox::default()));
        }
        let mut harness = Self { app, players };
        // The AI initializes (Reset) on the first tick.
        harness.advance(0.05);
        Some((harness, boss))
    }

    fn player(&self) -> Entity {
        self.players[0].0
    }

    fn advance(&mut self, secs: f32) {
        let frames = (secs / 0.05).round().max(1.0) as usize;
        for _ in 0..frames {
            crate::navigation::drain_fixture(&mut self.app);
            self.app.update();
            for (_, connection, inbox) in &mut self.players {
                let world = self.app.world_mut();
                inbox.chat.extend(drain::<ChatMessage>(world, *connection));
                inbox
                    .starts
                    .extend(drain::<EncounterStart>(world, *connection));
                inbox.ends.extend(drain::<EncounterEnd>(world, *connection));
                inbox
                    .timeline
                    .extend(drain::<shared::protocol::EncounterTimelineEvent>(
                        world,
                        *connection,
                    ));
                inbox
                    .engages
                    .extend(drain::<EncounterEngageUnit>(world, *connection));
                inbox
                    .disengages
                    .extend(drain::<EncounterDisengageUnit>(world, *connection));
                inbox
                    .log
                    .extend(drain::<CombatLogEvent>(world, *connection));
                inbox
                    .spell_gos
                    .extend(drain::<shared::protocol::SpellGo>(world, *connection));
                inbox
                    .opened
                    .extend(drain::<shared::protocol::InteractionOpened>(
                        world,
                        *connection,
                    ));
                inbox
                    .closed
                    .extend(drain::<shared::protocol::InteractionClosed>(
                        world,
                        *connection,
                    ));
            }
        }
    }

    fn inbox(&self, index: usize) -> &Inbox {
        &self.players[index].2
    }

    fn engage(&mut self, boss: Entity) {
        let player = self.player();
        crate::creature_aggro::add_threat(self.app.world_mut(), boss, player, 1.0);
    }

    /// The first player hits `boss` with Fireball damage `amount`.
    fn hit(&mut self, boss: Entity, amount: f32) {
        let player = self.player();
        let world = self.app.world_mut();
        let spell = world.resource::<SpellStore>().get(133).unwrap().clone();
        let ctx = EffectCtx {
            caster: player,
            spell: &spell,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        };
        deal_damage(world, &ctx, boss, amount, false, false);
    }

    /// A hit of `pct`% of the boss's max health (level 30 against its native level
    /// 30: no health scaling; fire: no armor).
    fn hit_pct(&mut self, boss: Entity, pct: f32) {
        let amount = self.health(boss).max * pct / 100.0;
        self.hit(boss, amount);
    }

    fn set_health_pct(&mut self, unit: Entity, pct: f32) {
        let mut health = self.app.world_mut().get_mut::<Health>(unit).unwrap();
        health.current = (health.max * pct / 100.0).round();
    }

    fn health(&self, unit: Entity) -> Health {
        *self.app.world().get::<Health>(unit).unwrap()
    }

    fn has_aura(&self, unit: Entity, spell_id: u32) -> bool {
        crate::auras::has_aura(self.app.world(), unit, spell_id)
    }

    fn casts(&self, index: usize, caster: Entity, spell_id: u32) -> usize {
        self.inbox(index)
            .log
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::CastSuccess
                    && event.source == Some(caster.to_bits())
                    && event.spell_id == Some(spell_id)
            })
            .count()
    }

    fn boss_state(&mut self, boss_id: u32) -> Option<EncounterState> {
        get_boss_state(self.app.world_mut(), COPY, boss_id)
    }

    fn wow_position(&self, unit: Entity) -> Vec3 {
        let pos = self.app.world().get::<Position>(unit).unwrap();
        let (x, y, z) = crate::terrain_height::bevy_to_wow(pos.x, pos.y, pos.z);
        Vec3::new(x, y, z)
    }

    fn creatures_of(&mut self, entry: u32) -> Vec<Entity> {
        self.app
            .world_mut()
            .query::<(Entity, &shared::components::Npc)>()
            .iter(self.app.world())
            .filter(|(_, npc)| npc.template_id == entry)
            .map(|(entity, _)| entity)
            .collect()
    }
}

fn drain<M: lightyear::prelude::Message>(world: &mut World, connection: Entity) -> Vec<M> {
    world
        .get_mut::<MessageReceiver<M>>(connection)
        .unwrap()
        .receive()
        .collect()
}

fn said(inbox: &Inbox) -> Vec<(ChatType, String, String)> {
    inbox
        .chat
        .iter()
        .map(|message| {
            (
                message.channel.clone(),
                message.sender.clone(),
                message.content.clone(),
            )
        })
        .collect()
}

// --- EventMap ---

#[test]
fn event_map_runs_due_events_in_order_and_repeats_from_now() {
    let mut events = EventMap::default();
    events.schedule(2, 3_000);
    events.schedule(1, 1_000);
    events.schedule(3, 1_000);
    events.update(999);
    assert_eq!(events.execute_event(), None);
    events.update(1);
    assert_eq!(events.execute_event(), Some(1));
    assert_eq!(events.execute_event(), Some(3));
    // Repeat reschedules the last executed event from the current time.
    events.repeat(500);
    assert_eq!(events.pending(), vec![3, 2]);
    events.update(500);
    assert_eq!(events.execute_event(), Some(3));
    assert_eq!(events.execute_event(), None);
    events.reset();
    assert!(events.pending().is_empty());
}

// --- creature_text ---

#[test]
fn stockade_texts_come_from_broadcast_text() {
    let Some(data) = data() else {
        return;
    };
    let text = |entry: u32, group: u8| data.texts.groups[&entry][&group][0].clone();
    let enrage = text(NPC_HOGGER, 1);
    assert_eq!((enrage.chat_type, enrage.text_range), (41, 3));
    // BroadcastText 46631 names Hogger where creature_text has "%s Enrages!".
    assert_eq!(enrage.male_text, "Hogger enrages!");
    assert_eq!(
        text(NPC_WARDEN_THELWATER, 1).male_text,
        "He's...he's dead? "
    );
    assert_eq!(text(NPC_WARDEN_THELWATER, 1).chat_type, 12);
    assert_eq!(
        text(NPC_MORTIMER_MOLOCH, 0).male_text,
        "Mortimer Moloch collapses from a heart attack!"
    );
}

// --- boss_hogger.cpp ---

#[test]
fn hogger_pull_yells_to_its_copy_starts_encounter_1144_and_engages_its_boss_frame() {
    // A second player in another copy of the Stockade, at the same place.
    let other = [158.0, 116.0, -35.1, 0.0];
    let Some((mut h, hogger)) = Harness::new(NPC_HOGGER, &[(OTHER_COPY, other)]) else {
        return;
    };
    // The player 3 yards away is inside Hogger's aggro radius: he pulls himself.
    h.engage(hogger);
    h.advance(0.1);
    assert_eq!(
        said(h.inbox(0)),
        vec![(
            ChatType::MonsterYell(hogger.to_bits()),
            "Hogger".into(),
            "Forest just setback!".into()
        )]
    );
    assert_eq!(h.boss_state(DATA_HOGGER), Some(EncounterState::InProgress));
    assert_eq!(h.inbox(0).starts.len(), 1);
    assert_eq!(h.inbox(0).starts[0].encounter_id, 1144);
    assert_eq!(
        h.inbox(0).engages,
        vec![EncounterEngageUnit {
            unit: hogger.to_bits(),
            target_frame_priority: 1
        }]
    );
    let other = h.inbox(1);
    assert!(other.chat.is_empty() && other.starts.is_empty() && other.engages.is_empty());
}

#[test]
fn hogger_casts_maddening_call_then_vicious_slice_at_their_timers() {
    let Some((mut h, hogger)) = Harness::new(NPC_HOGGER, &[]) else {
        return;
    };
    let player = h.player();
    h.engage(hogger);
    // Maddening Call is cast at Hogger himself: the player sees its damage.
    let calls = |h: &Harness| -> Vec<i32> {
        h.inbox(0)
            .log
            .iter()
            .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(86620))
            .map(|event| event.amount)
            .collect()
    };
    h.advance(0.9);
    assert!(
        h.app.world().get::<CastState>(hogger).is_none(),
        "Maddening Call waits 1-2 s"
    );
    h.advance(1.2);
    let cast = h
        .app
        .world()
        .get::<CastState>(hogger)
        .expect("Maddening Call cast bar");
    assert_eq!(cast.spell_id, 86620);
    // Its 2.5 s cast ends by 4.5 s.
    h.advance(2.5);
    let calls = calls(&h);
    assert_eq!(calls.len(), 1);
    // 7.74% of ExpectedStat CreatureSpellDamage at Hogger's level, ±5% variance,
    // times his damage multiplier against the level 30 player.
    let level = h.app.world().get::<UnitLevel>(hogger).unwrap().0;
    let data = data().unwrap();
    let base = (data.spell_stats.evaluate(false, level, 0) * 7.737_569 / 100.0).round();
    let scaling = crate::level_scaling::UnitScaling::of(h.app.world(), hogger);
    let player_scaling = crate::level_scaling::UnitScaling::of(h.app.world(), player);
    let low = crate::level_scaling::damage_dealt(&scaling, &player_scaling, (base * 0.95).round());
    let high = crate::level_scaling::damage_dealt(&scaling, &player_scaling, (base * 1.05).round());
    assert!(
        (low as i32 - 1..=high as i32 + 1).contains(&calls[0]),
        "level {level}: {} outside {low}..{high}",
        calls[0]
    );
    assert!(
        h.has_aura(player, 86620),
        "Maddening Call slows its targets"
    );
    let mods = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(player)
        .unwrap();
    assert!((mods.attack_cast_time_mult - 1.6).abs() < 1e-4);
    // Vicious Slice (due at 3-4 s) waits for the cast to end.
    h.advance(0.2);
    assert_eq!(h.casts(0, hogger, 86604), 1, "Vicious Slice");
    let slice_hit = h.inbox(0).log.iter().any(|event| {
        event.spell_id == Some(86604)
            && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
    });
    assert!(slice_hit);
}

#[test]
fn boss_timeline_script_events_are_not_alert_schedules_without_addon_data() {
    let (mut h, boss) = Harness::new(NPC_HOGGER, &[]).unwrap();
    h.engage(boss);
    h.advance(6.0);
    assert!(h.inbox(0).timeline.is_empty());
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.spell_id == Some(86620)),
        "Hogger's combat schedule remains active without addon alert data"
    );
}

#[test]
fn hogger_enrages_once_when_a_hit_takes_it_below_30_percent() {
    let Some((mut h, hogger)) = Harness::new(NPC_HOGGER, &[]) else {
        return;
    };
    h.engage(hogger);
    h.advance(0.1);
    h.set_health_pct(hogger, 31.0);
    h.hit_pct(hogger, 2.0);
    h.advance(0.1);
    assert!(h.health(hogger).current < h.health(hogger).max * 0.3);
    let enrages: Vec<_> = said(h.inbox(0))
        .into_iter()
        .filter(|(channel, ..)| matches!(channel, ChatType::RaidBossEmote(_)))
        .collect();
    assert_eq!(
        enrages,
        vec![(
            ChatType::RaidBossEmote(hogger.to_bits()),
            "Hogger".into(),
            "Hogger enrages!".into()
        )]
    );
    assert!(h.has_aura(hogger, 86736));
    // Enrage: +30% melee haste, +10% physical damage done.
    let world = h.app.world();
    assert_eq!(
        world
            .get::<crate::stats::StatModifiers>(hogger)
            .unwrap()
            .haste_pct,
        30.0
    );
    assert!(
        (world
            .get::<crate::auras::AuraStatMods>(hogger)
            .unwrap()
            .damage_done(1)
            - 1.1)
            .abs()
            < 1e-5
    );
    h.hit_pct(hogger, 2.0);
    h.advance(0.1);
    let emotes = h
        .inbox(0)
        .chat
        .iter()
        .filter(|message| matches!(message.channel, ChatType::RaidBossEmote(_)))
        .count();
    assert_eq!(emotes, 1);
}

#[test]
fn hogger_death_completes_the_encounter_and_warden_thelwater_arrives_and_speaks() {
    let Some((mut h, hogger)) = Harness::new(NPC_HOGGER, &[]) else {
        return;
    };
    h.engage(hogger);
    h.advance(0.1);
    h.set_health_pct(hogger, 1.0);
    h.hit(hogger, 1_000_000.0);
    h.advance(0.1);
    assert_eq!(h.boss_state(DATA_HOGGER), Some(EncounterState::Done));
    assert_eq!(
        h.inbox(0).ends,
        vec![EncounterEnd {
            encounter_id: 1144,
            difficulty_id: 1,
            group_size: 1,
            success: true
        }]
    );
    assert_eq!(h.inbox(0).disengages.len(), 1);
    assert!(said(h.inbox(0)).contains(&(
        ChatType::MonsterYell(hogger.to_bits()),
        "Hogger".into(),
        "Yiiipe!".into()
    )));
    let wardens = h.creatures_of(NPC_WARDEN_THELWATER);
    assert_eq!(wardens.len(), 1);
    let warden = wardens[0];
    let start = h.wow_position(warden);
    // Summoned at its spawn position, already running (8 yards/s) for two ticks.
    assert!(
        start.distance(Vec3::new(138.369, 78.2932, -33.85627)) < 1.0,
        "{start}"
    );
    let motion = |h: &Harness| *h.app.world().get::<CreatureMotion>(warden).unwrap();
    assert_eq!(
        motion(&h),
        CreatureMotion::Run,
        "runs to Hogger (plays Run)"
    );
    // 31.4 yards at run speed 7 × 1.14286: about 3.9 s, then 1 s to the first yell.
    h.advance(5.2);
    let end = h.wow_position(warden);
    assert!(
        end.distance(Vec3::new(152.019, 106.198, -35.1896)) < 0.5,
        "{end}"
    );
    assert_eq!(motion(&h), CreatureMotion::Still, "stands at its point");
    let warden_lines: Vec<String> = h
        .inbox(0)
        .chat
        .iter()
        .filter(|message| message.sender == "Warden Thelwater")
        .map(|message| message.content.clone())
        .collect();
    assert_eq!(warden_lines, vec!["This ends here, Hogger!".to_string()]);
    h.advance(7.0);
    let warden_lines: Vec<(ChatType, String)> = h
        .inbox(0)
        .chat
        .iter()
        .filter(|message| message.sender == "Warden Thelwater")
        .map(|message| (message.channel.clone(), message.content.clone()))
        .collect();
    assert_eq!(
        warden_lines,
        vec![
            (
                ChatType::MonsterYell(warden.to_bits()),
                "This ends here, Hogger!".into()
            ),
            (
                ChatType::MonsterSay(warden.to_bits()),
                "He's...he's dead? ".into()
            ),
            (
                ChatType::MonsterSay(warden.to_bits()),
                "It's simply too good to be true. You couldn't have killed him so easily!".into()
            ),
        ]
    );
}

#[test]
fn hogger_wipe_resets_the_encounter_and_a_new_pull_starts_it_again() {
    let Some((mut h, hogger)) = Harness::new(NPC_HOGGER, &[]) else {
        return;
    };
    let player = h.player();
    h.engage(hogger);
    h.advance(0.1);
    h.set_health_pct(hogger, 31.0);
    h.hit_pct(hogger, 2.0);
    h.advance(0.1);
    assert!(h.has_aura(hogger, 86736));
    // The only player dies: no valid victim left, Hogger evades.
    h.app.world_mut().get_mut::<Health>(player).unwrap().current = 0.0;
    h.advance(0.2);
    assert_eq!(h.boss_state(DATA_HOGGER), Some(EncounterState::NotStarted));
    assert_eq!(h.inbox(0).ends.len(), 1);
    assert!(!h.inbox(0).ends[0].success);
    assert_eq!(h.inbox(0).disengages.len(), 1);
    assert!(!h.has_aura(hogger, 86736), "RemoveAurasOnEvade");
    // `_EnterEvadeMode` keeps its health (regeneration refills it once home).
    assert!(h.health(hogger).current < h.health(hogger).max);
    // Revive and pull again: the pull yell, the encounter start and Maddening
    // Call's first timer come back once.
    {
        let mut health = h.app.world_mut().get_mut::<Health>(player).unwrap();
        health.current = health.max;
    }
    h.advance(12.0);
    h.engage(hogger);
    h.advance(4.7);
    let pulls = h
        .inbox(0)
        .chat
        .iter()
        .filter(|message| message.content == "Forest just setback!")
        .count();
    assert_eq!(pulls, 2);
    assert_eq!(h.inbox(0).starts.len(), 2);
    let calls = h
        .inbox(0)
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(86620))
        .count();
    assert_eq!(
        calls, 1,
        "the first pull's events were reset: one Maddening Call"
    );
}

// --- boss_lord_overheat.cpp ---

#[test]
fn lord_overheat_casts_fireball_overheat_and_rain_of_fire() {
    let Some(data) = data() else {
        return;
    };
    let spawn = data
        .spawns
        .iter()
        .find(|spawn| spawn.template_entry == NPC_LORD_OVERHEAT)
        .unwrap();
    // A second player of the copy 12 yards away, outside Rain of Fire's 5 yards.
    let far = [
        spawn.position_x - 12.0,
        spawn.position_y,
        spawn.position_z,
        0.0,
    ];
    let Some((mut h, overheat)) = Harness::new(NPC_LORD_OVERHEAT, &[(COPY, far)]) else {
        return;
    };
    let (player, far_player) = (h.player(), h.players[1].0);
    let mana_max = h.app.world().get::<Mana>(overheat).unwrap().max;
    // creature_classlevelstats basemana at his level (paladin) × ManaModifier 4.6127.
    let level = h.app.world().get::<UnitLevel>(overheat).unwrap().0;
    assert_eq!(
        mana_max,
        (data.base_mana[&(level, 2)] as f32 * 4.6127).round()
    );
    h.engage(overheat);
    h.advance(0.1);
    assert!(said(h.inbox(0)).contains(&(
        ChatType::MonsterYell(overheat.to_bits()),
        "Lord Overheat".into(),
        "ALL MUST BURN!".into()
    )));
    h.advance(2.0);
    let cast = h
        .app
        .world()
        .get::<CastState>(overheat)
        .expect("Fireball cast bar");
    assert_eq!((cast.spell_id, cast.cast_type), (12466, CastType::Normal));
    // Fireball chains 3 s casts every 2 s: Overheat and Rain of Fire wait for them.
    h.advance(25.0);
    assert!(h.casts(0, overheat, 12466) >= 3);
    assert!(h.casts(0, overheat, 86633) >= 1, "Overheat");
    assert!(h.casts(0, overheat, 86636) >= 1, "Rain of Fire");
    let applied = |inbox: &Inbox, target: Entity, spell_id: u32| {
        inbox.log.iter().any(|event| {
            event.kind == CombatLogKind::AuraApplied
                && event.spell_id == Some(spell_id)
                && event.target == Some(target.to_bits())
        })
    };
    assert!(
        applied(h.inbox(0), player, 86633),
        "Overheat burns its random player"
    );
    // Rain of Fire's 5 yard area at the victim burns it, not the player 15 yards off.
    assert!(applied(h.inbox(0), player, 86636));
    assert!(!applied(h.inbox(1), far_player, 86636));
    let rain_ticks = h
        .inbox(0)
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(86636))
        .count();
    assert!(rain_ticks >= 1, "Rain of Fire ticks every 3 s");
    let mana = h.app.world().get::<Mana>(overheat).unwrap().current;
    assert!(mana < mana_max, "Overheat and Rain of Fire cost mana");
}

// --- boss_randolph_moloch.cpp ---

#[test]
fn randolph_moloch_vanishes_below_71_percent_and_shadowsteps_behind_a_player() {
    let Some((mut h, moloch)) = Harness::new(NPC_RANDOLPH_MOLOCH, &[]) else {
        return;
    };
    let player = h.player();
    // TC Object.cpp:1003 requires reciprocal type4 flags for an invisible caster
    // to detect a unit. Eye of Divinity's1000 cannot reveal Vanish's9999, but
    // supplies that flag. FULL_MASK does not include IGNORE_TARGET_CHECK.
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: player,
            target: player,
            spell_id: 23101,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.engage(moloch);
    h.advance(0.1);
    assert!(said(h.inbox(0)).iter().any(|(_, _, text)| text
        == "Allow me to introduce myself. I am Randolph Moloch, and I will be killing you all today."));
    h.set_health_pct(moloch, 72.0);
    h.hit_pct(moloch, 3.0);
    // EVENT_VANISH 1 s later.
    h.advance(1.3);
    assert!(said(h.inbox(0)).contains(&(
        ChatType::RaidBossEmote(moloch.to_bits()),
        "Randolph Moloch".into(),
        "Randolph Moloch vanishes!".into()
    )));
    assert!(h.has_aura(moloch, 55964));
    let world = h.app.world();
    assert!(!crate::auras::invisibility::can_detect(
        world, player, moloch
    ));
    assert!(!world.get::<UnitFlags>(moloch).unwrap().is_attackable());
    assert!(
        world
            .get::<crate::auras::ActiveAuras>(moloch)
            .unwrap()
            .is_stunned()
    );
    assert_eq!(
        *world.get::<ReactState>(moloch).unwrap(),
        ReactState::Passive
    );
    // EVENT_JUST_VANISHED 2 s later: Shadowstep 4 yards behind the player (who
    // faces WoW angle pi), aggressive again while Vanish's 2.5 s run out.
    h.advance(2.0);
    assert_eq!(h.casts(0, moloch, 55966), 1);
    assert_eq!(
        *h.app.world().get::<ReactState>(moloch).unwrap(),
        ReactState::Aggressive
    );
    let behind = h.wow_position(player) + Vec3::new(4.0, 0.0, 0.0);
    let moloch_at = h.wow_position(moloch);
    assert!(
        Vec2::new(moloch_at.x - behind.x, moloch_at.y - behind.y).length() < 0.5,
        "{moloch_at} vs {behind}"
    );
    h.advance(0.5);
    assert!(!h.has_aura(moloch, 55964));
    let world = h.app.world();
    assert!(crate::auras::invisibility::can_detect(
        world, player, moloch
    ));
    assert!(world.get::<UnitFlags>(moloch).unwrap().is_attackable());
    // A hit from 55% (below 59% before it) does not vanish him again; one from
    // 42% to 39% does.
    h.set_health_pct(moloch, 55.0);
    h.hit_pct(moloch, 3.0);
    h.advance(1.5);
    let vanishes = |h: &Harness| {
        h.inbox(0)
            .chat
            .iter()
            .filter(|message| message.content == "Randolph Moloch vanishes!")
            .count()
    };
    assert_eq!(vanishes(&h), 1);
    h.set_health_pct(moloch, 42.0);
    h.hit_pct(moloch, 3.0);
    h.advance(6.0);
    assert_eq!(vanishes(&h), 2);
}

#[test]
fn randolph_moloch_shadowstep_requires_reciprocal_detection_until_vanish_expires() {
    let Some((mut h, moloch)) = Harness::new(NPC_RANDOLPH_MOLOCH, &[]) else {
        return;
    };
    let player = h.player();
    h.engage(moloch);
    h.advance(0.1);
    h.set_health_pct(moloch, 72.0);
    h.hit_pct(moloch, 3.0);
    h.advance(1.3);
    assert!(h.has_aura(moloch, 55964));
    h.advance(2.0);
    // The source event attempts55966 while Vanish still lasts. A plain player
    // lacks type4 detection, so TC CheckTarget rejects it, even when triggered.
    assert_eq!(h.casts(0, moloch, 55966), 0);
    assert_eq!(
        crate::spell_cast::creature_cast(h.app.world_mut(), moloch, 55966, Some(player), true),
        Err(crate::spell_cast::CastFailure::InvalidTarget)
    );
    assert!(h.has_aura(moloch, 55964));
    h.advance(0.5);
    assert!(!h.has_aura(moloch, 55964));
    assert_eq!(
        h.casts(0, moloch, 55966),
        0,
        "the source event does not retry"
    );
    // The same native cast succeeds after invisibility expires, without a
    // fabricated target-check exemption or detection aura on the boss.
    assert_eq!(
        crate::spell_cast::creature_cast(h.app.world_mut(), moloch, 55966, Some(player), true),
        Ok(())
    );
    // Observe the instant teleport before aggressive pursuit resumes next tick.
    let behind = h.wow_position(player) + Vec3::new(4.0, 0.0, 0.0);
    let moloch_at = h.wow_position(moloch);
    assert!(
        Vec2::new(moloch_at.x - behind.x, moloch_at.y - behind.y).length() < 0.5,
        "{moloch_at} vs {behind}"
    );
    h.advance(0.1);
    assert_eq!(h.casts(0, moloch, 55966), 1);
}

#[test]
fn randolph_moloch_channels_wildly_stabbing_at_his_victim() {
    let Some((mut h, moloch)) = Harness::new(NPC_RANDOLPH_MOLOCH, &[]) else {
        return;
    };
    let player = h.player();
    h.engage(moloch);
    h.advance(5.2);
    let channel = h
        .app
        .world()
        .get::<CastState>(moloch)
        .expect("Wildly Stabbing channel at 4-5 s")
        .clone();
    assert_eq!(
        (channel.spell_id, channel.cast_type),
        (86726, CastType::Channel)
    );
    assert_eq!(channel.target, player.to_bits());
    // The channel's self aura lowers his hit chance by 35%: 40% of his stabs miss.
    let mods = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(moloch)
        .unwrap();
    assert_eq!(crate::combat::creature_miss_chance(Some(mods)), 40.0);
    h.advance(2.0);
    let stabs = h
        .inbox(0)
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(86727)
                && event.target == Some(player.to_bits())
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
        .count();
    // One stab per 250 ms tick of the channel's PERIODIC_TRIGGER_SPELL.
    assert!(stabs >= 6, "{stabs} stabs");
}

#[test]
fn randolph_moloch_death_sends_mortimer_walking_over_to_collapse() {
    let Some((mut h, moloch)) = Harness::new(NPC_RANDOLPH_MOLOCH, &[]) else {
        return;
    };
    h.engage(moloch);
    h.advance(0.1);
    h.set_health_pct(moloch, 1.0);
    h.hit(moloch, 1_000_000.0);
    h.advance(0.1);
    assert_eq!(
        h.boss_state(DATA_RANDOLPH_MOLOCH),
        Some(EncounterState::Done)
    );
    assert_eq!(h.inbox(0).ends[0].encounter_id, 1146);
    assert!(
        said(h.inbox(0))
            .iter()
            .any(|(_, _, text)| text == "My epic schemes, my great plans! Gone!")
    );
    let mortimer = h.creatures_of(NPC_MORTIMER_MOLOCH)[0];
    assert!(
        h.app
            .world()
            .get::<super::movement::Walking>(mortimer)
            .is_some()
    );
    let from = h.wow_position(mortimer);
    let moloch_at = h.wow_position(moloch);
    // 26.9 yards at walk speed 2.5: about 10.8 s, then 4 s and 7 s.
    let walk = from.distance(moloch_at) / 2.5;
    h.advance(walk + 4.3);
    assert!(said(h.inbox(0)).contains(&(
        ChatType::MonsterYell(mortimer.to_bits()),
        "Mortimer Moloch".into(),
        "Egad! My sophisticated heart!".into()
    )));
    assert!(h.health(mortimer).current > 0.0);
    h.advance(3.0);
    assert!(said(h.inbox(0)).contains(&(
        ChatType::MonsterEmote(mortimer.to_bits()),
        "Mortimer Moloch".into(),
        "Mortimer Moloch collapses from a heart attack!".into()
    )));
    assert_eq!(h.health(mortimer).current, 0.0);
}
