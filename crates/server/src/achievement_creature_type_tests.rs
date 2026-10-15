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
        InMap::continent(0),
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
