//! Real-DB tests against data/world.db.

use std::path::PathBuf;
use std::time::Duration;

use bevy::time::TimeUpdateStrategy;
use lightyear::connection::host::HostClient;
use lightyear::prelude::{Channel, Connected, PeerId, RemoteId};
use rusqlite::{Connection, OpenFlags};
use shared::components::{CharacterAppearance, Health, UnitFactionTemplate};
use shared::protocol::{
    InteractNpc, InteractionFailed, InteractionKind, InteractionOpened, NpcFlags, ProtocolPlugin,
    SelectGossipOption,
};

use super::*;
use crate::faction::world_db_faction_templates;
use crate::npc_interaction::{NpcInteractionPlugin, load_gossip_catalog};
use crate::persistence::ClientId;
use crate::terrain_height::wow_to_bevy;

const HUMAN_TEMPLATE: u32 = 1;
const DUNGAR_LONGDRINK: u32 = 352;
/// Dungar's spawn (guid 79658).
const DUNGAR_AT: (f32, f32, f32) = (-8835.76, 490.084, 109.699);
const STORMWIND: u32 = 2;
const SENTINEL_HILL: u32 = 4;
/// TaxiPath 6 (Stormwind -> Sentinel Hill) lands here.
const SENTINEL_HILL_LANDING: (f32, f32, f32) = (-10550.8, 1034.49, 59.07);

fn world_db_path() -> Option<PathBuf> {
    let path = crate::test_data::world_db_path();
    Some(path)
}

#[derive(Resource)]
struct Inbox<M>(Vec<M>);

fn collect<M: lightyear::prelude::Message>(
    mut receivers: Query<&mut MessageReceiver<M>>,
    mut inbox: ResMut<Inbox<M>>,
) {
    for mut receiver in &mut receivers {
        inbox.0.extend(receiver.receive());
    }
}

fn add_inbox<M: lightyear::prelude::Message>(app: &mut App) {
    app.insert_resource(Inbox::<M>(Vec::new()));
    app.add_systems(First, collect::<M>);
}

type Pair<M> = (MessageSender<M>, MessageReceiver<M>);

fn pair<M: lightyear::prelude::Message>() -> Pair<M> {
    (MessageSender::default(), MessageReceiver::default())
}

struct Harness {
    app: App,
    conn: Connection,
    character_id: u64,
    player: Entity,
    connection: Entity,
}

fn bevy(position: (f32, f32, f32)) -> Position {
    let (x, y, z) = wow_to_bevy(position.0, position.1, position.2);
    Position { x, y, z }
}

fn harness() -> Option<Harness> {
    let path = world_db_path()?;
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        50,
    )));
    app.add_plugins(ProtocolPlugin);
    add_inbox::<InteractionOpened>(&mut app);
    add_inbox::<InteractionFailed>(&mut app);
    add_inbox::<InteractionClosed>(&mut app);
    add_inbox::<TaxiMap>(&mut app);
    add_inbox::<TaxiNodeDiscovered>(&mut app);
    add_inbox::<TaxiFailed>(&mut app);
    app.insert_resource(world_db_faction_templates()?);
    app.insert_resource(load_gossip_catalog(&conn).unwrap());
    app.insert_resource(GameDatabase::new(persistence::temp_database()));
    app.init_resource::<GoldUpdateQueue>();
    app.add_plugins((NpcInteractionPlugin, TaxiPlugin));
    app.insert_resource(load_taxi_network(&conn).unwrap());
    app.finish();
    app.cleanup();
    let db = &app.world().resource::<GameDatabase>().db;
    let (character_id, _) =
        persistence::create_character(db, 1, "Flyer", 1, 1).expect("create character");
    let at_dungar = bevy((DUNGAR_AT.0 + 2.0, DUNGAR_AT.1, DUNGAR_AT.2));
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Flyer".into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            ClientId(character_id),
            at_dungar,
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            MovementSpeed(RUN_SPEED),
            Health {
                current: 100.0,
                max: 100.0,
            },
            UnitFactionTemplate(HUMAN_TEMPLATE),
            crate::maps::InMap::continent(0),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            InWorld { character_id },
            PlayerLink(player),
            (
                pair::<InteractNpc>(),
                pair::<SelectGossipOption>(),
                pair::<InteractionOpened>(),
                pair::<InteractionFailed>(),
                pair::<InteractionClosed>(),
                pair::<TaxiMap>(),
                pair::<TaxiNodeDiscovered>(),
                pair::<TaxiFailed>(),
                pair::<ActivateTaxi>(),
            ),
        ))
        .id();
    app.update();
    Some(Harness {
        app,
        conn,
        character_id,
        player,
        connection,
    })
}

impl Harness {
    fn spawn_dungar(&mut self) -> Entity {
        let (name, faction, npcflag): (String, u32, u32) = self
            .conn
            .query_row(
                "SELECT name, faction, npcflag FROM content_creature_template WHERE entry = ?1",
                [DUNGAR_LONGDRINK],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        self.app
            .world_mut()
            .spawn((
                Npc {
                    template_id: DUNGAR_LONGDRINK,
                    name,
                },
                NpcFlags::from_azerothcore(npcflag),
                crate::maps::InMap::continent(0),
                bevy(DUNGAR_AT),
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                UnitFactionTemplate(faction),
            ))
            .id()
    }

    fn send<M: lightyear::prelude::Message, C: Channel>(&mut self, message: M) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<M>>(self.connection)
            .unwrap()
            .send::<C>(message);
        // Request, role frame, then the taxi reply: two hops of message delivery.
        for _ in 0..5 {
            self.app.update();
        }
    }

    fn take<M: lightyear::prelude::Message>(&mut self) -> Vec<M> {
        std::mem::take(&mut self.app.world_mut().resource_mut::<Inbox<M>>().0)
    }

    /// Right-click Dungar and pick "I need a ride." (gossip icon 2, taxi).
    fn talk_to_dungar(&mut self, dungar: Entity) {
        self.send::<_, InteractionChannel>(InteractNpc {
            npc: dungar.to_bits(),
        });
        let opened = self.take::<InteractionOpened>();
        let InteractionKind::Gossip(menu) = &opened[0].kind else {
            panic!("Dungar greets with gossip: {opened:?}");
        };
        let ride = menu
            .options
            .iter()
            .find(|option| option.icon == 2)
            .expect("taxi option");
        let option_id = ride.option_id;
        self.send::<_, InteractionChannel>(SelectGossipOption {
            npc: dungar.to_bits(),
            option_id,
        });
        let role: Vec<_> = self
            .take::<InteractionOpened>()
            .into_iter()
            .map(|o| o.kind)
            .collect();
        assert_eq!(role, [InteractionKind::Role(NpcRole::FlightMaster)]);
    }

    fn known(&self) -> BTreeSet<u32> {
        load_known_taxi_nodes(
            &self.app.world().resource::<GameDatabase>().db,
            self.character_id,
        )
    }

    fn learn(&mut self, nodes: &[u32]) {
        let db = &self.app.world().resource::<GameDatabase>().db;
        save_known_taxi_nodes(db, self.character_id, &nodes.iter().copied().collect());
    }

    fn set_gold(&mut self, copper: u64) {
        let db = &self.app.world().resource::<GameDatabase>().db;
        let mut inventory = persistence::load_inventory(db, self.character_id);
        inventory.gold = copper;
        persistence::save_inventory(db, self.character_id, &inventory);
    }

    fn gold(&self) -> u64 {
        persistence::load_inventory(
            &self.app.world().resource::<GameDatabase>().db,
            self.character_id,
        )
        .gold
    }

    fn fly_to(&mut self, dungar: Entity, destination: u32) {
        self.send::<_, TaxiChannel>(ActivateTaxi {
            npc: dungar.to_bits(),
            destination,
        });
    }

    fn failures(&mut self) -> Vec<TaxiError> {
        self.take::<TaxiFailed>()
            .into_iter()
            .map(|failed| failed.error)
            .collect()
    }

    fn control(&self) -> MovementControl {
        *self
            .app
            .world()
            .get::<MovementControl>(self.player)
            .unwrap()
    }
}

#[test]
#[ignore = "requires WORLD_POPULATION_DB pointing to a promoted private copy"]
fn zone_population_flight_master_discovers_opens_and_flies_on_its_own_map() {
    const MAP: u32 = 1643;
    const ANNA_RIDGELEY: u32 = 129984;
    let path = std::env::var("WORLD_POPULATION_DB").expect("promoted copy");
    let content = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut h = harness().expect("authentic taxi network");
    h.app.insert_resource(load_taxi_network(&content).unwrap());
    let (name, faction, flags, x, y, z): (String, u32, u32, f32, f32, f32) = content
        .query_row(
            "SELECT t.name,t.faction,t.npcflag,c.position_x,c.position_y,c.position_z
             FROM content_creature c JOIN content_creature_template t ON t.entry=c.id1
             WHERE c.map=?1 AND c.id1=?2 LIMIT 1",
            [MAP, ANNA_RIDGELEY],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
        .unwrap();
    let at = bevy((x, y, z));
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert((at, crate::maps::InMap::continent(MAP)));
    let npc = h
        .app
        .world_mut()
        .spawn((
            Npc {
                template_id: ANNA_RIDGELEY,
                name,
            },
            NpcFlags::from_azerothcore(flags),
            at,
            crate::maps::InMap::continent(MAP),
            UnitFactionTemplate(faction),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    h.send::<_, InteractionChannel>(InteractNpc { npc: npc.to_bits() });
    assert_eq!(
        h.take::<InteractionOpened>()[0].kind,
        InteractionKind::Role(NpcRole::FlightMaster)
    );
    let maps = h.take::<TaxiMap>();
    assert_eq!(maps.len(), 1);
    assert_eq!(maps[0].continent, MAP);
    let current = maps[0]
        .nodes
        .iter()
        .find(|n| n.state == TaxiNodeState::Current)
        .unwrap()
        .node;
    assert_eq!(h.known(), BTreeSet::from([current]));
    let network = h.app.world().resource::<TaxiNetwork>();
    let known: BTreeSet<_> = network
        .map_nodes(MAP, Team::Alliance)
        .map(|(id, _)| id)
        .collect();
    let destination = known
        .iter()
        .copied()
        .find(|&id| network.route(current, id, &known, Team::Alliance).is_some())
        .expect("authored route");
    h.learn(&known.iter().copied().collect::<Vec<_>>());
    h.set_gold(1_000_000);
    h.fly_to(npc, destination);
    let failures = h.failures();
    assert!(failures.is_empty(), "taxi failures: {failures:?}");
    assert!(h.app.world().get::<TaxiFlight>(h.player).is_some());
    assert!(h.control().controlled);
    assert!(h.gold() < 1_000_000);
}

fn state_of(map: &TaxiMap, node: u32) -> TaxiNodeState {
    map.nodes
        .iter()
        .find(|info| info.node == node)
        .unwrap_or_else(|| panic!("node {node} on the map"))
        .state
}

#[test]
fn first_talk_to_dungar_discovers_stormwind_and_shows_the_eastern_kingdoms_map() {
    let Some(mut h) = harness() else { return };
    let dungar = h.spawn_dungar();

    h.talk_to_dungar(dungar);

    assert_eq!(
        h.take::<TaxiNodeDiscovered>(),
        vec![TaxiNodeDiscovered { node: STORMWIND }]
    );
    assert_eq!(h.known(), BTreeSet::from([STORMWIND]));
    let maps = h.take::<TaxiMap>();
    assert_eq!(maps.len(), 1);
    assert_eq!(maps[0].npc, dungar.to_bits());
    assert_eq!(state_of(&maps[0], STORMWIND), TaxiNodeState::Current);
    assert_eq!(
        state_of(&maps[0], SENTINEL_HILL),
        TaxiNodeState::Unreachable
    );
    assert!(
        maps[0]
            .nodes
            .iter()
            .all(|node| node.node == STORMWIND || node.state == TaxiNodeState::Unreachable),
        "nothing else is known yet"
    );

    h.talk_to_dungar(dungar);
    assert!(h.take::<TaxiNodeDiscovered>().is_empty(), "known already");
    assert_eq!(h.take::<TaxiMap>().len(), 1);
}

#[test]
fn a_known_sentinel_hill_is_reachable_for_5_copper() {
    let Some(mut h) = harness() else { return };
    let dungar = h.spawn_dungar();
    h.learn(&[STORMWIND, SENTINEL_HILL]);

    h.talk_to_dungar(dungar);

    let map = &h.take::<TaxiMap>()[0];
    let sentinel = map
        .nodes
        .iter()
        .find(|node| node.node == SENTINEL_HILL)
        .unwrap();
    assert_eq!(sentinel.state, TaxiNodeState::Reachable);
    assert_eq!(sentinel.cost, 5);
    assert_eq!(sentinel.route, [STORMWIND, SENTINEL_HILL]);
    assert_eq!(sentinel.name, "Sentinel Hill, Westfall");
}

#[test]
fn flying_to_sentinel_hill_pays_rides_the_gryphon_and_lands_on_the_path_end() {
    let Some(mut h) = harness() else { return };
    let dungar = h.spawn_dungar();
    h.learn(&[STORMWIND, SENTINEL_HILL]);
    h.set_gold(12);
    h.talk_to_dungar(dungar);
    h.take::<InteractionClosed>();

    h.fly_to(dungar, SENTINEL_HILL);

    assert!(h.failures().is_empty());
    assert_eq!(h.gold(), 7);
    assert_eq!(
        h.take::<InteractionClosed>().len(),
        1,
        "the flight map closes"
    );
    assert_eq!(
        h.app.world().get::<Mounted>(h.player),
        Some(&Mounted {
            mount_display_id: 6852,
            ..Default::default()
        }),
        "Riding Gryphon"
    );
    assert!(h.control().controlled);
    let start = *h.app.world().get::<Position>(h.player).unwrap();

    for _ in 0..20 {
        h.app.update();
    }
    let moved = *h.app.world().get::<Position>(h.player).unwrap();
    assert_ne!(moved, start, "the server flies the player");

    let mut updates = 0;
    while h.app.world().get::<TaxiFlight>(h.player).is_some() {
        h.app.update();
        updates += 1;
        assert!(updates < 5000, "landed within 250 s");
    }
    let landed = *h.app.world().get::<Position>(h.player).unwrap();
    let expected = bevy(SENTINEL_HILL_LANDING);
    assert!(
        (landed.x - expected.x).abs() < 0.1 && (landed.z - expected.z).abs() < 0.1,
        "{landed:?}"
    );
    assert_eq!(
        h.control(),
        MovementControl {
            epoch: 1,
            controlled: false
        }
    );
    assert!(h.app.world().get::<Mounted>(h.player).is_none());
    assert_eq!(h.known(), BTreeSet::from([STORMWIND, SENTINEL_HILL]));
}

#[test]
fn refusals_use_retail_taxi_errors() {
    let Some(mut h) = harness() else { return };
    let dungar = h.spawn_dungar();
    h.learn(&[STORMWIND, SENTINEL_HILL]);
    h.set_gold(4);

    h.fly_to(dungar, SENTINEL_HILL);
    assert_eq!(
        h.failures(),
        vec![TaxiError::NoVendorNearby],
        "map not open"
    );

    h.talk_to_dungar(dungar);
    h.fly_to(dungar, SENTINEL_HILL);
    assert_eq!(h.failures(), vec![TaxiError::NotEnoughMoney]);
    assert_eq!(h.gold(), 4);

    h.fly_to(dungar, 12);
    assert_eq!(
        h.failures(),
        vec![TaxiError::NotVisited],
        "Darkshire unknown"
    );
    h.fly_to(dungar, STORMWIND);
    assert_eq!(h.failures(), vec![TaxiError::SameNode]);
    assert!(h.app.world().get::<TaxiFlight>(h.player).is_none());
}

#[test]
fn flying_is_refused_through_a_lull_in_a_fight_and_allowed_once_the_creature_dies() {
    let Some(mut h) = harness() else { return };
    h.app
        .add_systems(Update, crate::combat_refs::expire_combat_refs_system);
    let dungar = h.spawn_dungar();
    h.learn(&[STORMWIND, SENTINEL_HILL]);
    h.set_gold(12);
    h.talk_to_dungar(dungar);
    let wolf = h
        .app
        .world_mut()
        .spawn((
            Health {
                current: 100.0,
                max: 100.0,
            },
            shared::components::ThreatTable::default(),
        ))
        .id();
    crate::combat_refs::set_in_combat_with(h.app.world_mut(), h.player, wolf);
    // 8 s without a hit: the fight with a creature does not time out.
    for _ in 0..160 {
        h.app.update();
    }
    h.fly_to(dungar, SENTINEL_HILL);
    assert_eq!(h.failures(), vec![TaxiError::InCombat]);

    crate::combat::combat_stop(h.app.world_mut(), wolf);
    h.fly_to(dungar, SENTINEL_HILL);
    assert!(h.failures().is_empty());
    assert!(h.app.world().get::<TaxiFlight>(h.player).is_some());
}

#[test]
fn a_flight_steps_through_its_points_at_the_given_distance() {
    let mut flight = TaxiFlight {
        points: vec![Vec3::new(10.0, 0.0, 0.0), Vec3::new(10.0, 0.0, 10.0)],
        next: 0,
    };
    let mut position = Vec3::ZERO;
    assert!(!advance_flight(&mut flight, &mut position, 4.0));
    assert_eq!(position, Vec3::new(4.0, 0.0, 0.0));
    assert!(!advance_flight(&mut flight, &mut position, 8.0));
    assert_eq!(position, Vec3::new(10.0, 0.0, 2.0));
    assert!(advance_flight(&mut flight, &mut position, 100.0));
    assert_eq!(position, Vec3::new(10.0, 0.0, 10.0));
}

#[test]
fn achievements_criteria_paid_flight_counts_flight_and_copper_but_refusals_do_not() {
    use crate::achievement::PlayerAchievementState;
    let Some(mut h) = harness() else { return };
    let conn = Connection::open(world_db_path().unwrap()).unwrap();
    h.app
        .insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
    h.app.init_resource::<crate::maps::ServerClock>();
    let (player, connection) = (h.player, h.connection);
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(PlayerAchievementState::default());
    h.app
        .world_mut()
        .entity_mut(connection)
        .insert(crate::auth::Authenticated { account_id: 1 });
    let progress = |h: &Harness, criterion: u32| {
        h.app
            .world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&criterion)
            .copied()
    };
    let dungar = h.spawn_dungar();
    h.learn(&[STORMWIND, SENTINEL_HILL]);
    h.set_gold(4);
    h.talk_to_dungar(dungar);
    h.fly_to(dungar, SENTINEL_HILL);
    assert_eq!(h.failures(), vec![TaxiError::NotEnoughMoney]);
    // Flight paths taken349 (criterion5305) and Gold spent on travel1146 (3356).
    assert_eq!((progress(&h, 5305), progress(&h, 3356)), (None, None));
    h.set_gold(12);
    h.fly_to(dungar, SENTINEL_HILL);
    assert!(h.failures().is_empty());
    assert_eq!(h.gold(), 7);
    assert_eq!((progress(&h, 5305), progress(&h, 3356)), (Some(1), Some(5)));
}

#[test]
fn achievements_criteria_learned_taxi_node_is_a_boolean_node_fact() {
    let conn = Connection::open(world_db_path().unwrap()).unwrap();
    let catalog = crate::achievement::load_achievement_catalog(&conn).unwrap();
    let mut state = crate::achievement::PlayerAchievementState::default();
    // Khaz Algar Flight Master40430 leaf68170 is TaxiNodes2928 (no hosted Khaz Algar
    // network, so only the event semantics are exercised here).
    let event = crate::achievement::CriteriaEvent::LearnTaxiNode(2928);
    catalog.update(&mut state, event, None, 1);
    catalog.update(&mut state, event, None, 1);
    assert_eq!(state.criteria_progress.get(&68170), Some(&1));
    assert!(!state.criteria_progress.contains_key(&68171));
}
