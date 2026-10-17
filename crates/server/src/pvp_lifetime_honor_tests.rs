//! TC Player6752/CriteriaHandler748: real persisted lifetime HK, never35 or Honor XP.
use super::honor_race::{counter, hit, race_world_in_world, spawn_race_victim};
use super::*;
use crate::achievement::{AchievementPlugin, PlayerAchievementState};
use crate::auth::InWorld;
use crate::networking::PlayerLink;

#[test]
fn achievements_criteria_lifetime_honor113_full_native_hundred_and_relog() {
    let (mut app, players, file) = lifetime_app(|index| index);
    let killer = players[0].1;
    let peer = players[2].1;
    let friendly = spawn_race_victim(app.world_mut(), 2, 2);
    hit(app.world_mut(), killer, friendly, 1000.0);
    let gray = spawn_race_victim(app.world_mut(), 2, 1);
    for player in [killer, peer] {
        app.world_mut().entity_mut(player).insert(Level(30));
    }
    app.world_mut().entity_mut(gray).insert(Level(20));
    hit(app.world_mut(), killer, gray, 1000.0);
    for player in [killer, peer] {
        app.world_mut().entity_mut(player).insert(Level(1));
    }
    let mut earned = Vec::new();
    for kills in 1..=100 {
        let victim = spawn_race_victim(app.world_mut(), 2, 1);
        hit(app.world_mut(), killer, victim, 1.0);
        assert_eq!(
            counter(app.world(), killer, 63413),
            (kills > 1).then_some(kills - 1),
            "nonlethal"
        );
        hit(app.world_mut(), killer, victim, 1000.0);
        hit(app.world_mut(), killer, victim, 1000.0);
        assert_eq!(
            counter(app.world(), killer, 63413),
            Some(kills),
            "native eligible kills, no dead replay"
        );
        let state = app.world().get::<PlayerAchievementState>(killer).unwrap();
        earned.push((
            state.earned_ids.contains(&238),
            state.account.as_ref().unwrap().earned_ids.contains(&513),
            state.earned_ids.contains(&39935),
        ));
    }
    for player in [killer, peer] {
        assert_eq!(
            lifetime_total(app.world(), player),
            serde_json::json!(100),
            "full100 roots238/513/39935 require real lifetime authority before113"
        );
        assert_eq!(counter(app.world(), player, 13253), Some(100));
        assert_eq!(
            app.world()
                .get::<PlayerPvpState>(player)
                .unwrap()
                .progress
                .lifetime_honor_xp,
            0
        );
    }
    for (index, &(first, account100, character100)) in earned.iter().enumerate() {
        assert!(first, "238 starts on first real kill");
        assert_eq!(account100, index == 99, "513 keeps authored100");
        assert_eq!(character100, index == 99, "39935 keeps authored100");
    }
    assert_eq!(
        counter(app.world(), players[4].1, 13253),
        None,
        "unrelated character"
    );
    let before = app.world().get::<PlayerAchievementState>(killer).unwrap();
    let character_dates = before.earned_at.clone();
    let account_date = before.account.as_ref().unwrap().earned_at[&513];
    // Relog at a distinct clock time so rewriting an earned date cannot pass by accident.
    app.world_mut()
        .resource_mut::<crate::maps::ServerClock>()
        .offset_secs += 3600;
    assert!(app.world().resource::<crate::maps::ServerClock>().unix() > account_date);
    reopen_and_relog(&mut app, players[0], file.path());
    assert_eq!(lifetime_total(app.world(), killer), serde_json::json!(100));
    assert_eq!(
        counter(app.world(), killer, 13253),
        Some(100),
        "login restores projection from persisted PvP lifetime state"
    );
    let after = app.world().get::<PlayerAchievementState>(killer).unwrap();
    assert_eq!(after.earned_at[&238], character_dates[&238]);
    assert_eq!(after.earned_at[&39935], character_dates[&39935]);
    assert_eq!(
        after.account.as_ref().unwrap().earned_at[&513],
        account_date
    );
    app.update();
    assert_eq!(
        counter(app.world(), killer, 13253),
        Some(100),
        "idle is not an increment"
    );
}

#[test]
fn achievements_criteria_lifetime_honor113_account_sums_two_actual_character_totals() {
    let (mut app, players, _file) = lifetime_app(|index| if index == 3 { 1 } else { index });
    app.world_mut()
        .remove_resource::<crate::group::GroupRegistry>();
    for (owner, kills) in [(players[0].1, 40), (players[2].1, 60)] {
        for _ in 0..kills {
            let victim = spawn_race_victim(app.world_mut(), 2, 1);
            hit(app.world_mut(), owner, victim, 1000.0);
        }
    }
    let last = players[2].1;
    assert_eq!(counter(app.world(), players[0].1, 63413), Some(40));
    assert_eq!(counter(app.world(), last, 63413), Some(60));
    let state = app.world().get::<PlayerAchievementState>(last).unwrap();
    assert_eq!(
        state
            .account
            .as_ref()
            .unwrap()
            .criteria_progress
            .get(&13253),
        Some(&100),
        "sum actual40+60, not maximum60"
    );
    assert!(state.account.as_ref().unwrap().earned_ids.contains(&513));
    for player in [players[0].1, last] {
        assert!(
            !app.world()
                .get::<PlayerAchievementState>(player)
                .unwrap()
                .earned_ids
                .contains(&39935),
            "no character individually reached100"
        );
    }
}

#[test]
fn achievements_criteria_lifetime_honor113_existing_json_starts_at_zero_policy() {
    let mut legacy = serde_json::to_value(PlayerPvpProgress::default()).unwrap();
    legacy
        .as_object_mut()
        .unwrap()
        .remove("lifetime_honorable_kills");
    let restored: PlayerPvpProgress = serde_json::from_value(legacy).unwrap();
    assert_eq!(
        serde_json::to_value(restored).unwrap()["lifetime_honorable_kills"],
        0,
        "LIFETIME_HK_TRACKING_STARTS_AT_ZERO: no backfill or XP alias"
    );
}

fn lifetime_app(
    account_for_index: impl Fn(u64) -> u64,
) -> (App, Vec<(u64, Entity)>, tempfile::NamedTempFile) {
    let mut app = App::new();
    app.add_plugins(AchievementPlugin);
    let initial = std::mem::take(app.world_mut());
    let (world, players, file) = race_world_in_world(1, initial, account_for_index);
    *app.world_mut() = world;
    app.update();
    (app, players, file)
}

fn lifetime_total(world: &World, player: Entity) -> serde_json::Value {
    serde_json::to_value(&world.get::<PlayerPvpState>(player).unwrap().progress).unwrap()["lifetime_honorable_kills"].clone()
}

fn reopen_and_relog(app: &mut App, (character, player): (u64, Entity), path: &std::path::Path) {
    let database = app.world_mut().remove_resource::<GameDatabase>().unwrap();
    let mut achievements = crate::persistence::load_achievements(&database.db, character);
    // Discard derived113 only: login must reconstruct it from persisted lifetime authority.
    achievements.criteria_progress.remove(&13253);
    drop(database);
    let db = redb::Database::open(path).unwrap();
    let progress = crate::persistence::load_pvp(&db, character);
    app.world_mut()
        .entity_mut(player)
        .insert((PlayerPvpState::from_progress(progress), achievements));
    app.world_mut().insert_resource(GameDatabase::new(db));
    let connection = app
        .world_mut()
        .query::<(Entity, &PlayerLink)>()
        .iter(app.world())
        .find(|(_, link)| link.0 == player)
        .unwrap()
        .0;
    app.world_mut().entity_mut(connection).remove::<InWorld>();
    app.update();
    app.world_mut().entity_mut(connection).insert(InWorld {
        character_id: character,
    });
    app.update();
}
