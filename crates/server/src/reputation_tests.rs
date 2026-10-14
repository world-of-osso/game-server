use super::*;
use crate::world_data::{CreatureTemplate, WorldData};
use shared::reputation::Standing;
use std::collections::HashMap;

fn test_world_data(template: CreatureTemplate) -> WorldData {
    let mut templates = HashMap::new();
    templates.insert(template.entry, template);
    WorldData {
        templates,
        spawns: Vec::new(),
        spawns_by_map: HashMap::new(),
        waypoints: HashMap::new(),
        ..Default::default()
    }
}

fn test_template(entry: u32, faction: u16, min_level: u8, max_level: u8) -> CreatureTemplate {
    CreatureTemplate {
        entry,
        name: "Fixture wolf".into(),
        subname: String::new(),
        creature_type: 1,
        family: 0,
        type_flags: 0,
        flags_extra: 0,
        min_level,
        max_level,
        faction,
        npc_flags: 0,
        speed_walk: 1.0,
        speed_run: 1.0,
        health_modifier: 1.0,
        unit_class: 1,
        expansion: 0,
        rank: 0,
        experience_modifier: 1.0,
        gives_experience: true,
        script_name: String::new(),
        detection_range: 20.0,
        aggro_mode: crate::creature_aggro::AggroMode::Aggressive,
        trigger: false,
        unit_flags: 0,
        models: Vec::new(),
        tuning: None,
        armor: 0.0,
        mana_modifier: 1.0,
        regenerates_power: false,
        regenerates_health: true,
    }
}

#[test]
fn npc_kill_reputation_waits_for_fifo_and_preserves_fresh_durable_state() {
    use lightyear::connection::host::HostClient;
    use shared::components::{CharacterAppearance, Player};
    use std::time::{Duration, Instant};

    let db = persistence::temp_database();
    let registry = default_faction_registry();
    let mut durable = CharacterReputation::new_for_race(&registry, 1);
    durable.set_rep(72, 100);
    persistence::save_reputation(&db, 42, &durable);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<ReputationStateUpdate>();
    app.insert_resource(GameDatabase::new(db));
    app.insert_resource(test_world_data(test_template(1001, 72, 5, 5)));
    app.add_plugins(ReputationPlugin);
    app.finish();
    app.cleanup();
    let position = Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let map = crate::maps::InMap::continent(0);
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "RepFixture".into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            ClientId(42),
            Level(5),
            position,
            map,
            CharacterReputation::new_for_race(&registry, 1),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            Connected,
            RemoteId(PeerId::Local(1)),
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: 1 },
            InWorld { character_id: 42 },
            PlayerLink(player),
            MessageSender::<ReputationStateUpdate>::default(),
            MessageReceiver::<ReputationStateUpdate>::default(),
        ))
        .id();
    for _ in 0..3 {
        app.update();
    }
    app.world_mut()
        .get_mut::<MessageReceiver<ReputationStateUpdate>>(connection)
        .unwrap()
        .receive()
        .for_each(drop);
    let held = app
        .world()
        .resource::<GameDatabase>()
        .db
        .begin_write()
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    let started = Instant::now();
    for _ in 0..2 {
        app.world_mut().spawn((
            Npc {
                template_id: 1001,
                name: "Fixture wolf".into(),
            },
            CreatureTap::tapped_by(42),
            position,
            map,
            DeathState {
                respawn_timer: 10.0,
            },
        ));
        app.update();
    }
    let elapsed = started.elapsed();
    let premature: Vec<_> = app
        .world_mut()
        .get_mut::<MessageReceiver<ReputationStateUpdate>>(connection)
        .unwrap()
        .receive()
        .collect();
    release.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(50),
        "reputation tick waited: {elapsed:?}"
    );
    assert!(premature.is_empty(), "reputation snapshot precedes commit");
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut committed_values = Vec::new();
    while committed_values.len() < 2 {
        app.update();
        let responses: Vec<_> = app
            .world_mut()
            .get_mut::<MessageReceiver<ReputationStateUpdate>>(connection)
            .unwrap()
            .receive()
            .collect();
        for response in responses {
            assert!(response.error.is_none());
            let snapshot = response.snapshot.unwrap();
            committed_values.push(
                snapshot
                    .entries
                    .iter()
                    .find(|entry| entry.faction_id == 72)
                    .unwrap()
                    .value,
            );
        }
        assert!(Instant::now() < deadline, "no committed reputation replies");
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(committed_values, vec![110, 120]);
    assert_eq!(
        app.world()
            .get::<CharacterReputation>(player)
            .unwrap()
            .get_value(72),
        120
    );
    let persisted =
        persistence::load_reputation(&app.world().resource::<GameDatabase>().db, 42, || {
            panic!("durable reputation missing")
        });
    assert_eq!(persisted.get_value(72), 120);
}

#[test]
fn npc_kill_updates_primary_and_spillover_reputation() {
    let mut registry = default_faction_registry();
    // Explicit test policy: the authentic registry does not invent city spillover.
    let mut stormwind_policy = registry.get(72).unwrap().clone();
    stormwind_policy.spillover = vec![shared::reputation::SpilloverRule {
        faction_id: 47,
        rate: 0.25,
    }];
    registry.add(stormwind_policy);
    let world_data = test_world_data(test_template(1001, 72, 5, 5));
    let mut reputation = CharacterReputation::new_for_race(&registry, 1);
    // Pinned race-class baselines: Human Stormwind/Ironforge/Orgrimmar start values.
    let [stormwind, ironforge, orgrimmar] = [72, 47, 76].map(|id| reputation.get_value(id));

    let message = apply_npc_kill_reputation(&mut reputation, 5, 1001, &world_data, &registry)
        .expect("kill should award reputation");

    assert_eq!(reputation.get_value(72), stormwind + 10);
    assert_eq!(reputation.get_value(47), ironforge + 2);
    assert_eq!(reputation.get_value(76), orgrimmar);
    assert!(message.contains("Stormwind"));
    assert!(message.contains("spillover"));
}

#[test]
fn grey_kill_grants_no_reputation() {
    let registry = default_faction_registry();
    let world_data = test_world_data(test_template(1001, 72, 5, 5));
    let mut reputation = CharacterReputation::new_for_race(&registry, 1);
    let stormwind = reputation.get_value(72);

    let message = apply_npc_kill_reputation(&mut reputation, 20, 1001, &world_data, &registry);

    assert!(message.is_none());
    assert_eq!(reputation.get_value(72), stormwind);
}

/// Seed only the assigned private native-proof characters; never production/gameplay eligibility.
#[test]
#[ignore = "requires an offline owned private DB and explicit proof character"]
fn private_reputation_bar_fixture_persists_standard_watch() {
    let path = std::path::PathBuf::from(
        std::env::var("REPUTATION_BAR_PRIVATE_DB").expect("explicit private DB required"),
    );
    assert_eq!(
        path.canonicalize().unwrap(),
        std::path::PathBuf::from("/home/osso/.worktrees/adventure-guide4-proof/private.redb")
    );
    let name = std::env::var("REPUTATION_BAR_PRIVATE_CHARACTER")
        .expect("explicit private character required");
    assert!(matches!(name.as_str(), "Guidefourm" | "Guidefourf"));
    let db = redb::Database::open(&path).expect("owned server must be stopped before seeding");
    let character = crate::persistence_character::find_character_id_by_name(&db, &name)
        .expect("private character exists");
    let mut reputation =
        crate::persistence::load_reputation(&db, character, CharacterReputation::default);
    reputation.set_rep(72, 3800);
    reputation.watch(72);
    crate::persistence::save_reputation(&db, character, &reputation);
    drop(db);
    let reopened = redb::Database::open(&path).unwrap();
    let persisted = crate::persistence::load_reputation(&reopened, character, || {
        panic!("private watch must persist")
    });
    assert_eq!(
        (persisted.watched_faction(), persisted.get_value(72)),
        (Some(72), 3800)
    );
}

#[test]
fn initial_reputation_update_restores_persisted_watch_without_a_journeys_query() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let h = crate::bank_test_harness::harness_at(file.path(), "Barlogin").unwrap();
    let character = h.character_id;
    let registry = default_faction_registry();
    let mut reputation = CharacterReputation::new_for_race(&registry, 1);
    reputation.set_rep(72, 3800);
    reputation.watch(72);
    crate::persistence::save_reputation(h.db(), character, &reputation);
    drop(h);
    let mut reopened = crate::bank_test_harness::harness_at(file.path(), "Barprobe").unwrap();
    reopened.play_as(character);
    let persisted = crate::persistence::load_reputation(reopened.db(), character, || {
        panic!("persisted watch required")
    });
    reopened
        .app
        .world_mut()
        .entity_mut(reopened.player)
        .insert(persisted);
    reopened.app.insert_resource(ReputationRegistry(registry));
    reopened
        .app
        .add_systems(Update, send_initial_reputation_snapshot);
    reopened
        .app
        .world_mut()
        .entity_mut(reopened.connection)
        .remove::<InWorld>();
    reopened
        .app
        .world_mut()
        .entity_mut(reopened.connection)
        .insert((
            InWorld {
                character_id: character,
            },
            MessageSender::<ReputationStateUpdate>::default(),
            MessageReceiver::<ReputationStateUpdate>::default(),
        ));
    let mut updates = Vec::new();
    for _ in 0..10 {
        reopened.app.update();
        updates.extend(
            reopened
                .app
                .world_mut()
                .get_mut::<MessageReceiver<ReputationStateUpdate>>(reopened.connection)
                .unwrap()
                .receive(),
        );
    }
    let watched = updates
        .into_iter()
        .find_map(|update| update.snapshot.and_then(|snapshot| snapshot.watched))
        .expect("login must publish watched bar without RenownRequest");
    assert_eq!(
        (watched.faction_id, watched.value, watched.maximum),
        (72, 800, 6000)
    );
}

#[test]
fn watched_bar_reads_real_renown_progress_and_standard_tier_values_without_catalog_delivery() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let h = crate::bank_test_harness::harness_at(file.path(), "Barseeker").unwrap();
    let registry = default_faction_registry();
    let mut reputation = CharacterReputation::default();
    assert_eq!(
        read_watched_reputation(h.db(), h.character_id, &reputation, &registry),
        None
    );
    reputation.set_rep(72, 3800);
    reputation.watch(72);
    let watched = read_watched_reputation(h.db(), h.character_id, &reputation, &registry).unwrap();
    assert_eq!(
        (
            watched.name.as_str(),
            watched.value,
            watched.maximum,
            watched.capped,
            watched.is_major_faction,
            watched.reaction
        ),
        ("Stormwind", 800, 6000, false, false, Standing::Friendly)
    );
    reputation.set_rep(72, 42_000);
    let capped = read_watched_reputation(h.db(), h.character_id, &reputation, &registry).unwrap();
    assert_eq!((capped.value, capped.maximum, capped.capped), (1, 1, true));
    crate::renown::admin_grant(h.db(), "Barseeker", 12, 6000).unwrap();
    // Persisted-watch fixture does not assert that the realm evaluated Maruuk eligibility.
    reputation.watch(2503);
    let renown = read_watched_reputation(h.db(), h.character_id, &reputation, &registry).unwrap();
    assert_eq!(
        (
            renown.faction_id,
            renown.name.as_str(),
            renown.value,
            renown.maximum,
            renown.capped,
            renown.is_major_faction
        ),
        (2503, "Maruuk Centaur", 1000, 2500, false, true)
    );
    crate::renown::admin_grant(h.db(), "Barseeker", 12, 1_000_000).unwrap();
    let capped = read_watched_reputation(h.db(), h.character_id, &reputation, &registry).unwrap();
    assert_eq!(
        (capped.value, capped.maximum, capped.capped),
        (0, 2500, true)
    );
    reputation.unwatch();
    assert_eq!(
        read_watched_reputation(h.db(), h.character_id, &reputation, &registry),
        None
    );
}

#[test]
fn snapshot_lists_known_factions_with_standing_text() {
    let registry = default_faction_registry();
    let mut reputation = CharacterReputation::new_for_race(&registry, 1);
    reputation.set_rep(72, 30_000);

    let h = crate::bank_test_harness::harness().unwrap();
    let snapshot = read_reputation_snapshot(h.db(), h.character_id, &reputation, &registry);

    assert_eq!(snapshot.entries.len(), registry.iter().count());
    let stormwind = snapshot
        .entries
        .iter()
        .find(|entry| entry.faction_id == 72)
        .expect("stormwind entry");
    assert_eq!(stormwind.standing, format!("{:?}", Standing::Revered));
    assert_eq!(stormwind.value, 30_000);
}
