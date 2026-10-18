use super::*;
use crate::achievement::{PlayerAchievementState, load_achievement_catalog};
use crate::auth::{Authenticated, InWorld};
use crate::networking::{FallTracker, PlayerLink};
use crate::persistence::{self, ClientId, GameDatabase};
use bevy::ecs::system::RunSystemOnce;
use shared::components::{Health, Level, MovementControl, Player, Position, VerticalVelocity};

struct FallFixture {
    app: App,
    player: Entity,
    character: u64,
    file: tempfile::NamedTempFile,
    floor: f32,
}

fn create_fall_fixture() -> FallFixture {
    let data = std::env::var_os("GAME_SERVER_GROUND_DIR").expect("real ground directory");
    let mut ground = GroundMap::new(std::path::Path::new(&data), HashMap::from([(0, 775971)]));
    let (x, y, z) = crate::terrain_height::wow_to_bevy(-8950.0, -132.0, 183.0);
    let GroundProbe::Supported(floor) = ground.ground_at(0, Vec3::new(x, y, z)) else {
        panic!("missing Stormwind landing floor");
    };
    let file = tempfile::NamedTempFile::new().unwrap();
    let (mut app, character) = create_fall_app(&file, ground);
    let player = spawn_fall_actor(app.world_mut(), character, Position { x, y: floor, z });
    app.world_mut().spawn((
        Authenticated { account_id: 1 },
        InWorld {
            character_id: character,
        },
        PlayerLink(player),
    ));
    FallFixture {
        app,
        player,
        character,
        file,
        floor,
    }
}

fn create_fall_app(file: &tempfile::NamedTempFile, ground: GroundMap) -> (App, u64) {
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "Landingproof", 1, 1).unwrap();
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins)
        .add_message::<crate::death::PlayerDeathTransition>();
    app.init_resource::<crate::maps::ServerClock>();
    app.init_resource::<crate::auras::UnhandledAuras>();
    app.insert_resource(
        crate::spell_cast::test_harness::world_db()
            .unwrap()
            .store
            .clone(),
    );
    app.insert_resource(ground);
    app.insert_resource(load_achievement_catalog(&conn).unwrap());
    app.insert_resource(GameDatabase::new(db));
    (app, character)
}

fn spawn_fall_actor(world: &mut World, character: u64, position: Position) -> Entity {
    world
        .spawn((
            Player {
                name: "Landingproof".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Level(1),
            ClientId(character),
            crate::maps::InMap::continent(0),
            position,
            Health {
                current: 1000.0,
                max: 1000.0,
            },
            MovementControl::default(),
            VerticalVelocity::default(),
            FallTracker::grounded(position.y),
            PlayerAchievementState::default(),
        ))
        .id()
}

fn start_fall(fixture: &mut FallFixture, distance: f32, health: f32) {
    let height = fixture.floor + distance;
    let world = fixture.app.world_mut();
    world.get_mut::<Position>(fixture.player).unwrap().y = height;
    world.entity_mut(fixture.player).insert((
        FallTracker::grounded(height),
        VerticalVelocity::default(),
        Health {
            current: health,
            max: 1000.0,
        },
    ));
}

fn run_until_landing(fixture: &mut FallFixture) {
    for _ in 0..400 {
        fixture
            .app
            .world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        fixture
            .app
            .world_mut()
            .run_system_once(crate::networking::apply_terrain_gravity)
            .unwrap();
        let height = fixture
            .app
            .world()
            .get::<Position>(fixture.player)
            .unwrap()
            .y;
        if height <= fixture.floor + crate::terrain_height::GROUND_SNAP_THRESHOLD {
            return;
        }
    }
    panic!(
        "native fall did not land: position {:?}, tracker {:?}",
        fixture.app.world().get::<Position>(fixture.player),
        fixture.app.world().get::<FallTracker>(fixture.player)
    );
}

fn apply_landing_aura(fixture: &mut FallFixture, spell: u32) {
    crate::auras::apply_aura(
        fixture.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: fixture.player,
            target: fixture.player,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(
        fixture
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(fixture.player)
            .unwrap()
            .has_spell(spell)
    );
}

fn landing_centiyards(fixture: &FallFixture) -> u64 {
    let height = fixture
        .app
        .world()
        .get::<Position>(fixture.player)
        .unwrap()
        .y;
    ((height - fixture.floor) * 100.0) as u64
}

fn fall_progress(fixture: &FallFixture) -> Option<u64> {
    fixture
        .app
        .world()
        .get::<PlayerAchievementState>(fixture.player)
        .unwrap()
        .criteria_progress
        .get(&2072)
        .copied()
}

#[test]
fn achievements_criteria_native_survived_landing_credits_distance_and_reopens() {
    let mut fixture = create_fall_fixture();
    start_fall(&mut fixture, 60.0, 1000.0);
    let first_distance = landing_centiyards(&fixture);
    assert!((5999..=6000).contains(&first_distance));
    assert_eq!(
        fall_progress(&fixture),
        None,
        "starting height is not credit"
    );
    run_until_landing(&mut fixture);
    let health = fixture
        .app
        .world()
        .get::<Health>(fixture.player)
        .unwrap()
        .current;
    assert!(
        health > 0.0 && health < 1000.0,
        "actual damaging survived landing: {health}"
    );
    assert_eq!(fall_progress(&fixture), Some(first_distance));
    assert!(
        !fixture
            .app
            .world()
            .get::<PlayerAchievementState>(fixture.player)
            .unwrap()
            .earned_ids
            .contains(&964)
    );
    start_fall(&mut fixture, 30.0, 1000.0);
    let second_distance = landing_centiyards(&fixture);
    assert!((2999..=3000).contains(&second_distance));
    run_until_landing(&mut fixture);
    assert_eq!(
        fall_progress(&fixture),
        Some(second_distance),
        "TC PROGRESS_SET, not accumulated or highest"
    );
    for _ in 0..3 {
        fixture
            .app
            .world_mut()
            .run_system_once(crate::networking::apply_terrain_gravity)
            .unwrap();
    }
    assert_eq!(
        fall_progress(&fixture),
        Some(second_distance),
        "idle grounded ticks do not emit"
    );
    let FallFixture {
        app,
        file,
        character,
        ..
    } = fixture;
    drop(app);
    let db = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&db, character);
    assert_eq!(saved.criteria_progress.get(&2072), Some(&second_distance));
    assert!(!saved.earned_ids.contains(&964));
}

#[test]
fn achievements_criteria_native_bouncy_survived_landing_grants_going_down() {
    let mut fixture = create_fall_fixture();
    fixture
        .app
        .world_mut()
        .get_mut::<Player>(fixture.player)
        .unwrap()
        .race = 24;
    // Authored Pandaren racial107076 aura374/-50, applied through the native aura loader.
    apply_landing_aura(&mut fixture, 107076);
    start_fall(&mut fixture, 66.0, 1000.0);
    let distance = landing_centiyards(&fixture);
    assert_eq!(
        fall_progress(&fixture),
        None,
        "racial application cannot fabricate falling credit"
    );
    run_until_landing(&mut fixture);
    assert_eq!(
        fixture
            .app
            .world()
            .get::<Health>(fixture.player)
            .unwrap()
            .current,
        500.0
    );
    assert_eq!(fall_progress(&fixture), Some(distance));
    assert!(
        fixture
            .app
            .world()
            .get::<PlayerAchievementState>(fixture.player)
            .unwrap()
            .earned_ids
            .contains(&964)
    );
    let FallFixture {
        app,
        file,
        character,
        ..
    } = fixture;
    drop(app);
    let db = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&db, character);
    assert_eq!(saved.criteria_progress.get(&2072), Some(&distance));
    assert!(saved.earned_ids.contains(&964));
}

#[test]
fn achievements_criteria_native_fall_safe_feather_flying_and_lethal_do_not_credit() {
    for (distance, aura, flying) in [
        (1.0, None, false),
        (5.0, None, false),
        (66.0, None, false),
        (66.0, Some(130), false),
        (66.0, Some(262359), false),
        // Native physical immunity and17yd Safe Fall still reject the source predicate.
        (30.0, Some(1022), false),
        (30.0, Some(1860), false),
        (66.0, None, true),
    ] {
        let mut fixture = create_fall_fixture();
        if let Some(spell) = aura {
            apply_landing_aura(&mut fixture, spell);
        }
        start_fall(&mut fixture, distance, 1000.0);
        if flying {
            fixture.app.world_mut().entity_mut(fixture.player).insert(
                shared::components::PlayerMotion(shared::components::PlayerMotion::FLYING),
            );
            fixture
                .app
                .world_mut()
                .run_system_once(crate::networking::apply_terrain_gravity)
                .unwrap();
        } else {
            run_until_landing(&mut fixture);
        }
        assert_eq!(
            fall_progress(&fixture),
            None,
            "distance{distance} aura{aura:?} flying{flying}"
        );
        let health = fixture
            .app
            .world()
            .get::<Health>(fixture.player)
            .unwrap()
            .current;
        if distance == 66.0 && aura.is_none() && !flying {
            assert_eq!(health, 0.0);
        }
        if matches!(aura, Some(130 | 262359)) || flying {
            assert_eq!(health, 1000.0);
        }
        // The realm damage path does not implement immunity/safe-fall parity yet;
        // this assertion proves only that the achievement cannot bypass either leaf.
    }
}

#[test]
fn achievements_criteria_native_knockback_landing_credits_only_once() {
    let mut fixture = create_fall_fixture();
    let data = std::env::var_os("GAME_SERVER_GROUND_DIR").unwrap();
    fixture.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&std::path::Path::new(&data).join("los")).unwrap(),
    );
    start_fall(&mut fixture, 30.0, 1000.0);
    let distance = landing_centiyards(&fixture);
    let position = *fixture.app.world().get::<Position>(fixture.player).unwrap();
    let origin = Vec3::new(position.x + 5.0, position.y, position.z);
    crate::knockback::launch(fixture.app.world_mut(), fixture.player, origin, 0.0, 0.0).unwrap();
    assert_eq!(
        fall_progress(&fixture),
        None,
        "launch is not achievement credit"
    );
    for _ in 0..200 {
        fixture
            .app
            .world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        crate::knockback::update_knockbacks(fixture.app.world_mut());
        if !fixture
            .app
            .world()
            .get::<MovementControl>(fixture.player)
            .unwrap()
            .controlled
        {
            break;
        }
    }
    assert!(
        !fixture
            .app
            .world()
            .get::<MovementControl>(fixture.player)
            .unwrap()
            .controlled
    );
    assert_eq!(fall_progress(&fixture), Some(distance));
    let health = fixture
        .app
        .world()
        .get::<Health>(fixture.player)
        .unwrap()
        .current;
    assert!(health > 0.0 && health < 1000.0);
    for _ in 0..3 {
        crate::knockback::update_knockbacks(fixture.app.world_mut());
        fixture
            .app
            .world_mut()
            .run_system_once(crate::networking::apply_terrain_gravity)
            .unwrap();
    }
    assert_eq!(fall_progress(&fixture), Some(distance));
    let FallFixture {
        app,
        file,
        character,
        ..
    } = fixture;
    drop(app);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&db, character)
            .criteria_progress
            .get(&2072),
        Some(&distance)
    );
}

#[test]
fn achievements_criteria_alive_fact_rejects_missing_health_and_dead_life_state() {
    let mut fixture = create_fall_fixture();
    assert_eq!(
        crate::achievement::player_conditions(fixture.app.world(), fixture.player)
            .player
            .unwrap()
            .alive,
        Some(true)
    );
    fixture
        .app
        .world_mut()
        .get_mut::<Health>(fixture.player)
        .unwrap()
        .current = 0.0;
    assert_eq!(
        crate::achievement::player_conditions(fixture.app.world(), fixture.player)
            .player
            .unwrap()
            .alive,
        Some(false)
    );
    fixture
        .app
        .world_mut()
        .get_mut::<Health>(fixture.player)
        .unwrap()
        .current = 1000.0;
    fixture
        .app
        .world_mut()
        .entity_mut(fixture.player)
        .insert(crate::death::PlayerDeathState {
            state: shared::death::DeathState::Dead,
            ..Default::default()
        });
    assert_eq!(
        crate::achievement::player_conditions(fixture.app.world(), fixture.player)
            .player
            .unwrap()
            .alive,
        Some(false)
    );
    fixture
        .app
        .world_mut()
        .entity_mut(fixture.player)
        .remove::<Health>();
    assert_eq!(
        crate::achievement::player_conditions(fixture.app.world(), fixture.player)
            .player
            .unwrap()
            .alive,
        None
    );
}
