use super::*;
use crate::persistence::temp_database;
use crate::persistence_progression::save_xp;

fn mob(level: u8) -> KillVictim {
    KillVictim {
        level,
        elite: false,
        expansion: 0,
        experience_modifier: 1.0,
        gives_experience: true,
    }
}

fn member(character_id: u64, level: u8, is_killer: bool) -> KillMember {
    KillMember {
        character_id,
        level,
        alive: true,
        is_killer,
    }
}

#[test]
fn xp_to_next_level_follows_gt_xp_up_to_the_level_90_cap() {
    assert_eq!(xp_to_next_level(1), 250);
    assert_eq!(xp_to_next_level(10), 10_590);
    assert_eq!(xp_to_next_level(89), 592_980);
    assert_eq!(xp_to_next_level(90), 0);
}

#[test]
fn same_level_kill_at_level_10_gives_per_kill_xp() {
    assert_eq!(kill_gain(10, &mob(10)), 65);
    // Higher mobs add 5% per level: 65 * 1.05.
    assert_eq!(kill_gain(10, &mob(11)), 68);
    let elite = KillVictim {
        elite: true,
        ..mob(10)
    };
    assert_eq!(kill_gain(10, &elite), 130);
}

#[test]
fn grey_mobs_give_no_xp_and_green_mobs_scale_down() {
    assert_eq!(gray_level(10), 4);
    assert_eq!(kill_gain(10, &mob(4)), 0);
    // Level 5 PerKill 40 * (1 - 5/7).
    assert_eq!(kill_gain(10, &mob(5)), 11);
    assert_eq!(kill_gain(10, &mob(8)), 39);
    for (player, gray) in [(6, 0), (7, 1), (15, 8), (34, 24), (35, 25), (80, 70)] {
        assert_eq!(gray_level(player), gray, "player level {player}");
    }
}

#[test]
fn critters_give_nothing_and_old_expansion_mobs_give_a_tenth() {
    let critter = KillVictim {
        gives_experience: false,
        ..mob(10)
    };
    assert_eq!(kill_gain(10, &critter), 0);
    // Level 30 PerKill 165 / 10: classic creatures past GetExpansionForLevel 30.
    assert_eq!(kill_gain(30, &mob(30)), 17);
    assert_eq!(kill_gain(29, &mob(29)), 160);
}

#[test]
fn solo_kill_rewards_only_the_living_killer() {
    assert_eq!(
        kill_rewards(&mob(10), &[member(1, 10, true)], false),
        vec![(1, 65)]
    );
    let dead = KillMember {
        alive: false,
        ..member(1, 10, true)
    };
    assert!(kill_rewards(&mob(10), &[dead], false).is_empty());
}

#[test]
fn group_kill_xp_is_split_by_level_with_the_group_bonus() {
    let pair = [member(1, 10, true), member(2, 10, false)];
    assert_eq!(kill_rewards(&mob(10), &pair, true), vec![(1, 32), (2, 32)]);
    // 65 * 1.166 * 10 / 30.
    let trio = [
        member(1, 10, true),
        member(2, 10, false),
        member(3, 10, false),
    ];
    assert_eq!(
        kill_rewards(&mob(10), &trio, true),
        vec![(1, 25), (2, 25), (3, 25)]
    );
    // A dead member is not counted and gets nothing.
    let with_dead = [
        member(1, 10, true),
        KillMember {
            alive: false,
            ..member(2, 10, false)
        },
    ];
    assert_eq!(kill_rewards(&mob(10), &with_dead, true), vec![(1, 65)]);
}

#[test]
fn group_with_a_member_the_mob_is_grey_for_gets_half_xp() {
    // Level 8 is grey for level 20 (gray 12): XP comes from the level 10
    // member, halved plus one; the level 20 member gets none.
    let members = [member(1, 10, true), member(2, 20, false)];
    assert_eq!(kill_rewards(&mob(8), &members, true), vec![(1, 7)]);
}

#[test]
fn quest_xp_uses_quest_xp_db2_with_the_low_level_reduction() {
    // A Threat Within 783: QuestLevel 1, RewardXPDifficulty 3.
    assert_eq!(quest_xp(1, 1, 3), 100);
    assert_eq!(quest_xp(1, 6, 3), 20);
    assert_eq!(quest_xp(1, 10, 3), 10);
    // QuestLevel -1 scales to the player: level 10 difficulty 5 = 1415, rounded.
    assert_eq!(quest_xp(-1, 10, 5), 1400);
    // Level 80+: old-expansion quests give 1/9 (11765 / 9, rounded).
    assert_eq!(quest_xp(80, 80, 5), 1300);
    assert_eq!(quest_xp(255, 10, 5), 0);
}

#[test]
fn give_xp_carries_over_across_levels_and_stops_at_the_cap() {
    assert_eq!(give_xp(1, 0, 250), (2, 0));
    assert_eq!(give_xp(1, 200, 700), (2, 650));
    assert_eq!(give_xp(1, 0, 1000), (3, 95));
    assert_eq!(give_xp(89, 592_900, 200), (90, 120));
    assert_eq!(give_xp(90, 5, 100), (90, 5));
}

fn award_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GameDatabase {
        db: temp_database(),
    });
    app.add_plugins(ExperiencePlugin);
    app
}

#[test]
fn crossing_the_threshold_saves_level_and_xp_and_refills_health_and_mana() {
    let mut app = award_app();
    let db = &app.world().resource::<GameDatabase>().db;
    let (character_id, _) = persistence::create_character(db, 1, "Leveler", 1, 2).unwrap();
    save_xp(db, character_id, 200);
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Leveler".into(),
                race: 1,
                class: 2,
                appearance: default(),
            },
            ClientId(character_id),
            Level(1),
            UnitLevel(1),
            Health {
                current: 10.0,
                max: 100.0,
            },
            Mana {
                current: 5.0,
                max: 50.0,
            },
        ))
        .id();

    app.world_mut()
        .resource_mut::<XpAwards>()
        .0
        .push((character_id, 100));
    app.update();
    app.update();

    assert_eq!(app.world().get::<Level>(player), Some(&Level(2)));
    assert_eq!(app.world().get::<UnitLevel>(player), Some(&UnitLevel(2)));
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 100.0);
    assert_eq!(app.world().get::<Mana>(player).unwrap().current, 50.0);
    let db = &app.world().resource::<GameDatabase>().db;
    assert_eq!(load_xp(db, character_id), 50);
    assert_eq!(
        persistence::load_character(db, character_id).unwrap().level,
        2
    );
}

#[test]
fn dead_players_gain_no_xp() {
    let mut app = award_app();
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Ghost".into(),
                race: 1,
                class: 1,
                appearance: default(),
            },
            ClientId(7),
            Level(1),
            PlayerDeathState {
                state: PlayerLifeState::Ghost,
                ..default()
            },
        ))
        .id();
    app.world_mut().resource_mut::<XpAwards>().0.push((7, 500));
    app.update();

    assert_eq!(app.world().get::<Level>(player), Some(&Level(1)));
    assert_eq!(load_xp(&app.world().resource::<GameDatabase>().db, 7), 0);
}
