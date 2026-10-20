use super::*;
use crate::persistence::temp_database;
use crate::persistence_progression::save_xp;

fn mob(level: u8) -> KillVictim {
    KillVictim {
        level: UnitLevel(level),
        scaling: None,
        elite: false,
        experience_modifier: 1.0,
        gives_experience: true,
        player_damage_factor: 1.0,
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

/// Hogger's range (ContentTuning 73: 1-30), non-elite for plain numbers.
fn scaled_hogger() -> KillVictim {
    KillVictim {
        scaling: Some(LevelScaling {
            content_tuning_id: 73,
            min_level: 1,
            max_level: 30,
            delta: 0,
        }),
        ..mob(30)
    }
}

#[test]
fn tuned_kill_gives_xp_for_the_killers_scaled_level() {
    // Level 10 to a level 10 player: a same-level kill.
    assert_eq!(kill_gain(10, &scaled_hogger()), kill_gain(10, &mob(10)));
    assert_eq!(kill_gain(10, &scaled_hogger()), 65);
    // Capped at 30 for a level 33 player; grey to a level 40 player.
    assert_eq!(kill_gain(33, &scaled_hogger()), kill_gain(33, &mob(30)));
    assert_eq!(kill_gain(40, &scaled_hogger()), 0);
}

#[test]
fn group_members_see_a_tuned_kill_at_their_own_level() {
    // A level 40 member sees Hogger grey (30), so the level 10 killer is the
    // highest non-grey member: half XP split by level.
    let grouped = kill_rewards(
        &scaled_hogger(),
        &[member(1, 10, true), member(2, 40, false)],
        true,
    );
    assert_eq!(
        grouped.shares,
        vec![(1, (65.0 * 10.0 / 50.0 / 2.0) as u32 + 1)]
    );
}

#[test]
fn critters_give_nothing() {
    let critter = KillVictim {
        gives_experience: false,
        player_damage_factor: 1.0,
        ..mob(10)
    };
    assert_eq!(kill_gain(10, &critter), 0);
}

#[test]
fn level_35_same_level_kill_gets_full_xp_without_the_expansion_penalty() {
    // gtXp PerKill at 35; Retail's 10% old-expansion rule is omitted.
    assert_eq!(kill_gain(35, &mob(35)), 190);
}

#[test]
fn solo_kill_rewards_only_the_living_killer() {
    assert_eq!(
        kill_rewards(&mob(10), &[member(1, 10, true)], false),
        KillRewards {
            group_bonus: 1.0,
            shares: vec![(1, 65)],
        }
    );
    let dead = KillMember {
        alive: false,
        ..member(1, 10, true)
    };
    assert!(kill_rewards(&mob(10), &[dead], false).shares.is_empty());
}

#[test]
fn group_kill_xp_is_split_by_level_with_the_group_bonus() {
    let pair = [member(1, 10, true), member(2, 10, false)];
    assert_eq!(
        kill_rewards(&mob(10), &pair, true).shares,
        vec![(1, 32), (2, 32)]
    );
    // 65 * 1.166 * 10 / 30.
    let trio = [
        member(1, 10, true),
        member(2, 10, false),
        member(3, 10, false),
    ];
    assert_eq!(
        kill_rewards(&mob(10), &trio, true),
        KillRewards {
            group_bonus: 1.166,
            shares: vec![(1, 25), (2, 25), (3, 25)],
        }
    );
    // A dead member is not counted and gets nothing.
    let with_dead = [
        member(1, 10, true),
        KillMember {
            alive: false,
            ..member(2, 10, false)
        },
    ];
    assert_eq!(
        kill_rewards(&mob(10), &with_dead, true).shares,
        vec![(1, 65)]
    );
}

#[test]
fn group_with_a_member_the_mob_is_grey_for_gets_half_xp() {
    // Level 8 is grey for level 20 (gray 12): XP comes from the level 10
    // member, halved plus one; the level 20 member gets none.
    let members = [member(1, 10, true), member(2, 20, false)];
    assert_eq!(kill_rewards(&mob(8), &members, true).shares, vec![(1, 7)]);
}

#[test]
fn quest_xp_uses_quest_xp_db2_with_the_low_level_reduction() {
    // A Threat Within 783: QuestLevel 1, RewardXPDifficulty 3.
    assert_eq!(quest_xp(1, 1, 3), 100);
    assert_eq!(quest_xp(1, 6, 3), 20);
    assert_eq!(quest_xp(1, 10, 3), 10);
    // QuestLevel -1 scales to the player: level 10 difficulty 5 = 1415, rounded.
    assert_eq!(quest_xp(-1, 10, 5), 1400);
    assert_eq!(quest_xp(255, 10, 5), 0);
}

#[test]
fn level_85_quest_at_level_85_gets_the_full_quest_xp_without_the_expansion_cut() {
    // QuestXP level 85 difficulty 5 = 12440, rounded to 50; no ÷9 past 80.
    assert_eq!(quest_xp(85, 85, 5), 12_450);
    assert_eq!(quest_xp(80, 80, 5), 11_750);
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
    app.insert_resource(GameDatabase::new(temp_database()));
    app.add_plugins(ExperiencePlugin);
    app
}

/// `power_type` rows 1-4 (Mana, Rage, Energy): Flags 12480 and 29824 carry
/// `SetToMaxOnLevelUp` 0x1000, Rage's 8192 does not.
fn level_up_power_table() -> PowerTypeTable {
    let info =
        |power, max_base_power, set_to_max_on_level_up| crate::spell_cast::powers::PowerTypeInfo {
            power,
            min_power: 0,
            max_base_power,
            center_power: 0,
            default_power: 0,
            display_modifier: 1,
            regen_interrupt_ms: 0,
            regen_peace: 0.0,
            regen_combat: 0.0,
            set_to_max_on_level_up,
        };
    PowerTypeTable::fixture([
        info(PowerType::Mana, 0, true),
        info(PowerType::Rage, 1000, false),
        info(PowerType::Energy, 100, true),
    ])
}

#[test]
fn crossing_the_threshold_saves_level_and_xp_and_fills_health_mana_and_energy_not_rage() {
    let mut app = award_app();
    let table = level_up_power_table();
    let mut pools = PowerPools::from_table(
        &table,
        vec![PowerType::Energy, PowerType::Mana, PowerType::Rage],
    );
    pools.get_mut(PowerType::Energy).unwrap().current = 20.0;
    pools.get_mut(PowerType::Rage).unwrap().current = 500.0;
    app.insert_resource(table);
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
            pools,
        ))
        .id();

    app.world_mut()
        .resource_mut::<XpAwards>()
        .0
        .push(XpAward::quest(character_id, 100));
    app.update();
    app.update();

    assert_eq!(app.world().get::<Level>(player), Some(&Level(2)));
    assert_eq!(app.world().get::<UnitLevel>(player), Some(&UnitLevel(2)));
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 100.0);
    assert_eq!(app.world().get::<Mana>(player).unwrap().current, 50.0);
    let pools = app.world().get::<PowerPools>(player).unwrap();
    assert_eq!(pools.get(PowerType::Energy).unwrap().current, 100.0);
    assert_eq!(pools.get(PowerType::Rage).unwrap().current, 500.0);
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
    app.world_mut()
        .resource_mut::<XpAwards>()
        .0
        .push(XpAward::quest(7, 500));
    app.update();

    assert_eq!(app.world().get::<Level>(player), Some(&Level(1)));
    assert_eq!(load_xp(&app.world().resource::<GameDatabase>().db, 7), 0);
}

/// `RestMgr::GetRestBonusFor`: a 100 XP kill draws 100 more from a 300 XP pool, so the
/// player gains 200 and keeps 200 rested; a quest reward draws nothing.
#[test]
fn kill_xp_spends_rested_xp_for_a_matching_bonus_and_quest_xp_does_not() {
    let mut app = award_app();
    let victim = app.world_mut().spawn_empty().id();
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Rested".into(),
                race: 1,
                class: 1,
                appearance: default(),
            },
            ClientId(9),
            Level(10),
            PlayerRestState::fixture(10, 300),
        ))
        .id();
    app.world_mut().resource_mut::<XpAwards>().0.extend([
        XpAward {
            character_id: 9,
            amount: 100,
            victim: Some(victim),
            group_bonus: 1.0,
            reason: XpGainReason::Kill,
        },
        XpAward::quest(9, 50),
    ]);
    app.update();

    assert_eq!(load_xp(&app.world().resource::<GameDatabase>().db, 9), 250);
    assert_eq!(
        app.world()
            .get::<PlayerRestState>(player)
            .unwrap()
            .rested_xp(),
        200
    );
}

#[test]
#[ignore = "aura-world native RED/GREEN gate"]
fn aura_world_kill_xp_enlisted_before_rested_and_honorless_control() {
    let data = crate::test_data::world_data();
    for (spell, boosted) in [(269083, true), (282559, false)] {
        let mut app = award_app();
        app.insert_resource(data.clone());
        let auras = crate::auras::world_xp_tests::native_auras(spell);
        let player = app
            .world_mut()
            .spawn((
                Player {
                    name: "XpAura".into(),
                    race: 1,
                    class: 1,
                    appearance: default(),
                },
                ClientId(9),
                Level(10),
                Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                crate::maps::InMap::continent(0),
                PlayerRestState::fixture(10, 1000),
                auras,
            ))
            .id();
        let npc = Npc {
            template_id: 6,
            name: "Kobold Vermin".into(),
        };
        let victim = victim_of(data, &npc, &UnitLevel(10), None).unwrap();
        let base = kill_gain(10, &victim);
        let expected = if boosted {
            (base as f32 * 1.1) as u32
        } else {
            base
        };
        app.world_mut().spawn((
            npc,
            UnitLevel(10),
            CreatureTap::tapped_by(9),
            Health {
                current: 0.0,
                max: 100.0,
            },
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            crate::maps::InMap::continent(0),
            DeathState {
                respawn_timer: 100.0,
            },
        ));
        app.update();
        assert_eq!(
            load_xp(&app.world().resource::<GameDatabase>().db, 9),
            expected * 2,
            "native{spell}: XP before rested"
        );
        assert_eq!(
            app.world()
                .get::<PlayerRestState>(player)
                .unwrap()
                .rested_xp(),
            1000 - expected
        );
    }
}
