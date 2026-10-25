//! Controlled deaths prove Normal run persistence, not fights or escort completion.
use super::*;
use crate::maps::{InstanceManager, instances, locks::InstanceLocks, persistence};

const WAILING_MAP: u32 = 43;
const NORMAL: u32 = 1;
const OWNER: u64 = 2001;
const FRESH_OWNER: u64 = 2002;
const FULL_PROGRESS: u32 = 0xff;
const PRIMARY_ENTRIES: [u32; 8] = [3671, 3669, 3653, 3670, 3674, 3673, 5775, 3654];
const ENCOUNTERS: [u32; 8] = [585, 586, 587, 588, 589, 590, 591, 592];
const STATIC_PRIMARIES: [u32; 7] = [3671, 3669, 3653, 3670, 3674, 3673, 5775];
const DISCIPLE: u32 = 3678;
const MUTANUS: u32 = 3654;

fn complete_controlled_wailing_run(h: &mut Harness, map: InMap) {
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 1_000_000_000.0,
        max: 1_000_000_000.0,
    });
    for entry in STATIC_PRIMARIES {
        let bosses = h.creatures_of(entry);
        assert_eq!(bosses.len(), 1, "one authored primary {entry}");
        h.engage(bosses[0]);
        h.hit_pct(bosses[0], 1_000.0);
        h.advance(0.1);
    }
    let boss = summon_authored_mutanus(h, map);
    h.engage(boss);
    h.hit_pct(boss, 1_000.0);
    h.advance(0.1);
    assert_full_wailing_credit(h, map);
}

fn summon_authored_mutanus(h: &mut Harness, map: InMap) -> Entity {
    let disciples = h.creatures_of(DISCIPLE);
    assert_eq!(disciples.len(), 1);
    // Stage only the source group4 callback; this does not prove the escort.
    let summons = Ctx {
        world: h.app.world_mut(),
        me: disciples[0],
    }
    .summon_creature_group(4)
    .expect("authored Disciple group4");
    assert_eq!(summons.len(), 1);
    let boss = summons[0];
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Npc>(boss)
            .unwrap()
            .template_id,
        MUTANUS
    );
    assert_eq!(h.app.world().get::<InMap>(boss), Some(&map));
    boss
}

fn assert_full_wailing_credit(h: &Harness, map: InMap) {
    for encounter in ENCOUNTERS {
        assert_eq!(
            h.inbox(0)
                .ends
                .iter()
                .filter(|end| end.encounter_id == encounter && end.success)
                .count(),
            1,
            "each primary grants its own encounter once"
        );
    }
    assert!(h.inbox(1).ends.is_empty(), "other copy gets no completion");
    assert_eq!(
        h.app
            .world()
            .resource::<InstanceManager>()
            .completed_encounters(map),
        FULL_PROGRESS
    );
}

fn reopen_wailing_progress(h: &Harness, file: &std::path::Path) -> InstanceManager {
    {
        let database = redb::Database::create(file).unwrap();
        let instances = h.app.world().resource::<InstanceManager>();
        let normal: Vec<_> = instances.normal_locks.values().cloned().collect();
        persistence::save_instance_locks(&database, &instances.locks.saved(), &normal);
    }
    let database = redb::Database::open(file).unwrap();
    let saved = persistence::load_instance_locks(&database);
    let normal = persistence::load_normal_instance_locks(&database);
    assert!(saved.locks.is_empty(), "Normal is not a scheduled lock");
    assert!(saved.shared.is_empty());
    assert_eq!(normal.len(), 1);
    InstanceManager::with_locks_and_normal(InstanceLocks::from_saved(saved), normal)
}

#[test]
fn encounter_coverage_wailing_eight_controlled_deaths_reopen_normal_run() {
    let (mut h, _, map) = encounter_fixture(
        3669,
        WAILING_MAP,
        1,
        NORMAL,
        None,
        EncounterState::NotStarted,
        true,
    );
    complete_controlled_wailing_run(&mut h, map);
    let directory = tempfile::tempdir().unwrap();
    let mut instances = reopen_wailing_progress(&h, &directory.path().join("wailing.redb"));
    let entrant = instances::InstanceEntrant {
        character_id: OWNER,
        leader: OWNER,
        difficulty_id: NORMAL,
        ..Default::default()
    };
    let now = instances::Now {
        elapsed: 0.0,
        unix: 1000,
    };
    let maps = h.app.world().resource::<crate::maps::MapStore>();
    assert_eq!(
        instances
            .create_map(maps, WAILING_MAP, entrant, now)
            .unwrap()
            .map,
        map
    );
    assert_eq!(instances.completed_encounters(map), FULL_PROGRESS);
    let done = instances.normal_locks[&map.instance_id].data.done_bosses;
    let mut dead_bosses = instance_script::done_boss_entries(maps, WAILING_MAP, done);
    dead_bosses.sort_unstable();
    let mut expected = PRIMARY_ENTRIES.to_vec();
    expected.sort_unstable();
    assert_eq!(dead_bosses, expected);
    let fresh_entrant = instances::InstanceEntrant {
        character_id: FRESH_OWNER,
        leader: FRESH_OWNER,
        ..entrant
    };
    let fresh = instances
        .create_map(maps, WAILING_MAP, fresh_entrant, now)
        .unwrap()
        .map;
    assert_ne!(fresh, map);
    assert_eq!(instances.completed_encounters(fresh), 0);
    let existing: std::collections::HashSet<_> = h
        .app
        .world_mut()
        .query_filtered::<Entity, With<shared::components::Npc>>()
        .iter(h.app.world())
        .collect();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    crate::zones::spawn_map_creatures(
        &mut commands,
        h.app.world().resource::<WorldData>(),
        crate::zones::MapContent {
            map,
            difficulty_id: NORMAL,
            dead_bosses,
        },
        false,
        Some(&mut crate::test_data::ground_map()),
    );
    queue.apply(h.app.world_mut());
    let population: Vec<_> = h
        .app
        .world_mut()
        .query::<(Entity, &shared::components::Npc, &InMap)>()
        .iter(h.app.world())
        .filter(|(entity, _, location)| !existing.contains(entity) && **location == map)
        .map(|(_, npc, _)| npc.template_id)
        .collect();
    assert!(
        !population.is_empty(),
        "unrelated authored population survives"
    );
    assert!(
        population.contains(&DISCIPLE),
        "Disciple is not a defeated primary"
    );
    assert!(
        population
            .iter()
            .all(|entry| !PRIMARY_ENTRIES.contains(entry)),
        "all defeated primaries filtered on reopen"
    );
}
