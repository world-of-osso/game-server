use super::test_harness::*;
use crate::achievement::{AchievementPlugin, PlayerAchievementState};
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{self, ClientId, GameDatabase};
use bevy::prelude::*;
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
fn achievements_criteria_native_penance_healing_counts_effective_and_raw_peaks_and_persists() {
    let mut h = harness(Vec::new(), &[47540, 47758, 47757, 47666, 47750], 5)
        .expect("real Penance pipeline");
    let ally = spawn_player(&mut h.app, 1, 2.0);
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (caster_id, _) = persistence::create_character(&db, 1, "Healproof", 1, 5).unwrap();
    let (ally_id, _) = persistence::create_character(&db, 2, "Healally", 1, 1).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((ClientId(caster_id), PlayerAchievementState::default()));
    h.app
        .world_mut()
        .entity_mut(ally)
        .insert((ClientId(ally_id), PlayerAchievementState::default()));
    h.app.world_mut().entity_mut(h.connection).insert(InWorld {
        character_id: caster_id,
    });
    h.app.world_mut().spawn((
        Authenticated { account_id: 2 },
        InWorld {
            character_id: ally_id,
        },
        PlayerLink(ally),
    ));
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
    h.app.insert_resource(GameDatabase::new(db));
    bevy::app::Plugin::build(&AchievementPlugin, &mut h.app);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 0.0,
        max: 1000.0,
    };
    h.cast_and_deliver(47540, Some(ally));
    assert!(!h.inbox.failed.is_empty(), "native dead-target refusal");
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&4943)
    );
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(ally)
            .unwrap()
            .criteria_progress
            .contains_key(&5375)
    );
    h.inbox.failed.clear();
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 950.0,
        max: 1000.0,
    };
    h.cast_and_deliver(47540, Some(ally));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(50);
    assert_eq!(h.health(ally), 1000.0);
    let heals: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(47750))
        .collect();
    assert_eq!(heals.len(), 3);
    assert_eq!(
        heals
            .iter()
            .map(|event| event.amount - event.overflow)
            .sum::<i32>(),
        50
    );
    let peak = heals.iter().map(|event| event.amount as u64).max().unwrap();
    assert!(peak > 50, "raw cast includes overheal");
    let caster = h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap();
    assert_eq!(caster.criteria_progress.get(&4943), Some(&50));
    assert_eq!(caster.criteria_progress.get(&5376), Some(&peak));
    assert!(!caster.criteria_progress.contains_key(&5375));
    let recipient = h.app.world().get::<PlayerAchievementState>(ally).unwrap();
    assert_eq!(recipient.criteria_progress.get(&5375), Some(&50));
    assert_eq!(recipient.criteria_progress.get(&5374), Some(&peak));
    assert!(!recipient.criteria_progress.contains_key(&4943));
    h.advance(30);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&4943),
        Some(&50)
    );
    drop(h);
    let reopened = redb::Database::open(file.path()).unwrap();
    let caster = persistence::load_achievements(&reopened, caster_id);
    let recipient = persistence::load_achievements(&reopened, ally_id);
    assert_eq!(caster.criteria_progress.get(&4943), Some(&50));
    assert_eq!(caster.criteria_progress.get(&5376), Some(&peak));
    assert_eq!(recipient.criteria_progress.get(&5375), Some(&50));
    assert_eq!(recipient.criteria_progress.get(&5374), Some(&peak));
}
