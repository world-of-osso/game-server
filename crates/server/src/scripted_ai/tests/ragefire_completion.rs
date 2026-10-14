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
