//! Whole-copy credit/reload boundaries, not traversal or complete-fight proof.
use super::*;

const PRIMARIES: [u32; 4] = [61408, 61412, 61463, 61528];
const ENCOUNTERS: [u32; 4] = [1443, 1444, 1445, 1446];
const ALL_DONE: u32 = 0b1111;

fn entrant(character_id: u64) -> crate::maps::instances::InstanceEntrant {
    crate::maps::instances::InstanceEntrant {
        character_id,
        leader: character_id,
        difficulty_id: 1,
        ..Default::default()
    }
}

fn now() -> crate::maps::instances::Now {
    crate::maps::instances::Now {
        elapsed: 0.0,
        unix: 1000,
    }
}

fn spawn_copy_actors(app: &mut App, content: crate::zones::MapContent) -> Vec<u32> {
    let map = content.map;
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    crate::zones::spawn_map_creatures(
        &mut commands,
        app.world().resource::<WorldData>(),
        content,
        false,
        Some(&mut crate::test_data::ground_map()),
    );
    queue.apply(app.world_mut());
    let world = app.world_mut();
    world
        .query::<(&shared::components::Npc, &InMap)>()
        .iter(world)
        .filter(|(_, actor_map)| **actor_map == map)
        .map(|(npc, _)| npc.template_id)
        .collect()
}

fn reload_normal_copy(app: &mut App, map: InMap) -> crate::zones::MapContent {
    use crate::maps::{InstanceManager, locks::InstanceLocks, persistence};
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("ragefire.redb");
    {
        let database = redb::Database::create(&file).unwrap();
        let instances = app.world().resource::<InstanceManager>();
        let normal: Vec<_> = instances.normal_locks.values().cloned().collect();
        persistence::save_instance_locks(&database, &instances.locks.saved(), &normal);
    }
    let database = redb::Database::open(&file).unwrap();
    let saved = persistence::load_instance_locks(&database);
    let normal = persistence::load_normal_instance_locks(&database);
    assert_eq!(normal.len(), 1, "only the completed Normal copy is saved");
    let mut instances =
        InstanceManager::with_locks_and_normal(InstanceLocks::from_saved(saved), normal);
    let restored = instances
        .create_map(
            app.world().resource::<crate::maps::MapStore>(),
            389,
            entrant(2001),
            now(),
        )
        .unwrap();
    assert_eq!(restored.map, map, "same owner resumes the saved copy");
    assert_eq!(instances.completed_encounters(map), ALL_DONE);
    let done = instances.normal_locks[&map.instance_id].data.done_bosses;
    let dead_bosses = instance_script::done_boss_entries(
        app.world().resource::<crate::maps::MapStore>(),
        389,
        done,
    );
    app.insert_resource(instances);
    crate::zones::MapContent {
        map,
        difficulty_id: 1,
        dead_bosses,
    }
}

#[test]
fn ragefire_completion_four_real_deaths_persist_credit_and_repopulate_without_bosses() {
    let (mut h, _, map) =
        encounter_fixture(SLAGMAW, 389, 2, 1, None, EncounterState::NotStarted, true);
    let mut bosses = Vec::new();
    for (bit, entry) in PRIMARIES.into_iter().enumerate() {
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::MapStore>()
                .raid_creature_encounters[&(389, entry)],
            bit as u32
        );
        let actors = h.creatures_of(entry);
        assert_eq!(actors.len(), 1, "authored primary {entry}");
        bosses.push(actors[0]);
    }
    for (bit, boss) in bosses.iter().copied().enumerate() {
        let position = *h.app.world().get::<Position>(boss).unwrap();
        let player = h.player();
        h.app.world_mut().entity_mut(player).insert(Position {
            x: position.x + 3.0,
            ..position
        });
        h.engage(boss);
        h.advance(0.1);
        assert_eq!(
            get_boss_state(h.app.world_mut(), map, bit as u32),
            Some(EncounterState::InProgress)
        );
        // Controlled lethal Fireball uses real damage/death callbacks, not SetBossState.
        // It intentionally does not certify ordinary party balance or every mechanic.
        h.hit_pct(boss, 1_000.0);
        h.advance(0.1);
        assert_eq!(
            get_boss_state(h.app.world_mut(), map, bit as u32),
            Some(EncounterState::Done)
        );
        let expected = (1 << (bit + 1)) - 1;
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            expected
        );
        assert!(
            h.inbox(0)
                .ends
                .iter()
                .any(|end| end.encounter_id == ENCOUNTERS[bit] && end.success)
        );
    }
    assert!(h.inbox(1).starts.is_empty());
    assert!(h.inbox(1).ends.is_empty());
    assert!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .locks
            .saved()
            .locks
            .is_empty(),
        "Normal dungeon progress is not a scheduled raid/heroic lock"
    );
    for boss in bosses {
        h.app.world_mut().despawn(boss);
    }
    let content = reload_normal_copy(&mut h.app, map);
    let entries = spawn_copy_actors(&mut h.app, content);
    assert!(!entries.is_empty(), "living dungeon population remains");
    assert!(
        PRIMARIES.iter().all(|entry| !entries.contains(entry)),
        "completed bosses stay absent after persistence reload"
    );
    let store = h.app.world().resource::<crate::maps::MapStore>().clone();
    let fresh = h
        .app
        .world_mut()
        .resource_mut::<crate::maps::InstanceManager>()
        .create_map(&store, 389, entrant(2002), now())
        .unwrap()
        .map;
    assert_ne!(fresh, map);
    let entries = spawn_copy_actors(
        &mut h.app,
        crate::zones::MapContent {
            map: fresh,
            difficulty_id: 1,
            dead_bosses: vec![],
        },
    );
    assert!(
        PRIMARIES.iter().all(|entry| entries.contains(entry)),
        "fresh owner has every primary"
    );
}

fn trigger_adarogg_intro(navigation: &std::path::Path) -> (Harness, Entity) {
    let (mut h, _, _) =
        encounter_fixture(ADAROGG, 389, 0, 1, None, EncounterState::NotStarted, true);
    h.app
        .insert_resource(crate::navigation::Navigation::new(navigation));
    let boss = h.creatures_of(ADAROGG)[0];
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (x, y, z): (f32, f32, f32) = conn
        .query_row(
            "SELECT Pos_0,Pos_1,Pos_2 FROM area_trigger WHERE ID=7904",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
    let player = h.player();
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(Position { x, y, z });
    (h, boss)
}

#[test]
fn ragefire_completion_adarogg_intro_unbaked_navigation_cannot_fake_path_arrivals() {
    let directory = tempfile::tempdir().unwrap();
    let (mut h, boss) = trigger_adarogg_intro(directory.path());
    h.advance(60.0);
    assert_eq!(spell_gos(&h, boss, 120113).len(), 3);
    assert!(
        spell_gos(&h, boss, 120064).is_empty(),
        "no baked route, no arrival eats"
    );
    let second_point = Vec3::new(-281.1894, -54.73433, -60.34256);
    assert!(
        h.wow_position(boss).distance(second_point) < 0.01,
        "only the two source MovePoints ran; no direct MovePath shortcut"
    );
}

#[test]
#[ignore = "requires RAGEFIRE_NAV_ROOT containing the authenticated map389 bake"]
fn ragefire_completion_adarogg_intro_walks_source_paths_and_eats_without_arrival_injection() {
    let navigation = std::path::PathBuf::from(std::env::var_os("RAGEFIRE_NAV_ROOT").unwrap());
    let (mut h, boss) = trigger_adarogg_intro(&navigation);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    h.app.insert_resource(crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        std::collections::HashMap::from([(389, maps.map(389).unwrap().wdt_file_data_id)]),
    ));
    let meals: Vec<Vec<_>> = [61659, 61669]
        .into_iter()
        .map(|entry| {
            h.creatures_of(entry)
                .into_iter()
                .map(Entity::to_bits)
                .collect()
        })
        .collect();
    // Real trigger, scheduler, route movement and arrival callbacks. No point/path
    // completion hook is dispatched by the fixture, and the boss is never repositioned.
    let spawn = h.wow_position(boss);
    h.advance(6.0);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::CreatureMotion>(boss),
        Some(&shared::components::CreatureMotion::Walk),
        "TDB path MoveType0 walks"
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::MovementSpeed>(boss)
            .unwrap()
            .0,
        2.5,
        "authored walk speed, not the actor's preceding run mode"
    );
    // Three actual walk paths plus 6148/6996ms source waits and the final meal
    // repeat exceed60s even before route detours. Wait for completion, bounded90s.
    let mut elapsed = 6.0;
    while spell_gos(&h, boss, 120064).len() < 6 && elapsed < 90.0 {
        h.advance(0.05);
        elapsed += 0.05;
    }
    assert_eq!(
        spell_gos(&h, boss, 120113).len(),
        3,
        "three source chow tasks"
    );
    let eats = spell_gos(&h, boss, 120064);
    assert!(
        eats.len() >= 6,
        "three authored path arrivals must each eat twice: {}",
        eats.len()
    );
    assert!(
        h.wow_position(boss).distance(spawn) > 5.0,
        "boss actually traversed its route"
    );
    // TDB Source13/120064 has OR groups: Slain Trogg61659 or Oggleflint61669.
    // PATH_INTRO/PATH_INTRO3 approach Oggleflint; PATH_INTRO2 approaches the troggs.
    let hits: Vec<_> = eats.iter().flat_map(|eat| &eat.hit_targets).collect();
    assert!(
        hits.len() >= 2,
        "walked path reaches the actual Slain Trogg meals: {eats:?}"
    );
    assert!(
        hits.iter()
            .all(|target| meals.iter().any(|group| group.contains(target))),
        "every selected target satisfies an authored meal condition: {eats:?}"
    );
    assert!(
        meals
            .iter()
            .all(|group| hits.iter().any(|target| group.contains(target))),
        "walked route reaches both Oggleflint and Slain Trogg meals: {eats:?}"
    );
    assert_eq!(
        h.inbox(0)
            .chat
            .iter()
            .filter(|chat| chat.content.contains("He's cornered!"))
            .count(),
        1
    );
}

#[test]
fn ragefire_completion_normal_loot_uses_each_authored_boss_pool_and_money_range() {
    use rand::SeedableRng;
    let path = world_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let tables = crate::loot::load_loot_tables(&conn).unwrap();
    let items = crate::item_templates::load_item_templates(&path).unwrap();
    // TDB1210 boss references; normalized content namespaces preserve these pools.
    let pools: [&[u32]; 4] = [
        &[82772, 82879, 82880, 151421, 151422],
        &[82877, 82881, 82882, 132551],
        &[82878, 82884, 82885, 132552],
        &[82883, 82886, 82888, 151424, 151425],
    ];
    for (bit, entry) in PRIMARIES.into_iter().enumerate() {
        for seed in 0..16 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            let loot = tables.roll_creature(entry, 1, &items, |_| false, &mut rng);
            let money = if entry == 61528 {
                160000..=170000
            } else {
                65000..=75000
            };
            assert!(money.contains(&loot.copper), "{entry}: {loot:?}");
            assert!((1..=2).contains(&loot.items.len()), "{entry}: {loot:?}");
            assert!(
                loot.items
                    .iter()
                    .all(|item| pools[bit].contains(&item.item_id) && item.count == 1),
                "{entry}: {loot:?}"
            );
        }
    }
}
