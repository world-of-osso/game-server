//! TC RewardHonor6754: actual victim class, for every eligible HK recipient.
use super::honor_race::{counter, hit, race_world, spawn_race_victim};
use super::*;
use crate::achievement::PlayerAchievementState;
use shared::components::{Health, Player};

// Imported70391 That Takes Class245: all thirteen authored one-kill leaves.
const CLASSES: &[(u8, u32)] = &[
    (6, 2359),
    (3, 2361),
    (8, 2362),
    (2, 2363),
    (5, 2364),
    (7, 2366),
    (9, 2367),
    (1, 2368),
    (4, 2365),
    (11, 2360),
    (10, 34908),
    (12, 34909),
    (13, 57810),
];

#[test]
fn achievements_criteria_honor_class52_full_thirteen_classes_and_reopen() {
    let (mut world, players, file) = race_world(1);
    let killer = players[0].1;
    let peer = players[2].1;
    let friendly = spawn_class_victim(&mut world, 6, 2);
    hit(&mut world, killer, friendly, 1000.0);
    let gray = spawn_class_victim(&mut world, 6, 1);
    for player in [killer, peer] {
        world.entity_mut(player).insert(Level(30));
    }
    world.entity_mut(gray).insert(Level(20));
    hit(&mut world, killer, gray, 1000.0);
    for player in [killer, peer] {
        world.entity_mut(player).insert(Level(1));
    }
    assert_eq!(counter(&world, killer, 2359), None);
    for (index, &(class, criterion)) in CLASSES.iter().enumerate() {
        let victim = spawn_class_victim(&mut world, class, 1);
        hit(&mut world, killer, victim, 1.0);
        assert_eq!(counter(&world, killer, criterion), None, "nonlethal");
        hit(&mut world, killer, victim, 1000.0);
        assert_eq!(world.get::<Health>(victim).unwrap().current, 0.0);
        assert_eq!(counter(&world, killer, 63413), Some(index as u64 + 1));
        assert_eq!(
            counter(&world, killer, criterion),
            Some(1),
            "actual class{class}"
        );
        assert_eq!(
            counter(&world, peer, criterion),
            Some(1),
            "nonfinal group recipient"
        );
        for player in [killer, peer] {
            let account = world
                .get::<PlayerAchievementState>(player)
                .unwrap()
                .account
                .as_ref()
                .unwrap();
            assert_eq!(
                account.earned_ids.contains(&245),
                index + 1 == CLASSES.len()
            );
            assert_eq!(
                world
                    .get::<PlayerPvpState>(player)
                    .unwrap()
                    .progress
                    .lifetime_honor_xp,
                0
            );
        }
        for &(_, future) in &CLASSES[index + 1..] {
            assert_eq!(
                counter(&world, killer, future),
                None,
                "other class cannot substitute"
            );
        }
        hit(&mut world, killer, victim, 1000.0);
        assert_eq!(counter(&world, killer, criterion), Some(1), "dead replay");
        assert_eq!(
            counter(&world, players[4].1, criterion),
            None,
            "unrelated character"
        );
    }
    let date = world
        .get::<PlayerAchievementState>(killer)
        .unwrap()
        .account
        .as_ref()
        .unwrap()
        .earned_at[&245];
    let saved =
        crate::persistence::load_achievements(&world.resource::<GameDatabase>().db, players[0].0);
    drop(world.remove_resource::<GameDatabase>().unwrap());
    let db = redb::Database::open(file.path()).unwrap();
    let reopened = crate::persistence::load_achievements(&db, players[0].0);
    assert_eq!(reopened.criteria_progress, saved.criteria_progress);
    for &(_, criterion) in CLASSES {
        assert_eq!(reopened.criteria_progress.get(&criterion), Some(&1));
    }
    world.entity_mut(killer).insert(reopened);
    world.insert_resource(GameDatabase::new(db));
    crate::achievement::record_player_event(
        &mut world,
        killer,
        crate::achievement::CriteriaEvent::Login,
    );
    assert_eq!(
        world
            .get::<PlayerAchievementState>(killer)
            .unwrap()
            .account
            .as_ref()
            .unwrap()
            .earned_at
            .get(&245),
        Some(&date)
    );
}

fn spawn_class_victim(world: &mut World, class: u8, faction: u8) -> Entity {
    let victim = spawn_race_victim(world, 2, faction);
    world.get_mut::<Player>(victim).unwrap().class = class;
    victim
}
