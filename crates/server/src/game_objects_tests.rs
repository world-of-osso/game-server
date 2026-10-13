//! Real-DB tests against data/world.db.

use rusqlite::OpenFlags;

use super::*;

/// Stormwind bank Guild Vault (TrinityCore entry 205107, guid 220030, display 7607); Retail
/// Stormwind has one entry per vault, 205104-205107.
const STORMWIND_VAULT: u32 = 205107;
const STORMWIND_VAULTS: std::ops::RangeInclusive<u32> = 205104..=205107;

fn spawns() -> Option<GameObjectSpawns> {
    let path = crate::test_data::world_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    Some(load_game_object_spawns(&conn).unwrap())
}

#[test]
fn core_fragment_authored_normal_copy_replicates_only_to_its_observer() {
    use crate::interest::{InterestPlugin, NetworkVisibility};
    use crate::maps::InMap;
    use crate::networking::PlayerLink;
    use lightyear::prelude::ReplicationSender;
    let spawns = spawns().expect("real world game objects");
    let core = spawns
        .0
        .iter()
        .find(|spawn| spawn.info.entry == 179553 && spawn.map_id == 230)
        .unwrap();
    let position = game_object_bundle(core).0;
    // Only the Core Fragment: map 230's gathering nodes need the full WorldData pool state.
    let spawns = GameObjectSpawns(vec![core.clone()]);
    let copy = InMap {
        map_id: 230,
        instance_id: 1,
    };
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: std::time::Duration::from_millis(50),
    });
    app.add_plugins(InterestPlugin);
    let player = app.world_mut().spawn((position, copy)).id();
    let connection = app
        .world_mut()
        .spawn((PlayerLink(player), ReplicationSender))
        .id();
    let outsider = app
        .world_mut()
        .spawn((
            position,
            InMap {
                map_id: 230,
                instance_id: 2,
            },
        ))
        .id();
    let other_connection = app
        .world_mut()
        .spawn((PlayerLink(outsider), ReplicationSender))
        .id();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    spawn_map_game_objects(&mut commands, &spawns, copy, 1);
    queue.apply(app.world_mut());
    app.world_mut().run_schedule(FixedUpdate);
    let mut objects = app
        .world_mut()
        .query::<(&GameObjectInfo, &InMap, &NetworkVisibility)>();
    let visible = objects
        .iter(app.world())
        .find(|(info, map, _)| info.entry == 179553 && **map == copy)
        .unwrap()
        .2;
    assert!(
        visible.is_visible(connection),
        "authored Core Fragment difficulty1 reaches its copy's client"
    );
    assert!(
        !visible.is_visible(other_connection),
        "same coordinates in another copy do not reveal it"
    );
}

#[test]
fn eastern_kingdoms_guild_vaults_load_with_retail_display_boxes() {
    let Some(spawns) = spawns() else { return };
    // Stormwind 4, Ironforge 2, Undercity 4, Booty Bay 2 (TrinityCore map 0 type 34).
    let vaults = spawns
        .0
        .iter()
        .filter(|s| s.map_id == 0 && s.info.go_type == 34)
        .count();
    assert_eq!(vaults, 12);
    let stormwind: Vec<_> = spawns
        .0
        .iter()
        .filter(|s| STORMWIND_VAULTS.contains(&s.info.entry))
        .collect();
    assert_eq!(stormwind.len(), 4);
    let vault = stormwind
        .iter()
        .find(|s| s.info.entry == STORMWIND_VAULT)
        .unwrap();
    assert_eq!(vault.info.name, "Guild Vault");
    assert_eq!(vault.info.display_id, 7607);
    // Retail GameObjectDisplayInfo 7607 GeoBox.
    assert!((vault.bounds.min.y - -1.9534).abs() < 1e-3);
    assert!((vault.bounds.max.z - 5.8225).abs() < 1e-3);
}

#[test]
fn gathering_nodes_load_authored_herb_and_ore_models() {
    let spawns = spawns().unwrap();
    for (entry, name, display) in [(1617, "Silverleaf", 270), (1731, "Copper Vein", 310)] {
        let node = spawns
            .0
            .iter()
            .find(|spawn| spawn.info.entry == entry && spawn.map_id == 0)
            .expect("Retail herb and ore nodes must load with authored positions");
        assert_eq!(node.info.name, name);
        assert_eq!(node.info.display_id, display);
        assert_eq!(node.info.go_type, 50);
    }
}

#[test]
#[ignore = "requires DUNGEON_POPULATION_DB pointing to an imported database copy"]
fn dungeon_population_loads_all_model_bearing_instance_gameobjects() {
    let path = std::env::var("DUNGEON_POPULATION_DB").expect("imported copy path");
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let spawns = load_game_object_spawns(&conn).unwrap();
    let mut statement = conn
        .prepare(
            "SELECT g.map,count(*) FROM content_gameobject g
         JOIN content_gameobject_template t ON t.entry=g.id
         JOIN game_object_display_info d ON d.ID=t.displayId
         WHERE g.map IN (SELECT map FROM tdb_instance_template) GROUP BY g.map",
        )
        .unwrap();
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, usize>(1)?))
        })
        .unwrap();
    for row in rows {
        let (map, expected) = row.unwrap();
        let actual = spawns.0.iter().filter(|spawn| spawn.map_id == map).count();
        assert_eq!(actual, expected, "map {map}");
    }
}

#[test]
fn follower_normal_population_spawns_normal_gameobjects_in_205_copy() {
    let normal = GameObjectSpawn {
        initial_state: 1,
        guid: 1,
        map_id: 2521,
        info: GameObjectInfo {
            entry: 206038,
            go_type: GAMEOBJECT_TYPE_GENERIC,
            display_id: 200,
            name: "Small Fire".into(),
            scale: 0.5,
        },
        position: Vec3::new(1386.2, -169.719, 131.0),
        orientation: 0.0,
        bounds: GameObjectBounds {
            min: Vec3::ZERO,
            max: Vec3::ONE,
            scale: 0.5,
            orientation: 0.0,
        },
        spawn_difficulties: vec![1, 2, 23],
        chair: None,
        quest_object: None,
    };
    let mut mythic = normal.clone();
    mythic.guid = 2;
    mythic.spawn_difficulties = vec![8, 23];
    let spawns = GameObjectSpawns(vec![normal, mythic]);
    let mut world = World::new();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, &world);
    let map = crate::maps::InMap {
        map_id: 2521,
        instance_id: 7,
    };
    assert_eq!(spawn_map_game_objects(&mut commands, &spawns, map, 205), 1);
    queue.apply(&mut world);
    assert_eq!(world.query::<&GameObjectInfo>().iter(&world).count(), 1);
    assert_eq!(
        *world.query::<&crate::maps::InMap>().single(&world).unwrap(),
        map
    );
}

#[test]
fn reach_is_the_model_box_grown_by_five_yards() {
    let bounds = GameObjectBounds {
        min: Vec3::new(-0.7, -1.95, 0.0),
        max: Vec3::new(1.1, 1.93, 5.8),
        scale: 1.0,
        orientation: 0.0,
    };
    let origin = Vec3::new(-8934.91, 618.27, 100.59);
    // 6 yd in front of the box face at x = 1.1: inside 1.1 + 5.
    assert!(bounds.reaches(origin, origin + Vec3::new(6.0, 0.0, 0.0), 5.0));
    assert!(!bounds.reaches(origin, origin + Vec3::new(6.2, 0.0, 0.0), 5.0));
    // Turned a quarter: the long y extent now lies along world x.
    let turned = GameObjectBounds {
        orientation: FRAC_PI_2,
        ..bounds
    };
    assert!(turned.reaches(origin, origin + Vec3::new(6.9, 0.0, 0.0), 5.0));
    assert!(!bounds.reaches(origin, origin + Vec3::new(0.0, 7.0, 0.0), 5.0));
}

#[test]
fn bundle_places_the_vault_in_server_space() {
    let Some(spawns) = spawns() else { return };
    let vault = &spawns.0[0];
    let (position, _, info, _, _) = game_object_bundle(vault);
    assert_eq!(wow_position(&position), vault.position);
    assert_eq!(info, vault.info);
}

const GOLDSHIRE_MAILBOX: u32 = 20_209_585;

#[test]
fn eastern_kingdoms_mailboxes_spawn_with_retail_display_boxes() {
    let Some(spawns) = spawns() else { return };
    // TrinityCore map 0 base-phase (PhaseId 0: 75, DEFAULT_PHASE 169: 3) type 19 rows with a
    // Retail GameObjectDisplayInfo box.
    let mailboxes: Vec<_> = spawns
        .0
        .iter()
        .filter(|s| s.map_id == 0 && s.info.go_type == 19)
        .collect();
    assert_eq!(mailboxes.len(), 78);
    // The Lion's Pride Inn mailbox in Goldshire (TrinityCore guid 209585 + 20,000,000).
    let goldshire = mailboxes
        .iter()
        .find(|s| s.guid == GOLDSHIRE_MAILBOX)
        .unwrap();
    assert_eq!(goldshire.info.name, "Mailbox");
    assert_eq!(goldshire.info.display_id, 1907);
    assert!((goldshire.position.x - -9455.99).abs() < 1e-2);
    assert!((goldshire.bounds.max.z - 3.0802).abs() < 1e-3);
}

#[test]
fn stockade_fires_and_bonfire_spawn_as_decoration() {
    let Some(spawns) = spawns() else { return };
    // TDB1210 map 34 (DEFAULT_PHASE 169): 23 Small Fire (0.5), 10 Medium Fire (0.75), 1 Bonfire.
    let stockade: Vec<_> = spawns.0.iter().filter(|s| s.map_id == 34).collect();
    let count = |entry: u32| stockade.iter().filter(|s| s.info.entry == entry).count();
    assert_eq!((count(206038), count(206039), count(206117)), (23, 10, 1));
    assert_eq!(stockade.len(), 34);
    let bonfire = stockade.iter().find(|s| s.info.entry == 206117).unwrap();
    assert_eq!(bonfire.info.go_type, GAMEOBJECT_TYPE_SPELL_FOCUS);
    assert_eq!(bonfire.info.display_id, 200);
    assert!((bonfire.position.x - 80.1872).abs() < 1e-3);
    assert!(!bonfire.info.is_usable());
    let fire = stockade.iter().find(|s| s.info.entry == 206038).unwrap();
    assert_eq!(fire.info.go_type, GAMEOBJECT_TYPE_GENERIC);
    assert!(!fire.info.is_usable());
}
