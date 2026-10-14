//! Map transfers and dungeon copies on real world.db data: the Stormwind Stockade (map 34), its entrance area trigger 101 in
//! Stormwind City and its exit trigger 503.

use std::sync::OnceLock;
use std::time::Duration;

use bevy::ecs::system::RunSystemOnce;
use lightyear::connection::host::HostClient;
use lightyear::prelude::*;
use lightyear::prelude::{Connected, PeerId, RemoteId};
use rusqlite::{Connection, OpenFlags};
use shared::components::{Health, MovementControl, Player, Position, Rotation};
use shared::death::{Corpse, DeathState};
use shared::protocol::*;

use super::instances::{INSTANCE_UNLOAD_DELAY_SECS, InstanceEntrant};
use super::store::MapStore;
use super::*;
use crate::auth::{Authenticated, InWorld, SentTiles};
use crate::death::PlayerDeathState;
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use crate::networking::{CombatTarget, PlayerLink};
use crate::persistence::ClientId;

const STOCKADE: u32 = 34;
/// `areatrigger_teleport` 101 "Stormwind Jail Entrance Target": its AreaTrigger box
/// center in Stormwind City.
const STOCKADE_ENTRANCE_TRIGGER: Vec3 = Vec3::new(-8761.85, 848.557, 87.8052);
/// `world_safe_locs` 3599 inside the Stockade.
const STOCKADE_ARRIVAL: Vec3 = Vec3::new(56.6821, 0.62376, -19.2691);
/// `world_safe_locs` 3618 "Stormwind Low Prison Exit Target".
const STOCKADE_EXIT_TARGET: Vec3 = Vec3::new(-8766.11, 845.499, 87.9952);
/// Area trigger 503's box center inside the Stockade.
const STOCKADE_EXIT_TRIGGER: Vec3 = Vec3::new(48.0937, 0.933267, -13.7769);
/// `world_safe_locs` 1854 "Stormwind Stockade, GY", the graveyard of zone 717.
const STOCKADE_GRAVEYARD: Vec3 = Vec3::new(-8786.88, 829.083, 97.6504);
/// Ratchet, Northern Barrens (Kalimdor).
const RATCHET: Vec3 = Vec3::new(-956.0, -3754.0, 5.0);

fn map_store() -> Option<&'static MapStore> {
    static STORE: OnceLock<Option<MapStore>> = OnceLock::new();
    STORE
        .get_or_init(|| {
            let path = crate::test_data::world_db_path();
            let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .expect("open world.db");
            Some(
                store::load_map_store(&conn)
                    .expect("map tables in world.db; run scripts/import-azerothcore.sh"),
            )
        })
        .as_ref()
}

fn close(a: Vec3, b: Vec3) -> bool {
    a.distance(b) < 0.01
}

fn wow(position: &Position) -> Vec3 {
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    Vec3::new(x, y, z)
}

fn bevy(wow: Vec3) -> Position {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(wow.x, wow.y, wow.z);
    Position { x, y, z }
}

#[test]
fn stockade_entrance_trigger_leads_to_its_arrival_point_and_the_exit_back() {
    let Some(store) = map_store() else { return };
    let map = store.map(STOCKADE).expect("map 34");
    assert_eq!(map.directory, "stormwindjail");
    assert!(map.is_dungeon());
    assert_eq!(map.area_table_id, 717);

    let entrance = store
        .area_triggers_on(0)
        .iter()
        .find(|trigger| trigger.id == 101)
        .expect("area trigger 101 on map 0");
    assert!(entrance.contains(STOCKADE_ENTRANCE_TRIGGER));
    // Box 4.97 x 9.69 x 7.44 turned 0.663 rad: 6 yd off-center along X is outside.
    assert!(!entrance.contains(STOCKADE_ENTRANCE_TRIGGER + Vec3::new(6.0, 0.0, 0.0)));
    assert!(entrance.allow_while_ghost);
    assert_eq!(entrance.target.map_id, STOCKADE);
    assert!(close(entrance.target.position, STOCKADE_ARRIVAL));

    let back = store
        .go_back_location(STOCKADE)
        .expect("go-back trigger 503");
    assert_eq!(back.map_id, 0);
    assert!(close(back.position, STOCKADE_EXIT_TARGET));
    let inside = store.entrance_location(STOCKADE).expect("entrance trigger");
    assert!(close(inside.position, STOCKADE_ARRIVAL));
}

#[test]
fn a_stockade_death_releases_to_the_stockade_graveyard_in_stormwind() {
    let Some(store) = map_store() else { return };
    let inside = WorldLocation {
        map_id: STOCKADE,
        position: STOCKADE_ARRIVAL,
        orientation: 0.0,
    };
    let grave = graveyard::closest_graveyard_in_zone(store, &inside, 717).expect("graveyard");
    assert_eq!(grave.map_id, 0);
    assert!(close(grave.position, STOCKADE_GRAVEYARD));
}

/// 2026-09-27 12:00 UTC (a Sunday): the instance-lock clock of these tests.
const SUNDAY_NOON: i64 = 1_790_510_400;

/// A Normal-difficulty entrant; a grouped one's leader is character 1.
fn entrant(character_id: u64, group: Option<u64>) -> InstanceEntrant {
    InstanceEntrant {
        character_id,
        group,
        leader: if group.is_some() { 1 } else { character_id },
        difficulty_id: store::DIFFICULTY_NORMAL,
    }
}

/// Elapsed seconds `elapsed` on Sunday noon.
fn at(elapsed: f64) -> instances::Now {
    instances::Now {
        elapsed,
        unix: SUNDAY_NOON,
    }
}

#[test]
fn group_members_share_a_stockade_copy_and_a_solo_player_gets_its_own() {
    let Some(store) = map_store() else { return };
    let mut instances = InstanceManager::default();
    let first = instances
        .create_map(store, STOCKADE, entrant(1, Some(7)), at(0.0))
        .unwrap();
    let second = instances
        .create_map(store, STOCKADE, entrant(2, Some(7)), at(1.0))
        .unwrap();
    let solo = instances
        .create_map(store, STOCKADE, entrant(3, None), at(2.0))
        .unwrap();

    assert!(first.created);
    assert_eq!(
        second,
        instances::ResolvedMap {
            map: first.map,
            created: false,
        }
    );
    assert!(solo.created);
    assert_ne!(solo.map.instance_id, first.map.instance_id);
    // The solo player's copy is not the group's to enter, and the other way round.
    assert_eq!(
        instances.find_instance(store, STOCKADE, entrant(3, Some(7)), SUNDAY_NOON),
        Some(first.map)
    );
    assert_eq!(
        instances.find_instance(store, STOCKADE, entrant(1, None), SUNDAY_NOON),
        None
    );
}

#[test]
fn a_full_stockade_copy_refuses_the_sixth_player_and_a_stranger_to_its_group() {
    let Some(store) = map_store() else { return };
    let mut world = World::new();
    let mut instances = InstanceManager::default();
    let copy = instances
        .create_map(store, STOCKADE, entrant(1, Some(7)), at(0.0))
        .unwrap()
        .map;
    // MapDifficulty 8 (Normal) holds 5 players.
    for _ in 0..5 {
        instances.add_player(copy, world.spawn_empty().id());
    }
    assert_eq!(
        instances.player_cannot_enter(store, STOCKADE, entrant(6, Some(7)), 80, SUNDAY_NOON),
        Some(TransferAbortReason::MaxPlayers)
    );
    assert_eq!(
        instances.cannot_enter(copy, entrant(9, Some(8)), SUNDAY_NOON),
        Some(TransferAbortReason::MaxPlayers)
    );
}

#[test]
fn an_empty_copy_unloads_after_thirty_minutes_and_comes_back_fresh() {
    let Some(store) = map_store() else { return };
    let mut world = World::new();
    let player = world.spawn_empty().id();
    let mut instances = InstanceManager::default();
    let copy = instances
        .create_map(store, STOCKADE, entrant(1, None), at(0.0))
        .unwrap()
        .map;
    instances.add_player(copy, player);
    instances.remove_player(copy, player, at(100.0));

    assert!(
        instances
            .take_expired(100.0 + INSTANCE_UNLOAD_DELAY_SECS - 1.0)
            .is_empty()
    );
    assert_eq!(
        instances.take_expired(100.0 + INSTANCE_UNLOAD_DELAY_SECS),
        vec![copy]
    );
    // The player re-enters its recent instance id as a new copy.
    let again = instances
        .create_map(store, STOCKADE, entrant(1, None), at(2000.0))
        .unwrap();
    assert_eq!(again.map, copy);
    assert!(again.created);
}

#[test]
fn raids_and_world_maps_without_npc_data_are_not_hosted() {
    let Some(store) = map_store() else { return };
    let instances = InstanceManager::default();
    // Outland (530); Molten Core (409), a raid.
    for map_id in [530, 409] {
        assert_eq!(
            instances.player_cannot_enter(store, map_id, entrant(1, None), 80, SUNDAY_NOON),
            Some(TransferAbortReason::MapNotAllowed),
            "map {map_id}"
        );
    }
    assert_eq!(
        instances.player_cannot_enter(store, 1, entrant(1, None), 1, SUNDAY_NOON),
        None
    );
}

/// The Culling of Stratholme (595) Normal: MapDifficulty 91's conditions (40817, 43808)
/// carry no ContentTuning, so no level is required.
const CULLING_OF_STRATHOLME: u32 = 595;

#[test]
fn a_level_one_player_may_enter_a_dungeon_without_a_level_condition() {
    let Some(store) = map_store() else { return };
    let instances = InstanceManager::default();
    assert_eq!(
        instances.player_cannot_enter(
            store,
            CULLING_OF_STRATHOLME,
            entrant(1, None),
            1,
            SUNDAY_NOON
        ),
        None
    );
}

// --- App-level transfers ---

struct Harness {
    app: App,
    /// Messages each connection received, drained every frame (a `MessageReceiver`
    /// only holds the current frame's messages).
    new_worlds: std::collections::HashMap<Entity, Vec<NewWorld>>,
    aborts: std::collections::HashMap<Entity, Vec<TransferAborted>>,
    saves: std::collections::HashMap<Entity, usize>,
}

struct Traveler {
    player: Entity,
    connection: Entity,
}

fn pair<M: lightyear::prelude::Message>() -> (MessageSender<M>, MessageReceiver<M>) {
    (MessageSender::default(), MessageReceiver::default())
}

fn harness() -> Option<Harness> {
    harness_with(|_| {})
}

/// A harness whose copies spawn world.db's creatures and game objects.
fn harness_with_content() -> Option<Harness> {
    harness_with_content_and(|_| {})
}

fn harness_with_content_and(extra: impl FnOnce(&mut App)) -> Option<Harness> {
    let path = crate::test_data::world_db_path();
    let world_data = crate::world_data::load_world_data(&path);
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("open world.db");
    let objects = crate::game_objects::load_game_object_spawns(&conn).expect("game objects");
    harness_with(move |app| {
        app.insert_resource(world_data);
        app.insert_resource(objects);
        extra(app);
    })
}

fn harness_with(setup: impl FnOnce(&mut App)) -> Option<Harness> {
    let store = map_store()?;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    app.add_plugins(MapPlugin);
    app.add_message::<crate::death::PlayerDeathTransition>();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(50),
    ));
    app.insert_resource(store.clone());
    app.insert_resource(ServerClock {
        offset_secs: 0,
        fixed_unix: Some(SUNDAY_NOON),
    });
    app.init_resource::<GroupRegistry>();
    setup(&mut app);
    app.finish();
    app.cleanup();
    app.update();
    Some(Harness {
        app,
        new_worlds: Default::default(),
        aborts: Default::default(),
        saves: Default::default(),
    })
}

/// The level harness players have: every hosted dungeon's level condition admits it
/// (Mythic Grim Batol, PlayerCondition 115171, needs 80).
const TRAVELER_LEVEL: u8 = 80;

impl Harness {
    fn spawn(&mut self, character_id: u64, map: InMap, at: Vec3) -> Traveler {
        self.spawn_at_level(character_id, TRAVELER_LEVEL, map, at)
    }

    fn spawn_at_level(&mut self, character_id: u64, level: u8, map: InMap, at: Vec3) -> Traveler {
        let player = self
            .app
            .world_mut()
            .spawn((
                Player {
                    name: format!("Traveler{character_id}"),
                    race: 1,
                    class: 1,
                    appearance: Default::default(),
                },
                ClientId(character_id),
                bevy(at),
                Rotation {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                MovementControl::default(),
                CombatTarget(None),
                shared::components::Level(level),
                map,
            ))
            .id();
        let connection = self
            .app
            .world_mut()
            .spawn((
                RemoteId(PeerId::Local(character_id)),
                Connected,
                HostClient { buffer: Vec::new() },
                Authenticated {
                    account_id: character_id,
                },
                InWorld { character_id },
                PlayerLink(player),
                SentTiles {
                    tiles: Default::default(),
                    last_tile: Some((32, 48)),
                },
                (
                    pair::<NewWorld>(),
                    pair::<WorldPortAck>(),
                    pair::<TransferAborted>(),
                    pair::<DeathStateUpdate>(),
                    pair::<InstanceSaveCreated>(),
                    pair::<WorldServerInfo>(),
                    pair::<SetDungeonDifficulty>(),
                    pair::<DungeonDifficultySet>(),
                ),
            ))
            .id();
        self.app.update();
        Traveler { player, connection }
    }

    fn advance(&mut self, frames: usize) {
        for _ in 0..frames {
            self.app.update();
            let world = self.app.world_mut();
            let mut connections = world.query::<(
                Entity,
                &mut MessageReceiver<NewWorld>,
                &mut MessageReceiver<TransferAborted>,
                &mut MessageReceiver<InstanceSaveCreated>,
            )>();
            for (connection, mut new_worlds, mut aborts, mut saves) in connections.iter_mut(world) {
                *self.saves.entry(connection).or_default() += saves.receive().count();
                self.new_worlds
                    .entry(connection)
                    .or_default()
                    .extend(new_worlds.receive());
                self.aborts
                    .entry(connection)
                    .or_default()
                    .extend(aborts.receive());
            }
        }
    }

    fn walk_to(&mut self, who: &Traveler, at: Vec3) {
        self.app.world_mut().entity_mut(who.player).insert(bevy(at));
        self.advance(2);
    }

    fn new_worlds(&mut self, who: &Traveler) -> Vec<NewWorld> {
        self.advance(2);
        self.new_worlds.remove(&who.connection).unwrap_or_default()
    }

    fn aborts(&mut self, who: &Traveler) -> Vec<TransferAborted> {
        self.advance(2);
        self.aborts.remove(&who.connection).unwrap_or_default()
    }

    fn ack(&mut self, who: &Traveler) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<WorldPortAck>>(who.connection)
            .unwrap()
            .send::<TransferChannel>(WorldPortAck);
        self.advance(3);
    }

    fn map_of(&self, who: &Traveler) -> Option<InMap> {
        self.app.world().get::<InMap>(who.player).copied()
    }

    fn position(&self, who: &Traveler) -> Vec3 {
        wow(self.app.world().get::<Position>(who.player).unwrap())
    }

    fn group(&mut self, members: &[&Traveler]) {
        let online: OnlineView = members
            .iter()
            .map(|member| {
                let id = self.app.world().get::<ClientId>(member.player).unwrap().0;
                (
                    id,
                    OnlineMember {
                        entity: member.player.to_bits(),
                        name: format!("Traveler{id}"),
                        class: 1,
                        level: 20,
                        portrait: Default::default(),
                        dungeon_difficulty: crate::maps::store::DIFFICULTY_NORMAL,
                    },
                )
            })
            .collect();
        let ids: Vec<u64> = members
            .iter()
            .map(|member| self.app.world().get::<ClientId>(member.player).unwrap().0)
            .collect();
        let mut registry = self.app.world_mut().resource_mut::<GroupRegistry>();
        registry.sync_presence(&online);
        for (index, &id) in ids.iter().enumerate().skip(1) {
            // A party holds five: a sixth member joins a raid.
            if index == 5 {
                registry.convert_to_raid(ids[0]);
            }
            registry.invite(ids[0], &format!("Traveler{id}"), &online);
            registry.respond_invite(id, true);
        }
        assert_eq!(registry.group_members(ids[0]).len(), ids.len());
    }

    fn arrivals(&self, who: &Traveler) -> u32 {
        self.app
            .world()
            .get::<shared::components::WorldArrival>(who.player)
            .unwrap()
            .0
    }

    /// Stockade creatures `who`'s connection sees.
    fn visible_stockade_creatures(&mut self, who: &Traveler) -> usize {
        let world = self.app.world_mut();
        let mut query = world.query_filtered::<
            (&InMap, &crate::interest::NetworkVisibility),
            With<shared::components::Npc>,
        >();
        query
            .iter(world)
            .filter(|(map, visibility)| {
                map.map_id == STOCKADE && visibility.is_visible(who.connection)
            })
            .count()
    }

    /// (template, name, level) of every creature in `map`, sorted.
    fn creatures_in(&mut self, map: InMap) -> Vec<(u32, String, u8)> {
        let world = self.app.world_mut();
        let mut query = world.query::<(&shared::components::Npc, &UnitLevelOf, &InMap)>();
        let mut creatures: Vec<_> = query
            .iter(world)
            .filter(|(_, _, in_map)| **in_map == map)
            .map(|(npc, level, _)| (npc.template_id, npc.name.clone(), level.0))
            .collect();
        creatures.sort();
        creatures
    }

    /// Walk from beside the entrance into trigger 101 and answer the `NewWorld`.
    fn enter_stockade(&mut self, who: &Traveler) -> NewWorld {
        self.walk_to(who, STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0));
        self.walk_to(who, STOCKADE_ENTRANCE_TRIGGER);
        let new_world = self.new_worlds(who).pop().expect("NewWorld for map 34");
        self.ack(who);
        new_world
    }
}

const STORMWIND: InMap = InMap::continent(0);
type UnitLevelOf = shared::components::UnitLevel;
/// Retail Stockade (TDB1210 map 34, DEFAULT_PHASE 169, Normal): 143 creatures, one Hogger (46254)
/// on ContentTuning 215 (levels 10-30) with LevelScalingDelta 2.
const STOCKADE_CREATURES: usize = 143;
const HOGGER: u32 = 46254;

#[test]
fn each_stockade_copy_spawns_its_own_retail_creatures_and_unloads_them() {
    let Some(mut h) = harness_with_content() else {
        return;
    };
    let start = STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0);
    let first = h.spawn(1, STORMWIND, start);
    let second = h.spawn(2, STORMWIND, start);
    h.enter_stockade(&first);
    h.enter_stockade(&second);
    let (a, b) = (h.map_of(&first).unwrap(), h.map_of(&second).unwrap());
    assert_ne!(a, b);
    for copy in [a, b] {
        let creatures = h.creatures_in(copy);
        assert_eq!(creatures.len(), STOCKADE_CREATURES, "{copy:?}");
        let hogger: Vec<_> = creatures.iter().filter(|c| c.0 == HOGGER).collect();
        assert_eq!(hogger.len(), 1, "{copy:?}");
        assert_eq!(hogger[0].1, "Hogger");
        assert_eq!(
            hogger[0].2, 32,
            "ContentTuning 215 top level 30 + Normal delta 2"
        );
    }

    // `first` leaves; after Instance.UnloadDelay its copy goes with its creatures.
    h.walk_to(&first, STOCKADE_EXIT_TRIGGER);
    h.new_worlds(&first);
    h.ack(&first);
    let delay = Duration::from_secs_f64(INSTANCE_UNLOAD_DELAY_SECS + 1.0);
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(delay);
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(delay));
    h.advance(2);
    assert!(h.creatures_in(a).is_empty());
    assert_eq!(
        h.creatures_in(b).len(),
        STOCKADE_CREATURES,
        "the occupied copy stays"
    );
}

#[test]
fn walking_into_the_stockade_entrance_transfers_the_player_into_a_copy_of_map_34() {
    let Some(mut h) = harness() else { return };
    let player = h.spawn(
        1,
        STORMWIND,
        STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0),
    );
    h.walk_to(&player, STOCKADE_ENTRANCE_TRIGGER);

    // Out of map 0, waiting for the client.
    assert_eq!(h.map_of(&player), None);
    let new_world = h.new_worlds(&player).pop().expect("NewWorld");
    assert_eq!(new_world.map_id, STOCKADE);
    assert_eq!(new_world.map_directory, "stormwindjail");
    assert!(close(
        wow(&Position {
            x: new_world.position[0],
            y: new_world.position[1],
            z: new_world.position[2],
        }),
        STOCKADE_ARRIVAL
    ));

    h.ack(&player);
    let map = h.map_of(&player).expect("in map 34");
    assert_eq!(map.map_id, STOCKADE);
    assert_ne!(map.instance_id, 0);
    assert!(close(h.position(&player), STOCKADE_ARRIVAL));
    let epoch = h
        .app
        .world()
        .get::<MovementControl>(player.player)
        .unwrap()
        .epoch;
    assert_eq!(epoch, 1, "the client adopts the arrival position");
}

/// `HandleMoveWorldportAck` sends the arriving player the destination's objects; the
/// `WorldArrival` count rises in the frame that makes them visible, so both go out in
/// one replication tick and the client knows its arrival is complete.
#[test]
fn the_arrival_count_rises_in_the_frame_the_stockade_creatures_become_visible() {
    let Some(mut h) = harness_with_content_and(|app| {
        app.add_plugins(crate::interest::InterestPlugin);
    }) else {
        return;
    };
    let player = h.spawn(
        1,
        STORMWIND,
        STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0),
    );
    h.walk_to(&player, STOCKADE_ENTRANCE_TRIGGER);
    h.new_worlds(&player).pop().expect("NewWorld for map 34");
    assert_eq!(h.arrivals(&player), 0);
    assert_eq!(h.visible_stockade_creatures(&player), 0);

    h.app
        .world_mut()
        .get_mut::<MessageSender<WorldPortAck>>(player.connection)
        .unwrap()
        .send::<TransferChannel>(WorldPortAck);
    let mut frames = Vec::new();
    for _ in 0..6 {
        h.app.update();
        frames.push((h.arrivals(&player), h.visible_stockade_creatures(&player)));
    }
    let arrived = frames
        .iter()
        .position(|&(arrivals, _)| arrivals == 1)
        .unwrap_or_else(|| panic!("no arrival in {frames:?}"));
    assert!(frames[arrived].1 > 0, "{frames:?}");
    assert!(
        frames[..arrived].iter().all(|&frame| frame == (0, 0)),
        "{frames:?}"
    );
    assert!(frames[arrived..].iter().all(|&(arrivals, _)| arrivals == 1));
}

#[test]
fn the_exit_trigger_returns_the_player_to_stormwind() {
    let Some(mut h) = harness() else { return };
    let player = h.spawn(
        1,
        STORMWIND,
        STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0),
    );
    h.enter_stockade(&player);

    h.walk_to(&player, STOCKADE_EXIT_TRIGGER);
    let new_world = h.new_worlds(&player).pop().expect("NewWorld for map 0");
    assert_eq!(new_world.map_directory, "azeroth");
    h.ack(&player);

    assert_eq!(h.map_of(&player), Some(STORMWIND));
    assert!(close(h.position(&player), STOCKADE_EXIT_TARGET));
}

#[test]
fn grouped_players_enter_one_stockade_copy_and_an_outsider_another() {
    let Some(mut h) = harness() else { return };
    let start = STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0);
    let leader = h.spawn(1, STORMWIND, start);
    let member = h.spawn(2, STORMWIND, start);
    let outsider = h.spawn(3, STORMWIND, start);
    h.group(&[&leader, &member]);

    h.enter_stockade(&leader);
    h.enter_stockade(&member);
    h.enter_stockade(&outsider);

    let copy = h.map_of(&leader).unwrap();
    assert_eq!(h.map_of(&member), Some(copy));
    let other = h.map_of(&outsider).unwrap();
    assert_eq!(other.map_id, STOCKADE);
    assert_ne!(other.instance_id, copy.instance_id);
}

#[test]
fn a_sixth_raid_member_is_refused_at_the_entrance_and_stays_in_stormwind() {
    let Some(mut h) = harness() else { return };
    let start = STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0);
    let members: Vec<Traveler> = (1..=6).map(|id| h.spawn(id, STORMWIND, start)).collect();
    h.group(&members.iter().collect::<Vec<_>>());
    for member in &members[..5] {
        h.enter_stockade(member);
    }

    let sixth = &members[5];
    h.walk_to(sixth, STOCKADE_ENTRANCE_TRIGGER);
    assert_eq!(
        h.aborts(sixth),
        vec![TransferAborted {
            map_id: STOCKADE,
            reason: TransferAbortReason::MaxPlayers,
        }]
    );
    assert!(h.new_worlds(sixth).is_empty());
    assert_eq!(h.map_of(sixth), Some(STORMWIND));
}

/// Stockade MapDifficulty 8 (Normal) → MapDifficultyXCondition 132 → PlayerCondition 74354:
/// ContentTuning 215 (levels 10-30), Flags 0x60 (`WithinOrAboveRecord` | `UseEffectiveLevel`).
fn stockade_level_refusal() -> TransferAborted {
    TransferAborted {
        map_id: STOCKADE,
        reason: TransferAbortReason::Condition(
            "You must be at least level 10 to enter Stormwind Stockade.".into(),
        ),
    }
}

#[test]
fn a_level_one_player_is_refused_at_the_stockade_entrance_and_stays_in_stormwind() {
    let Some(mut h) = harness() else { return };
    let player = h.spawn_at_level(
        1,
        1,
        STORMWIND,
        STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0),
    );
    h.walk_to(&player, STOCKADE_ENTRANCE_TRIGGER);

    assert_eq!(h.aborts(&player), vec![stockade_level_refusal()]);
    assert!(h.new_worlds(&player).is_empty());
    assert_eq!(h.map_of(&player), Some(STORMWIND));
    assert!(close(h.position(&player), STOCKADE_ENTRANCE_TRIGGER));
}

#[test]
fn level_nine_is_refused_and_level_ten_enters_the_stockade() {
    let Some(mut h) = harness() else { return };
    let start = STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0);
    let nine = h.spawn_at_level(1, 9, STORMWIND, start);
    let ten = h.spawn_at_level(2, 10, STORMWIND, start);

    h.walk_to(&nine, STOCKADE_ENTRANCE_TRIGGER);
    assert_eq!(h.aborts(&nine), vec![stockade_level_refusal()]);
    assert_eq!(h.map_of(&nine), Some(STORMWIND));

    let new_world = h.enter_stockade(&ten);
    assert_eq!(new_world.map_id, STOCKADE);
    assert!(h.aborts(&ten).is_empty());
    assert_eq!(h.map_of(&ten).map(|map| map.map_id), Some(STOCKADE));
}

#[test]
fn teleporting_across_continents_moves_the_player_to_kalimdor() {
    let Some(mut h) = harness() else { return };
    let player = h.spawn(1, STORMWIND, STOCKADE_EXIT_TARGET);
    let destination = WorldLocation {
        map_id: 1,
        position: RATCHET,
        orientation: 1.0,
    };
    let player_entity = player.player;
    let accepted = h
        .app
        .world_mut()
        .run_system_once(move |world: &mut World| {
            teleport_to(
                world,
                player_entity,
                destination,
                TeleportOptions::default(),
            )
        })
        .unwrap();
    assert!(accepted);
    assert_eq!(
        h.new_worlds(&player).pop().unwrap().map_directory,
        "kalimdor"
    );
    h.ack(&player);

    assert_eq!(h.map_of(&player), Some(InMap::continent(1)));
    assert!(close(h.position(&player), RATCHET));
}

#[test]
fn spell_teleport_hearthstone_cross_map_uses_new_world_and_world_port_ack() {
    let mut h = harness().unwrap();
    let player = h.spawn(1, STORMWIND, STOCKADE_EXIT_TARGET);
    let destination = WorldLocation {
        map_id: 1,
        position: RATCHET,
        orientation: 1.0,
    };
    let player_entity = player.player;
    let spell = crate::spell_cast::test_harness::world_db()
        .unwrap()
        .store
        .get(8690)
        .unwrap()
        .clone();
    h.app
        .world_mut()
        .entity_mut(player_entity)
        .insert(crate::spell_cast::homebind::HomeBind(destination));
    h.app
        .world_mut()
        .run_system_once(move |world: &mut World| {
            let effect = spell
                .effects
                .iter()
                .find(|effect| effect.effect == 252)
                .unwrap();
            let context = crate::spell_cast::effects::EffectCtx {
                caster: player_entity,
                spell: &spell,
                depth: 0,
                dest: None,
                chain_multiplier: 1.0,
            };
            crate::spell_cast::effects::handler(effect.effect).unwrap()(
                world,
                &context,
                effect,
                player_entity,
            );
        })
        .unwrap();
    assert!(h.app.world().get::<MapTransfer>(player_entity).is_some());
    assert_eq!(
        h.new_worlds(&player).pop().unwrap().map_directory,
        "kalimdor"
    );
    h.ack(&player);
    assert_eq!(h.map_of(&player), Some(InMap::continent(1)));
    assert!(close(h.position(&player), RATCHET));
}

#[test]
fn releasing_in_the_stockade_resurrects_the_player_at_the_stockade_graveyard() {
    let Some(mut h) = harness() else { return };
    let player = h.spawn(
        1,
        STORMWIND,
        STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0),
    );
    h.enter_stockade(&player);
    let map = h.map_of(&player).unwrap();
    let corpse = Corpse {
        player: 1,
        map_id: STOCKADE as u16,
        x: 0.0,
        y: 0.0,
        z: 0.0,
        created_at: 0,
    };
    h.app.world_mut().entity_mut(player.player).insert((
        Health {
            current: 0.0,
            max: 100.0,
        },
        PlayerDeathState {
            state: DeathState::Dead,
            corpse: Some(corpse),
            graveyard: None,
        },
    ));
    let player_entity = player.player;
    let store = map_store().unwrap().clone();
    h.app
        .world_mut()
        .run_system_once(
            move |mut commands: Commands, mut deaths: Query<&mut PlayerDeathState>| {
                let entry = store.map(map.map_id).unwrap().clone();
                let position = Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                };
                crate::death::release_spirit_in_dungeon(
                    player_entity,
                    &position,
                    &mut deaths.get_mut(player_entity).unwrap(),
                    &entry,
                    &store,
                    &mut commands,
                )
                .unwrap();
            },
        )
        .unwrap();
    h.advance(1);
    let new_world = h.new_worlds(&player).pop().expect("NewWorld to map 0");
    assert_eq!(new_world.map_id, 0);
    h.ack(&player);

    assert_eq!(h.map_of(&player), Some(STORMWIND));
    assert!(close(h.position(&player), STOCKADE_GRAVEYARD));
    let world = h.app.world();
    assert_eq!(
        world.get::<PlayerDeathState>(player.player).unwrap().state,
        DeathState::Alive
    );
    assert_eq!(world.get::<Health>(player.player).unwrap().current, 50.0);
}

// --- Difficulties and instance locks (Grim Batol, map 670) ---

const GRIM_BATOL: u32 = 670;
/// `Difficulty.db2` Heroic and (five-player) Mythic.
const DIFFICULTY_HEROIC: u32 = 2;
const DIFFICULTY_MYTHIC: u32 = 23;
/// `areatrigger_teleport` 6179 "Grim Batol Entrance" box center in the Twilight Highlands.
const GRIM_BATOL_ENTRANCE_TRIGGER: Vec3 = Vec3::new(-4042.39, -3444.45, 295.0);
/// `world_safe_locs` 3957 inside Grim Batol.
const GRIM_BATOL_ARRIVAL: Vec3 = Vec3::new(-624.155, -189.365, 272.04);
/// General Umbriss (boss 0, DungeonEncounter 1051, Bit 3).
const UMBRISS: u32 = 39625;
const TROGG_DWELLER: u32 = 39450;
const EASTERN_KINGDOMS: InMap = InMap::continent(0);
/// 2026-09-27 15:00 UTC: the daily reset after `SUNDAY_NOON`.
const SUNDAY_RESET: i64 = 1_790_521_200;

fn entrant_at(character_id: u64, difficulty_id: u32) -> InstanceEntrant {
    InstanceEntrant {
        difficulty_id,
        ..entrant(character_id, None)
    }
}

#[test]
fn grim_batol_offers_three_difficulties_and_the_stockade_downscales_to_normal() {
    let Some(store) = map_store() else { return };
    let heroic = store
        .downscaled_map_difficulty(GRIM_BATOL, DIFFICULTY_HEROIC)
        .unwrap();
    assert_eq!(
        (
            heroic.difficulty_id,
            heroic.reset_interval,
            heroic.is_instance_id_bound()
        ),
        (2, store::ResetInterval::Daily, true)
    );
    let mythic = store
        .downscaled_map_difficulty(GRIM_BATOL, DIFFICULTY_MYTHIC)
        .unwrap();
    assert_eq!(
        (
            mythic.difficulty_id,
            mythic.reset_interval,
            mythic.is_using_encounter_locks()
        ),
        (23, store::ResetInterval::Weekly, true)
    );
    let normal = store
        .downscaled_map_difficulty(GRIM_BATOL, store::DIFFICULTY_NORMAL)
        .unwrap();
    assert!(!normal.has_reset_schedule());
    // The Stockade has Normal only: Heroic and Mythic fall back (Mythic → Heroic → Normal).
    for difficulty in [DIFFICULTY_HEROIC, DIFFICULTY_MYTHIC] {
        assert_eq!(
            store
                .downscaled_map_difficulty(STOCKADE, difficulty)
                .unwrap()
                .difficulty_id,
            store::DIFFICULTY_NORMAL
        );
    }
    assert_eq!(store.difficulty_entries[&DIFFICULTY_MYTHIC].name, "Mythic");
    assert!(
        !store.difficulty_entries[&8].can_select(),
        "Mythic Keystone is not selectable"
    );
    assert_eq!(
        store
            .dungeon_encounter(1051, GRIM_BATOL, DIFFICULTY_HEROIC)
            .map(|e| e.bit),
        Some(3)
    );
}

#[test]
fn a_heroic_copy_is_created_at_heroic_and_normal_is_another_copy() {
    let Some(store) = map_store() else { return };
    let mut instances = InstanceManager::default();
    let heroic = instances
        .create_map(store, GRIM_BATOL, entrant_at(1, DIFFICULTY_HEROIC), at(0.0))
        .unwrap();
    // A change to Normal forgets the (unlocked) Heroic copy; even remembered, it is not entered.
    let normal = instances
        .create_map(
            store,
            GRIM_BATOL,
            entrant_at(1, store::DIFFICULTY_NORMAL),
            at(1.0),
        )
        .unwrap();
    assert_ne!(heroic.map, normal.map);
    assert_eq!(
        instances.copy_difficulty(heroic.map).unwrap().difficulty_id,
        2
    );
    assert_eq!(
        instances.copy_difficulty(normal.map).unwrap().difficulty_id,
        1
    );
    // The Stockade at Heroic is a Normal copy.
    let stockade = instances
        .create_map(store, STOCKADE, entrant_at(1, DIFFICULTY_HEROIC), at(2.0))
        .unwrap();
    assert_eq!(
        instances
            .copy_difficulty(stockade.map)
            .unwrap()
            .difficulty_id,
        1
    );
}

#[test]
fn a_heroic_kill_locks_the_player_to_its_copy_which_comes_back_with_the_boss_dead() {
    let Some(store) = map_store() else { return };
    let mut world = World::new();
    let player = world.spawn_empty().id();
    let mut instances = InstanceManager::default();
    let heroic = entrant_at(1, DIFFICULTY_HEROIC);
    let copy = instances
        .create_map(store, GRIM_BATOL, heroic, at(0.0))
        .unwrap()
        .map;
    instances.add_player(copy, player);
    let saved = instances.update_instance_lock(copy, &[(player, 1)], 1, Some(3), SUNDAY_NOON);
    assert_eq!(saved, vec![player]);
    instances.remove_player(copy, player, at(10.0));
    assert_eq!(
        instances.take_expired(10.0 + INSTANCE_UNLOAD_DELAY_SECS),
        vec![copy]
    );

    // Re-entering creates the same instance id with Umbriss done; he does not spawn.
    let again = instances
        .create_map(store, GRIM_BATOL, heroic, at(2000.0))
        .unwrap();
    assert_eq!((again.map, again.created), (copy, true));
    assert_eq!(instances.initial_done_bosses(copy), 1);
    assert_eq!(
        crate::scripted_ai::instance_script::done_boss_entries(store, GRIM_BATOL, 1),
        vec![UMBRISS]
    );
    // Saved to that copy, the player may not join another group's Heroic copy...
    let other = instances
        .create_map(
            store,
            GRIM_BATOL,
            entrant_at(2, DIFFICULTY_HEROIC),
            at(2001.0),
        )
        .unwrap()
        .map;
    assert_eq!(
        instances.cannot_enter(other, entrant_at(1, DIFFICULTY_HEROIC), SUNDAY_NOON),
        Some(TransferAbortReason::LockedToDifferentInstance)
    );
    // ...but may after the daily reset, and Normal never locks.
    assert_eq!(
        instances.cannot_enter(other, entrant_at(1, DIFFICULTY_HEROIC), SUNDAY_RESET + 1),
        None
    );
    let normal = instances
        .create_map(
            store,
            GRIM_BATOL,
            entrant_at(1, store::DIFFICULTY_NORMAL),
            at(2002.0),
        )
        .unwrap();
    assert!(normal.created);
    assert_eq!(instances.initial_done_bosses(normal.map), 0);
}

#[test]
fn an_expired_lock_resets_its_empty_copy() {
    let Some(store) = map_store() else { return };
    let mut world = World::new();
    let player = world.spawn_empty().id();
    let mut instances = InstanceManager::default();
    let heroic = entrant_at(1, DIFFICULTY_HEROIC);
    let copy = instances
        .create_map(store, GRIM_BATOL, heroic, at(0.0))
        .unwrap()
        .map;
    instances.add_player(copy, player);
    instances.update_instance_lock(copy, &[(player, 1)], 1, Some(3), SUNDAY_NOON);
    let after = instances::Now {
        elapsed: 100.0,
        unix: SUNDAY_RESET + 1,
    };
    // Occupied: the players are told, the copy stays.
    assert_eq!(instances.expire_locked_copies(after), vec![copy]);
    assert!(instances.take_expired(200.0).is_empty());
    // Empty after the reset: it unloads at once and the next entry is a fresh copy.
    instances.remove_player(copy, player, after);
    assert_eq!(instances.take_expired(101.0), vec![copy]);
    let fresh = instances
        .create_map(store, GRIM_BATOL, heroic, after)
        .unwrap();
    assert!(fresh.created);
    assert_ne!(fresh.map, copy);
    assert_eq!(instances.initial_done_bosses(fresh.map), 0);
}

#[test]
fn a_difficulty_change_resets_and_forgets_the_players_unlocked_copies() {
    let Some(store) = map_store() else { return };
    let mut instances = InstanceManager::default();
    let normal = instances
        .create_map(
            store,
            GRIM_BATOL,
            entrant_at(1, store::DIFFICULTY_NORMAL),
            at(0.0),
        )
        .unwrap()
        .map;
    let results = instances.reset_instances(entrant_at(1, store::DIFFICULTY_NORMAL), true, at(5.0));
    assert_eq!(
        results,
        vec![(GRIM_BATOL, instances::InstanceResetResult::Success)]
    );
    assert_eq!(instances.take_expired(6.0), vec![normal]);
    assert!(
        instances
            .create_map(
                store,
                GRIM_BATOL,
                entrant_at(1, store::DIFFICULTY_NORMAL),
                at(7.0)
            )
            .unwrap()
            .created
    );
}

impl Harness {
    fn enter_grim_batol(&mut self, who: &Traveler) {
        self.walk_to(who, GRIM_BATOL_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 40.0));
        self.walk_to(who, GRIM_BATOL_ENTRANCE_TRIGGER);
        let new_world = self.new_worlds(who).pop().expect("NewWorld for map 670");
        assert_eq!(new_world.map_id, GRIM_BATOL);
        self.ack(who);
    }

    fn leave_map(&mut self, who: &Traveler) {
        let player = who.player;
        self.app
            .world_mut()
            .run_system_once(move |world: &mut World| {
                let back = WorldLocation {
                    map_id: 0,
                    position: GRIM_BATOL_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 40.0),
                    orientation: 0.0,
                };
                teleport_to(world, player, back, TeleportOptions::default())
            })
            .unwrap();
        self.new_worlds(who);
        self.ack(who);
    }

    /// (entity, max health) of the creatures of `entry` in `map`.
    fn creature_health(&mut self, map: InMap, entry: u32) -> Vec<(Entity, f32)> {
        let world = self.app.world_mut();
        let mut query = world.query::<(Entity, &shared::components::Npc, &Health, &InMap)>();
        query
            .iter(world)
            .filter(|(_, npc, _, in_map)| **in_map == map && npc.template_id == entry)
            .map(|(entity, _, health, _)| (entity, health.max))
            .collect()
    }
}

#[test]
fn heroic_grim_batol_spawns_heroic_creatures_and_a_boss_kill_keeps_its_copy() {
    let Some(mut h) = harness_with_content_and(|app| {
        app.add_plugins(crate::scripted_ai::ScriptedAiPlugin);
    }) else {
        return;
    };
    let start = GRIM_BATOL_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 40.0);
    let hero = h.spawn(1, EASTERN_KINGDOMS, start);
    let casual = h.spawn(2, EASTERN_KINGDOMS, start);
    h.app
        .world_mut()
        .entity_mut(hero.player)
        .insert(DungeonDifficulty(DIFFICULTY_HEROIC));
    h.enter_grim_batol(&hero);
    h.enter_grim_batol(&casual);
    let (heroic, normal) = (h.map_of(&hero).unwrap(), h.map_of(&casual).unwrap());
    assert_ne!(heroic, normal);
    assert!(close(h.position(&hero), GRIM_BATOL_ARRIVAL));
    // Trogg Dweller: HealthModifier 1.125 at Heroic, 0.4 at Normal (creature_template_difficulty).
    let heroic_troggs = h.creature_health(heroic, TROGG_DWELLER);
    let normal_troggs = h.creature_health(normal, TROGG_DWELLER);
    assert_eq!((heroic_troggs.len(), normal_troggs.len()), (94, 94));
    let ratio = heroic_troggs[0].1 / normal_troggs[0].1;
    assert!((ratio - 1.125 / 0.4).abs() < 0.01, "{ratio}");
    let heroic_umbriss = h.creature_health(heroic, UMBRISS);
    assert_eq!(heroic_umbriss.len(), 1);

    // Umbriss dies (BossAI::_JustDied): the hero is saved.
    let umbriss = heroic_umbriss[0].0;
    h.advance(2);
    h.app
        .world_mut()
        .run_system_once(move |world: &mut World| {
            crate::scripted_ai::dispatch(world, umbriss, crate::scripted_ai::Hook::JustDied(None));
        })
        .unwrap();
    h.advance(2);
    assert_eq!(h.saves.get(&hero.connection).copied(), Some(1));
    let lock = h
        .app
        .world()
        .resource::<InstanceManager>()
        .locks
        .lock(1, (GRIM_BATOL, 0))
        .cloned()
        .expect("heroic lock");
    assert_eq!(
        (
            lock.instance_id,
            lock.difficulty_id,
            lock.data.completed_encounters
        ),
        (heroic.instance_id, 2, 1 << 3)
    );

    // Out, the copy unloads; back in, the same copy without Umbriss.
    h.leave_map(&hero);
    let delay = Duration::from_secs_f64(INSTANCE_UNLOAD_DELAY_SECS + 1.0);
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(delay);
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(delay));
    h.advance(2);
    assert!(h.creature_health(heroic, UMBRISS).is_empty());
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(50),
        ));
    h.enter_grim_batol(&hero);
    assert_eq!(h.map_of(&hero), Some(heroic));
    assert!(
        h.creature_health(heroic, UMBRISS).is_empty(),
        "Umbriss stays dead"
    );
    assert!(
        !h.creatures_in(heroic).is_empty(),
        "the rest of the copy respawned"
    );
}

impl Harness {
    fn request_difficulty(&mut self, who: &Traveler, difficulty_id: u32) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<SetDungeonDifficulty>>(who.connection)
            .unwrap()
            .send::<InstanceChannel>(SetDungeonDifficulty { difficulty_id });
    }

    /// The `DungeonDifficultySet`s `who` received over the next frames.
    fn difficulty_messages(&mut self, who: &Traveler) -> Vec<u32> {
        let mut received = Vec::new();
        for _ in 0..3 {
            self.app.update();
            received.extend(
                self.app
                    .world_mut()
                    .get_mut::<MessageReceiver<DungeonDifficultySet>>(who.connection)
                    .unwrap()
                    .receive()
                    .map(|set| set.difficulty_id),
            );
        }
        received
    }

    fn difficulty_of(&self, who: &Traveler) -> Option<u32> {
        self.app
            .world()
            .get::<DungeonDifficulty>(who.player)
            .map(|d| d.0)
    }
}

#[test]
fn the_player_picks_its_dungeon_difficulty_outside_instances_only() {
    let Some(mut h) = harness() else { return };
    let start = GRIM_BATOL_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 40.0);
    let player = h.spawn(1, EASTERN_KINGDOMS, start);
    h.request_difficulty(&player, DIFFICULTY_HEROIC);
    assert_eq!(h.difficulty_messages(&player), vec![2]);
    assert_eq!(h.difficulty_of(&player), Some(2));
    // Mythic Keystone (8) is not selectable; nothing changes.
    h.request_difficulty(&player, 8);
    assert!(h.difficulty_messages(&player).is_empty());
    // Inside Grim Batol (Heroic copy) the difficulty cannot change.
    h.enter_grim_batol(&player);
    assert_eq!(
        h.app
            .world()
            .resource::<InstanceManager>()
            .copy_difficulty(h.map_of(&player).unwrap())
            .unwrap()
            .difficulty_id,
        2
    );
    h.request_difficulty(&player, DIFFICULTY_MYTHIC);
    assert!(h.difficulty_messages(&player).is_empty());
    assert_eq!(h.difficulty_of(&player), Some(2));
}

#[test]
fn only_a_group_leader_sets_the_groups_difficulty_for_every_member() {
    let Some(mut h) = harness() else { return };
    let start = GRIM_BATOL_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 40.0);
    let leader = h.spawn(1, EASTERN_KINGDOMS, start);
    let member = h.spawn(2, EASTERN_KINGDOMS, start);
    h.group(&[&leader, &member]);
    h.request_difficulty(&member, DIFFICULTY_HEROIC);
    assert!(h.difficulty_messages(&member).is_empty());
    h.request_difficulty(&leader, DIFFICULTY_MYTHIC);
    h.app.update();
    assert_eq!(h.difficulty_messages(&member), vec![23]);
    assert_eq!(
        (h.difficulty_of(&leader), h.difficulty_of(&member)),
        (Some(23), Some(23))
    );
    // Both enter the group's Mythic copy.
    h.enter_grim_batol(&leader);
    h.enter_grim_batol(&member);
    let copy = h.map_of(&leader).unwrap();
    assert_eq!(h.map_of(&member), Some(copy));
    assert_eq!(
        h.app
            .world()
            .resource::<InstanceManager>()
            .copy_difficulty(copy)
            .unwrap()
            .difficulty_id,
        23
    );
}
