//! Actual spell-target28 with controlled same-map players; not garrison invasion content.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness_with_plugins};
use rusqlite::Connection;
use shared::components::Health;

const INVASION_SPELL: u32 = 178544;
const CRITERION: u32 = 27304;

fn spawn_counted_peer(app: &mut App, character: u64, map: InMap) -> Entity {
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: format!("Countpeer{character}"),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Level(10),
            persistence::ClientId(character),
            map,
            Health {
                current: 100.0,
                max: 100.0,
            },
            PlayerAchievementState::default(),
        ))
        .id();
    app.world_mut().spawn((
        Authenticated {
            account_id: character,
        },
        PlayerLink(player),
        InWorld {
            character_id: character,
        },
    ));
    player
}

fn assert_native_spell_hits(h: &Harness, expected: usize) {
    let casts: Vec<_> = h
        .inbox
        .go
        .iter()
        .filter(|cast| cast.spell_id == INVASION_SPELL)
        .collect();
    assert_eq!(casts.len(), expected, "authored native casts must succeed");
    for cast in casts {
        assert!(
            cast.hit_targets.contains(&h.caster.to_bits()),
            "cast must actually hit reference player: {cast:?}"
        );
    }
}

#[test]
fn achievements_criteria_map_count171_native_spell_hit_uses_exact_current_copy_players() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "Countguard", 1, 9).unwrap();
    let (second, _) = persistence::create_character(&db, 2, "Counttwo", 1, 1).unwrap();
    let (third, _) = persistence::create_character(&db, 3, "Countthree", 1, 1).unwrap();
    let (fourth, _) = persistence::create_character(&db, 4, "Countfour", 1, 1).unwrap();
    let mut h = harness_with_plugins(vec![], &[INVASION_SPELL], 9, false, |app| {
        app.add_plugins(AchievementPlugin);
    })
    .unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    h.app.insert_resource(GameDatabase { db });
    let map = *h.app.world().get::<InMap>(h.caster).unwrap();
    h.app.world_mut().entity_mut(h.caster).insert((
        persistence::ClientId(character),
        PlayerAchievementState::default(),
    ));
    h.app
        .world_mut()
        .get_mut::<InWorld>(h.connection)
        .unwrap()
        .character_id = character;
    spawn_counted_peer(&mut h.app, second, map);
    let third_player = spawn_counted_peer(
        &mut h.app,
        third,
        InMap {
            map_id: map.map_id,
            instance_id: map.instance_id + 1,
        },
    );
    let fourth_player = spawn_counted_peer(&mut h.app, fourth, InMap::continent(1));
    h.advance(1);
    assert_eq!(
        event_conditions(h.app.world(), h.caster, &[CriteriaEvent::ReachLevel(10)])
            .player
            .unwrap()
            .map_player_count,
        None,
        "unrelated event does not request map-roster authority"
    );
    assert_eq!(
        event_conditions(
            h.app.world(),
            h.caster,
            &[CriteriaEvent::BeSpellTarget(INVASION_SPELL)]
        )
        .player
        .unwrap()
        .map_player_count,
        Some(2)
    );
    h.cast(INVASION_SPELL, Some(h.caster));
    h.advance(30); // Authored spell178544 RecoveryTime is1000ms.
    assert_native_spell_hits(&h, 1);
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&CRITERION),
        "different copy/map peers cannot supply the third player"
    );
    h.app.world_mut().entity_mut(third_player).insert(map);
    // A dead but present player remains in the map roster; no living-only/group substitution.
    h.app
        .world_mut()
        .get_mut::<Health>(third_player)
        .unwrap()
        .current = 0.0;
    h.advance(1);
    assert!(!crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        INVASION_SPELL,
        Some(h.caster),
        255
    ));
    h.cast(INVASION_SPELL, Some(h.caster));
    h.advance(30);
    assert_native_spell_hits(&h, 2);
    let state = h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap();
    assert_eq!(
        state.criteria_progress.get(&CRITERION),
        Some(&1),
        "exact three current-map players qualify actual target receipt"
    );
    assert!(state.earned_ids.contains(&9244));
    let date = state.earned_at[&9244];
    h.app.world_mut().entity_mut(fourth_player).insert(map);
    h.cast(INVASION_SPELL, Some(h.caster));
    h.advance(30);
    assert_native_spell_hits(&h, 3);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&CRITERION),
        Some(&1),
        "four is not exactly three"
    );
    h.advance(3);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .earned_at[&9244],
        date
    );
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&db, character);
    assert_eq!(saved.criteria_progress.get(&CRITERION), Some(&1));
    assert_eq!(saved.earned_at[&9244], date);
    for foreign in [second, third, fourth] {
        assert!(
            !persistence::load_achievements(&db, foreign)
                .criteria_progress
                .contains_key(&CRITERION)
        );
    }
}
