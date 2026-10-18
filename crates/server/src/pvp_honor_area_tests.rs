//! TC RewardHonor6756: each eligible recipient's current terrain area, not the victim's.
use super::honor_race::{counter, hit, race_world, spawn_race_victim};
use super::*;
use crate::achievement::PlayerAchievementState;
use crate::maps::InMap;
use shared::components::Position;

// Real map1803 MCNK interiors for all twelve authored Blood and Sand12411 leaves.
const DEPOSITS: &[(f32, f32, f32)] = &[
    (1325.333, 2625.333, 9.293),
    (1292.0, 2925.333, 52.357),
    (1158.667, 2892.0, 59.897),
    (1158.667, 2825.333, 39.114),
    (1492.0, 2725.333, 44.586),
    (1392.0, 2958.667, 65.012),
    (1525.333, 2825.333, 76.553),
    (1392.0, 2792.0, 11.219),
    (1425.333, 2558.667, 22.828),
    (1392.0, 2658.667, 20.276),
    (1292.0, 2758.667, 21.344),
    (1492.0, 2625.333, 31.915),
];

#[test]
fn achievements_criteria_honor_area31_full_fifty_actual_recipient_areas_and_reopen() {
    let (mut world, players, file) = race_world(1);
    let killer = players[0].1;
    let peer = players[2].1;
    let ground = crate::ground::GroundMap::new(
        std::path::Path::new(&std::env::var("GAME_SERVER_GROUND_DIR").unwrap()),
        [(1803, 1686724)].into(),
    );
    world.insert_resource(ground);
    for (index, &point) in DEPOSITS.iter().enumerate() {
        let pos = position(point);
        assert_eq!(
            world
                .resource_mut::<crate::ground::GroundMap>()
                .area_at(1803, Vec3::new(pos.x, pos.y, pos.z)),
            Some(9511 + index as u32),
            "authentic extracted terrain"
        );
    }
    place(&mut world, killer, DEPOSITS[0]);
    place(&mut world, peer, DEPOSITS[9]);
    for (faction, level) in [(2, 1), (1, 20)] {
        let victim = victim_at(&mut world, faction, DEPOSITS[9]);
        world.entity_mut(victim).insert(Level(level));
        for player in [killer, peer] {
            world
                .entity_mut(player)
                .insert(Level(if level == 20 { 30 } else { 1 }));
        }
        hit(&mut world, killer, victim, 1000.0);
    }
    for player in [killer, peer] {
        world.entity_mut(player).insert(Level(1));
        assert_eq!(counter(&world, player, 39747), None, "ineligible kills");
    }
    let mut expected = [[0_u64; 12]; 2];
    for kill in 0..50 {
        let index = kill % DEPOSITS.len();
        // First kill: killer9511, living nearby peer9520, victim9520.
        let peer_index = if kill == 0 { 9 } else { index };
        place(&mut world, killer, DEPOSITS[index]);
        place(&mut world, peer, DEPOSITS[peer_index]);
        let victim = victim_at(&mut world, 1, DEPOSITS[peer_index]);
        let before = counter(&world, killer, 39747 + index as u32);
        hit(&mut world, killer, victim, 1.0);
        assert_eq!(
            counter(&world, killer, 39747 + index as u32),
            before,
            "nonlethal"
        );
        hit(&mut world, killer, victim, 1000.0);
        assert_eq!(
            counter(&world, killer, 63413),
            Some(kill as u64 + 1),
            "native HK boundary"
        );
        expected[0][index] += 1;
        expected[1][peer_index] += 1;
        for (recipient, player) in [killer, peer].into_iter().enumerate() {
            for (leaf, &count) in expected[recipient].iter().enumerate() {
                assert_eq!(
                    counter(&world, player, 39747 + leaf as u32),
                    (count > 0).then_some(count),
                    "recipient{recipient} leaf{leaf} kill{kill}"
                );
            }
            let account = world
                .get::<PlayerAchievementState>(player)
                .unwrap()
                .account
                .as_ref()
                .unwrap();
            assert_eq!(
                account.earned_ids.contains(&12411),
                kill == 49,
                "full authored50 threshold"
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
        hit(&mut world, killer, victim, 1000.0);
        assert_eq!(
            counter(&world, killer, 39747 + index as u32),
            Some(expected[0][index]),
            "dead replay"
        );
        assert_eq!(
            counter(&world, players[4].1, 39747 + index as u32),
            None,
            "unrelated"
        );
    }
    let date = world
        .get::<PlayerAchievementState>(killer)
        .unwrap()
        .account
        .as_ref()
        .unwrap()
        .earned_at[&12411];
    // Missing terrain must not borrow the victim's area or stale recipient position.
    world.entity_mut(killer).insert(Position {
        x: 1_000_000.0,
        y: 0.0,
        z: 0.0,
    });
    let victim = victim_at(&mut world, 1, DEPOSITS[0]);
    hit(&mut world, killer, victim, 1000.0);
    assert_eq!(counter(&world, killer, 63413), Some(51));
    let saved =
        crate::persistence::load_achievements(&world.resource::<GameDatabase>().db, players[0].0);
    for (leaf, &count) in expected[0].iter().enumerate() {
        assert_eq!(
            saved.criteria_progress.get(&(39747 + leaf as u32)),
            Some(&count)
        );
    }
    drop(world.remove_resource::<GameDatabase>().unwrap());
    let db = redb::Database::open(file.path()).unwrap();
    let reopened = crate::persistence::load_achievements(&db, players[0].0);
    assert_eq!(reopened.criteria_progress, saved.criteria_progress);
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
            .earned_at[&12411],
        date
    );
}

fn position((x, y, z): (f32, f32, f32)) -> Position {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
    Position { x, y, z }
}

fn place(world: &mut World, player: Entity, point: (f32, f32, f32)) {
    world
        .entity_mut(player)
        .insert((InMap::continent(1803), position(point)));
}

fn victim_at(world: &mut World, faction: u8, point: (f32, f32, f32)) -> Entity {
    let victim = spawn_race_victim(world, 2, faction);
    place(world, victim, point);
    victim
}
