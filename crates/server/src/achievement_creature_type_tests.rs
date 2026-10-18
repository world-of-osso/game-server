use super::*;
use crate::spell_cast::test_harness::{Harness, harness_with_plugins};
use rusqlite::Connection;
use shared::components::{Health, Position};

fn spawn_source_kill_target(h: &mut Harness, entry: u32) -> Entity {
    let world = h.app.world_mut();
    let data = world.resource::<crate::world_data::WorldData>();
    let spawn = data
        .spawns
        .iter()
        .find(|spawn| spawn.template_entry == entry && spawn.map == 0)
        .expect("source continent creature spawn");
    let template = data.templates.get(&entry).unwrap();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);
    let target = crate::zones::spawn_creature(
        &mut commands,
        data,
        template,
        &mut crate::zones::CreatureSpawnContext {
            map: InMap::continent(0),
            difficulty_id: crate::zones::map_difficulty_id(world, InMap::continent(0)),
            ground: Some(&mut crate::test_data::ground_map()),
            temporary: false,
        },
        false,
        spawn,
        &mut rand::thread_rng(),
    );
    queue.apply(world);
    let position = *world.get::<Position>(target).unwrap();
    world.entity_mut(h.caster).insert(Position {
        x: position.x + 3.0,
        ..position
    });
    target
}

fn kill_source_target_with_shadow_bolts(h: &mut Harness, target: Entity) {
    assert!(h.app.world().get::<Health>(target).unwrap().current > 0.0);
    for _ in 0..5 {
        h.cast(686, Some(target));
        h.advance(160);
        if h.app.world().get::<Health>(target).unwrap().current == 0.0 {
            break;
        }
    }
    assert_eq!(h.app.world().get::<Health>(target).unwrap().current, 0.0);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(target)
            .is_some()
    );
    assert!(h.inbox.go.iter().any(|cast| cast.spell_id == 686));
}

fn assert_creature_statistics(h: &Harness, total: u64, maximum: u64, distinct: u64) {
    let catalog = h.app.world().resource::<AchievementCatalog>();
    let state = h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap();
    for (achievement, value) in [(107, total), (1336, maximum), (1337, distinct)] {
        let AchievementCatalogPage::Statistics { achievements, .. } = catalog.page(
            state,
            QueryAchievementCatalog::Statistics {
                category_id: 135,
                after_id: achievement - 1,
            },
            1,
        ) else {
            panic!("expected creature statistics page");
        };
        let row = achievements.first().unwrap();
        assert_eq!(row.achievement_id, achievement);
        assert_eq!(row.statistic_value, Some(value));
        assert!(!row.earned, "statistics cannot become earned achievements");
    }
}

#[test]
fn achievements_criteria_native_creature_type_preserves_lethal_victim_and_reopened_counters() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let database = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&database);
    let (character, _) = persistence::create_character(&database, 1, "Types", 1, 9).unwrap();
    assert_eq!(character, 1);
    let (other, _) = persistence::create_character(&database, 2, "Other", 1, 9).unwrap();
    let mut h = harness_with_plugins(vec![], &[686], 9, false, |app| {
        app.add_plugins(AchievementPlugin);
    })
    .unwrap();
    h.app.world_mut().despawn(h.npc);
    h.app.insert_resource(crate::world_data::load_world_data(
        &crate::test_data::world_db_path(),
    ));
    h.app.insert_resource(GameDatabase::new(database));
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    h.app.world_mut().entity_mut(h.caster).insert((
        crate::persistence::ClientId(character),
        PlayerAchievementState::default(),
    ));
    assert_creature_statistics(&h, 0, 0, 0);
    for (entry, kind, criterion, total, typed) in [
        (40, 7, 4953, 1, 1),
        (299, 1, 4948, 2, 1),
        (94, 7, 4953, 3, 2),
    ] {
        let target = spawn_source_kill_target(&mut h, entry);
        assert_eq!(crate::unit_creature_type::get(h.app.world(), target), kind);
        assert!(!crate::spell_cast::try_cast_triggered(
            h.app.world_mut(),
            h.caster,
            686,
            Some(target),
            u8::MAX,
        ));
        h.advance(3);
        assert_eq!(
            h.app
                .world()
                .get::<PlayerAchievementState>(h.caster)
                .unwrap()
                .criteria_progress
                .get(&4944)
                .copied(),
            (total > 1).then_some(total - 1),
            "refused cast and living target cannot credit kills"
        );
        kill_source_target_with_shadow_bolts(&mut h, target);
        let state = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap();
        assert_eq!(state.criteria_progress.get(&4944), Some(&total));
        assert_eq!(
            state.criteria_progress.get(&criterion),
            Some(&typed),
            "actual victim type must qualify its statistic"
        );
        assert!(
            !state.criteria_progress.contains_key(&4950),
            "neither target is a dragonkin"
        );
        let counters = state.criteria_progress.clone();
        h.cast(686, Some(target));
        h.advance(160);
        assert_eq!(
            h.app
                .world()
                .get::<PlayerAchievementState>(h.caster)
                .unwrap()
                .criteria_progress,
            counters,
            "dead/idle updates cannot replay a kill"
        );
        assert_creature_statistics(&h, total, typed, total.min(2));
    }
    drop(h);
    let reopened = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&reopened, character);
    assert_eq!(saved.criteria_progress.get(&4948), Some(&1));
    assert_eq!(saved.criteria_progress.get(&4953), Some(&2));
    assert_eq!(saved.criteria_progress.get(&4944), Some(&3));
    assert!(
        persistence::load_achievements(&reopened, other)
            .criteria_progress
            .is_empty()
    );
}

// Controlled placement only, not an authored world spawn/acquisition claim.
// Immutable template, level, faction, model and health come from native spawn_creature.
fn spawn_controlled_native_target(h: &mut Harness, entry: u32) -> Entity {
    let world = h.app.world_mut();
    let data = world.resource::<crate::world_data::WorldData>();
    let template = data.templates.get(&entry).unwrap();
    let spawn = crate::world_data::CreatureSpawn {
        guid: 0,
        template_entry: entry,
        map: 0,
        zone_id: 0,
        position_x: 3.0,
        position_y: 0.0,
        position_z: 0.0,
        orientation: 0.0,
        wander_distance: 0.0,
        movement_type: 0,
        spawntimesecs: 300,
        display_id: None,
        unit_flags: None,
        spawn_difficulties: vec![0],
        equipment_id: 0,
    };
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);
    let target = crate::zones::spawn_creature(
        &mut commands,
        data,
        template,
        &mut crate::zones::CreatureSpawnContext {
            map: InMap::continent(0),
            difficulty_id: crate::zones::map_difficulty_id(world, InMap::continent(0)),
            ground: Some(&mut crate::test_data::ground_map()),
            temporary: false,
        },
        false,
        &spawn,
        &mut rand::thread_rng(),
    );
    queue.apply(world);
    target
}

fn add_level20_group_recipient(h: &mut Harness, character: u64, member: u64) -> Entity {
    use crate::group::{GroupRegistry, OnlineMember, OnlineView};
    let recipient = crate::spell_cast::test_harness::spawn_player(&mut h.app, 9, 2.0);
    h.app.world_mut().entity_mut(recipient).insert((
        crate::persistence::ClientId(member),
        shared::components::Level(20),
        PlayerAchievementState::default(),
    ));
    let connection =
        crate::spell_cast::test_harness::spawn_peer_connection(&mut h.app, recipient, member);
    h.app.world_mut().entity_mut(connection).insert((
        Authenticated { account_id: 3 },
        InWorld {
            character_id: member,
        },
    ));
    let online: OnlineView = [(character, "Demonlevels"), (member, "Recipient")]
        .into_iter()
        .map(|(id, name)| {
            (
                id,
                OnlineMember {
                    entity: id,
                    name: name.into(),
                    class: 9,
                    level: 20,
                    portrait: Default::default(),
                    dungeon_difficulty: 1,
                    raid_difficulty: Default::default(),
                },
            )
        })
        .collect();
    let mut groups = GroupRegistry::default();
    groups.invite(character, "Recipient", &online);
    groups.respond_invite(member, true);
    assert_eq!(groups.group_members(character).len(), 2);
    h.app.insert_resource(groups);
    recipient
}

#[test]
fn achievements_criteria_native_demon_level21_uses_killed_level_and_reopens() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let database = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&database);
    let (character, _) = persistence::create_character(&database, 1, "Demonlevels", 1, 9).unwrap();
    let (foreign, _) = persistence::create_character(&database, 2, "Foreign", 1, 9).unwrap();
    let (member, _) = persistence::create_character(&database, 3, "Recipient", 1, 9).unwrap();
    let mut h = harness_with_plugins(vec![], &[686], 9, false, |app| {
        app.add_plugins(AchievementPlugin);
    })
    .unwrap();
    h.app.world_mut().despawn(h.npc);
    h.app.insert_resource(crate::world_data::load_world_data(
        &crate::test_data::world_db_path(),
    ));
    h.app.insert_resource(GameDatabase::new(database));
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    h.app.world_mut().entity_mut(h.caster).insert((
        crate::persistence::ClientId(character),
        PlayerAchievementState::default(),
    ));
    let recipient = add_level20_group_recipient(&mut h, character, member);
    // The production content view gives Lesser Voidwalker418 an untuned level20.
    // Legacy Imp416 says5, but native ContentTuning482 makes its actual level90.
    for (level, kills, previous, expected) in [(19, 1, 0, 0), (20, 2, 0, 1), (21, 3, 1, 2)] {
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(shared::components::Level(level));
        let target = spawn_controlled_native_target(&mut h, 418);
        let scaling = h
            .app
            .world()
            .get::<shared::level_scaling::LevelScaling>(target);
        let area = crate::test_data::ground_map().area_at(0, bevy::math::Vec3::new(0.0, 0.0, -3.0));
        eprintln!(
            "GATE486 LesserVoidwalker scaling={:?} actual_area={area:?}",
            scaling.map(|s| (s.content_tuning_id, s.min_level, s.max_level, s.delta))
        );
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::UnitLevel>(target)
                .unwrap()
                .0,
            20
        );
        assert_eq!(crate::unit_creature_type::get(h.app.world(), target), 3);
        assert!(!crate::spell_cast::try_cast_triggered(
            h.app.world_mut(),
            h.caster,
            686,
            Some(target),
            u8::MAX
        ));
        h.advance(3);
        let before = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .clone();
        kill_source_target_with_shadow_bolts(&mut h, target);
        let state = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap();
        assert_eq!(
            state.criteria_progress.get(&4944),
            Some(&kills),
            "native generic kill must be accepted before checking21"
        );
        assert_eq!(
            state.criteria_progress.get(&4949),
            Some(&kills),
            "native Demon30/3 companion remains independent"
        );
        assert_eq!(
            state.criteria_progress.get(&107647).copied().unwrap_or(0),
            expected,
            "21/0 must compare actor against actual killed creature, inclusively"
        );
        assert_eq!(before.get(&107647).copied().unwrap_or(0), previous);
        let counters = state.criteria_progress.clone();
        let recipient_state = h
            .app
            .world()
            .get::<PlayerAchievementState>(recipient)
            .unwrap();
        assert_eq!(
            recipient_state.criteria_progress.get(&107647),
            Some(&kills),
            "eligible recipient compares their own20, not killer level"
        );
        assert_eq!(recipient_state.criteria_progress.get(&4944), Some(&kills));
        h.cast(686, Some(target));
        h.advance(160);
        assert_eq!(
            h.app
                .world()
                .get::<PlayerAchievementState>(h.caster)
                .unwrap()
                .criteria_progress,
            counters,
            "dead and idle frames cannot replay the victim"
        );
    }
    h.app
        .world_mut()
        .entity_mut(recipient)
        .insert(InMap::continent(1));
    let imp = spawn_controlled_native_target(&mut h, 416);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitLevel>(imp)
            .unwrap()
            .0,
        90
    );
    kill_source_target_with_shadow_bolts(&mut h, imp);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&107647),
        Some(&2),
        "legacy template level5 must not replace the killed Imp's actual90"
    );
    let target = spawn_controlled_native_target(&mut h, 40);
    assert_eq!(crate::unit_creature_type::get(h.app.world(), target), 7);
    kill_source_target_with_shadow_bolts(&mut h, target);
    let state = h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap();
    assert_eq!(state.criteria_progress.get(&4944), Some(&5));
    assert_eq!(
        state.criteria_progress.get(&107647),
        Some(&2),
        "non-Demon must not satisfy the conjunction"
    );
    let catalog = h.app.world().resource::<AchievementCatalog>();
    let AchievementCatalogPage::Statistics { achievements, .. } = catalog.page(
        state,
        QueryAchievementCatalog::Statistics {
            category_id: 15562,
            after_id: 60937,
        },
        1,
    ) else {
        panic!("expected statistics");
    };
    assert_eq!(achievements[0].achievement_id, 60938);
    assert_eq!(achievements[0].statistic_value, Some(2));
    assert!(!achievements[0].earned);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(recipient)
            .unwrap()
            .criteria_progress
            .get(&4944),
        Some(&3),
        "other map copy cannot receive further kill credit"
    );
    drop(h);
    let reopened = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&reopened, character);
    assert_eq!(saved.criteria_progress.get(&107647), Some(&2));
    assert_eq!(saved.criteria_progress.get(&4944), Some(&5));
    let recipient_saved = persistence::load_achievements(&reopened, member);
    assert_eq!(recipient_saved.criteria_progress.get(&107647), Some(&3));
    assert_eq!(recipient_saved.criteria_progress.get(&4944), Some(&3));
    assert!(
        persistence::load_achievements(&reopened, foreign)
            .criteria_progress
            .is_empty()
    );
}
