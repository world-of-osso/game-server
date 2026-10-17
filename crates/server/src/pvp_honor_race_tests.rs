//! TC RewardHonor6755 credits eligible recipients with the actual victim's race.
use super::*;
use crate::achievement::PlayerAchievementState;
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use crate::maps::InMap;
use crate::spell_cast::effects::{EffectCtx, deal_damage};
use shared::components::{CurrentPvpFaction, Health, Player, Position};

const HORDE_RACES: &[(u8, u32)] = &[
    (2, 2370),
    (5, 2372),
    (8, 2373),
    (6, 2371),
    (10, 2369),
    (9, 16764),
    (26, 19207),
    (27, 39736),
    (28, 39737),
    (70, 66738),
];
const ALLIANCE_RACES: &[(u8, u32)] = &[
    (1, 2375),
    (7, 2376),
    (3, 2377),
    (4, 2378),
    (11, 2374),
    (22, 16765),
    (25, 19208),
    (30, 39738),
    (29, 39739),
    (52, 66739),
];

#[test]
fn achievements_criteria_honor_race53_alliance_full_ten_races_and_reopen() {
    prove_race_achievement(1, 246, 1005, HORDE_RACES);
}

#[test]
fn achievements_criteria_honor_race53_horde_full_ten_races_and_reopen() {
    prove_race_achievement(2, 1005, 246, ALLIANCE_RACES);
}

fn prove_race_achievement(owner_race: u8, achievement: u32, opposite: u32, races: &[(u8, u32)]) {
    let (mut world, players, file) = race_world(owner_race);
    let killer = players[0].1;
    let peer = players[2].1;
    let enemy_faction = if owner_race == 1 { 1 } else { 2 };
    let (first_race, first_criterion) = races[0];
    let friendly = spawn_race_victim(&mut world, first_race, 3 - enemy_faction);
    hit(&mut world, killer, friendly, 1000.0);
    let gray = spawn_race_victim(&mut world, first_race, enemy_faction);
    for player in [killer, peer] {
        world.entity_mut(player).insert(Level(30));
    }
    world.entity_mut(gray).insert(Level(20));
    hit(&mut world, killer, gray, 1000.0);
    for player in [killer, peer] {
        world.entity_mut(player).insert(Level(1));
    }
    assert_eq!(counter(&world, killer, first_criterion), None);
    for (index, &(race, criterion)) in races.iter().enumerate() {
        let victim = spawn_race_victim(&mut world, race, enemy_faction);
        hit(&mut world, killer, victim, 1.0);
        assert_eq!(counter(&world, killer, criterion), None);
        hit(&mut world, killer, victim, 1000.0);
        assert_eq!(world.get::<Health>(victim).unwrap().current, 0.0);
        assert_eq!(
            counter(&world, killer, 63413),
            Some(index as u64 + 1),
            "eligible native35 precedes missing race credit"
        );
        assert_eq!(
            counter(&world, killer, criterion),
            Some(1),
            "actual race{race}"
        );
        assert_eq!(
            counter(&world, peer, criterion),
            Some(1),
            "eligible group recipient, not final blow"
        );
        for player in [killer, peer] {
            let account = world
                .get::<PlayerAchievementState>(player)
                .unwrap()
                .account
                .as_ref()
                .unwrap();
            assert_eq!(
                account.earned_ids.contains(&achievement),
                index + 1 == races.len()
            );
            assert!(!account.earned_ids.contains(&opposite));
            assert_eq!(
                world
                    .get::<PlayerPvpState>(player)
                    .unwrap()
                    .progress
                    .lifetime_honor_xp,
                0
            );
        }
        for &(_, future_criterion) in &races[index + 1..] {
            assert_eq!(
                counter(&world, killer, future_criterion),
                None,
                "other race cannot substitute"
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
        .earned_at[&achievement];
    let db = &world.resource::<GameDatabase>().db;
    let saved = crate::persistence::load_achievements(db, players[0].0);
    drop(world.remove_resource::<GameDatabase>().unwrap());
    let db = redb::Database::open(file.path()).unwrap();
    let reopened = crate::persistence::load_achievements(&db, players[0].0);
    assert_eq!(reopened.criteria_progress, saved.criteria_progress);
    for &(_, criterion) in races {
        assert_eq!(reopened.criteria_progress.get(&criterion), Some(&1));
    }
    world.entity_mut(killer).insert(reopened);
    world.insert_resource(GameDatabase { db });
    crate::achievement::record_player_event(
        &mut world,
        killer,
        crate::achievement::CriteriaEvent::Login,
    );
    let restored = world.get::<PlayerAchievementState>(killer).unwrap();
    assert_eq!(
        restored
            .account
            .as_ref()
            .unwrap()
            .earned_at
            .get(&achievement),
        Some(&date)
    );
}

fn race_world(owner_race: u8) -> (World, Vec<(u64, Entity)>, tempfile::NamedTempFile) {
    let (mut world, players, file) = native_flag_match(&shared::battleground::warsong_gulch());
    world.init_resource::<Time>();
    world.insert_resource(
        crate::spell_cast::test_harness::world_db()
            .unwrap()
            .store
            .clone(),
    );
    world.init_resource::<Messages<crate::death::PlayerDeathTransition>>();
    world.init_resource::<crate::death::DeathServerState>();
    world.init_resource::<crate::death::DeathTransitionCounters>();
    world.remove_resource::<PvpServerState>();
    for index in [0, 2] {
        let player = players[index].1;
        world.get_mut::<Player>(player).unwrap().race = owner_race;
        world.entity_mut(player).insert((
            InMap::continent(0),
            CurrentPvpFaction(if owner_race == 1 { 2 } else { 1 }),
            Level(1),
            PlayerPvpState::default(),
        ));
    }
    let party = [players[0], players[2]];
    let online: OnlineView = party
        .iter()
        .map(|&(character, player)| {
            (
                character,
                OnlineMember {
                    entity: player.to_bits(),
                    name: world.get::<Player>(player).unwrap().name.clone(),
                    class: 1,
                    level: 1,
                    portrait: Default::default(),
                    dungeon_difficulty: 1,
                    raid_difficulty: Default::default(),
                },
            )
        })
        .collect();
    let mut groups = GroupRegistry::default();
    groups.sync_presence(&online);
    groups.invite(party[0].0, &online[&party[1].0].name, &online);
    groups.respond_invite(party[1].0, true);
    assert_eq!(groups.group_members(party[0].0).len(), 2);
    world.insert_resource(groups);
    (world, players, file)
}

fn spawn_race_victim(world: &mut World, race: u8, faction: u8) -> Entity {
    world
        .spawn((
            Player {
                name: format!("Victim{race}"),
                race,
                class: 1,
                appearance: Default::default(),
            },
            InMap::continent(0),
            CurrentPvpFaction(faction),
            Level(1),
            Position {
                x: 3.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id()
}

fn hit(world: &mut World, killer: Entity, victim: Entity, amount: f32) {
    let data = crate::spell_cast::test_harness::world_db().unwrap();
    let ctx = EffectCtx {
        caster: killer,
        spell: data.store.get(133).unwrap(),
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    deal_damage(world, &ctx, victim, amount, false, false);
}

fn counter(world: &World, player: Entity, criterion: u32) -> Option<u64> {
    world
        .get::<PlayerAchievementState>(player)
        .unwrap()
        .criteria_progress
        .get(&criterion)
        .copied()
}
