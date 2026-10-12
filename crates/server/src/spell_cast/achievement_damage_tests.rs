use super::test_harness::*;
use crate::achievement::{AchievementPlugin, PlayerAchievementState};
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{self, ClientId, GameDatabase};
use bevy::prelude::*;
use shared::components::Health;

#[test]
fn achievements_criteria_native_damage_caps_totals_not_peaks_and_excludes_self_and_dead_replay() {
    let mut h = harness(Vec::new(), &[133], 8).expect("native damage world");
    let victim = spawn_player(&mut h.app, 1, 2.0);
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (caster_id, _) = persistence::create_character(&db, 1, "Hitproof", 1, 8).unwrap();
    let (victim_id, _) = persistence::create_character(&db, 2, "Hitvictim", 1, 1).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((ClientId(caster_id), PlayerAchievementState::default()));
    h.app
        .world_mut()
        .entity_mut(victim)
        .insert((ClientId(victim_id), PlayerAchievementState::default()));
    h.app.world_mut().entity_mut(h.connection).insert(InWorld {
        character_id: caster_id,
    });
    h.app.world_mut().spawn((
        Authenticated { account_id: 2 },
        InWorld {
            character_id: victim_id,
        },
        PlayerLink(victim),
    ));
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
    h.app.insert_resource(GameDatabase { db });
    bevy::app::Plugin::build(&AchievementPlugin, &mut h.app);
    h.app.add_message::<crate::death::PlayerDeathTransition>();
    *h.app.world_mut().get_mut::<Health>(victim).unwrap() = Health {
        current: 100.0,
        max: 100.0,
    };
    crate::combat::deal_direct_damage(h.app.world_mut(), h.caster, victim, 150.0);
    assert_eq!(h.health(victim), 0.0);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&162),
        Some(&100)
    );
    let state = h.app.world().get::<PlayerAchievementState>(victim).unwrap();
    assert_eq!(state.criteria_progress.get(&5371), Some(&150));
    assert_eq!(state.criteria_progress.get(&5372), Some(&100));
    crate::combat::deal_direct_damage(h.app.world_mut(), h.caster, victim, 500.0);
    crate::combat::deal_direct_damage(h.app.world_mut(), h.caster, h.caster, 20.0);
    let state = h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap();
    assert_eq!(state.criteria_progress.get(&162), Some(&100));
    assert_eq!(state.criteria_progress.get(&5371), Some(&20));
    assert!(!state.criteria_progress.contains_key(&5372));
    h.advance(5);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(victim)
            .unwrap()
            .criteria_progress
            .get(&5372),
        Some(&100)
    );
    drop(h);
    let reopened = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&reopened, caster_id)
            .criteria_progress
            .get(&162),
        Some(&100)
    );
    let saved = persistence::load_achievements(&reopened, victim_id);
    assert_eq!(saved.criteria_progress.get(&5371), Some(&150));
    assert_eq!(saved.criteria_progress.get(&5372), Some(&100));
}
