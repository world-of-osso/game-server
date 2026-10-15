use super::*;

#[test]
fn achievements_criteria_unearned_snapshot_rejects_missing_earned_component() {
    let mut world = World::new();
    world.insert_resource(catalog());
    let actor = world
        .spawn((
            shared::components::Player {
                name: "Noearned".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            shared::components::Level(1),
            crate::maps::InMap::continent(0),
        ))
        .id();
    let event = CriteriaEvent::CastSpell(1217637);
    let conditions = crate::achievement::event_conditions(&world, actor, &[event]);
    assert!(
        conditions.player.is_some(),
        "other reference-player facts are available"
    );
    let mut state = PlayerAchievementState::default();
    world
        .resource::<AchievementCatalog>()
        .update_at(&mut state, event, None, 1, &conditions);
    assert!(
        !state.criteria_progress.contains_key(&103451),
        "a missing earned-state component is unknown, not an empty known-unearned set"
    );
    world
        .entity_mut(actor)
        .insert(PlayerAchievementState::default());
    let conditions = crate::achievement::event_conditions(&world, actor, &[event]);
    world
        .resource::<AchievementCatalog>()
        .update_at(&mut state, event, None, 1, &conditions);
    assert!(
        !state.criteria_progress.contains_key(&103451),
        "character state without its account aggregate is incomplete earned authority"
    );
    let catalog = world.remove_resource::<AchievementCatalog>().unwrap();
    let conditions = crate::achievement::event_conditions(&world, actor, &[event]);
    catalog.update_at(&mut state, event, None, 1, &conditions);
    assert!(
        !state.criteria_progress.contains_key(&103451),
        "a missing catalog also cannot authorize known-unearned credit"
    );
}

#[test]
fn achievements_criteria_unearned_cast_requires_reference_and_authored_award() {
    let catalog = catalog();
    for (player, expected) in [
        (None, None),
        (Some(ReferencePlayer::default()), None),
        (
            Some(ReferencePlayer {
                earned_achievements: Some(BTreeSet::new()),
                account_achievements_known: true,
                ..Default::default()
            }),
            Some(1),
        ),
    ] {
        let mut state = PlayerAchievementState::default();
        let conditions = PlayerConditions {
            player,
            ..Default::default()
        };
        catalog.update_at(
            &mut state,
            CriteriaEvent::CastSpell(1217637),
            None,
            1,
            &conditions,
        );
        assert_eq!(state.criteria_progress.get(&103451).copied(), expected);
    }
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    assert!(
        load_criteria_facts(&conn, &[(-87, 41707)], &HashMap::new())
            .unwrap()
            .is_some()
    );
    assert!(
        load_criteria_facts(&conn, &[(-87, u32::MAX)], &HashMap::new())
            .unwrap()
            .is_none(),
        "unknown authored metadata is not known-unearned authority"
    );
}

#[test]
fn achievements_criteria_pet_type_level_resolves_species_and_preserves_zero_type() {
    let catalog = catalog();
    let conditions = PlayerConditions {
        player: Some(ReferencePlayer::default()),
        ..Default::default()
    };
    for (species, level, expected) in [
        (39, 25, Some(21377)),
        (83, 25, Some(21377)),
        (1426, 25, Some(21378)),
        (93, 25, Some(21378)),
        (107, 25, Some(21376)),
        (39, 10, Some(21367)),
        (1426, 10, Some(21368)),
        (107, 10, Some(21366)),
        (39, 24, None),
        (0, 25, None),
        (999999, 25, None),
    ] {
        let mut state = PlayerAchievementState::default();
        catalog.update_at(
            &mut state,
            CriteriaEvent::BattlePetReachedLevel { species, level },
            None,
            1,
            &conditions,
        );
        for criterion in [21366, 21367, 21368, 21376, 21377, 21378] {
            assert_eq!(
                state.criteria_progress.get(&criterion),
                (expected == Some(criterion)).then_some(&1),
                "species{species}/level{level} criterion{criterion}"
            );
        }
    }
    let mut state = PlayerAchievementState::default();
    catalog.update_at(
        &mut state,
        CriteriaEvent::ActivelyEarnedPetLevel {
            species: 39,
            level: 25,
        },
        None,
        1,
        &conditions,
    );
    assert!(
        !state.criteria_progress.contains_key(&21377),
        "type162 cannot impersonate160"
    );
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE battle_pet_species(species_id INTEGER PRIMARY KEY);
        CREATE TABLE battle_pet_species_metadata(ID INTEGER PRIMARY KEY,PetTypeEnum INTEGER);",
    )
    .unwrap();
    assert!(load_battle_pet_type_fact(&conn, 10).unwrap().is_none());
    assert!(load_battle_pet_type_fact(&conn, 9).unwrap().is_none());
    conn.execute("INSERT INTO battle_pet_species_metadata VALUES(39,9)", [])
        .unwrap();
    assert!(
        load_battle_pet_type_fact(&conn, 9).unwrap().is_none(),
        "metadata alone cannot supply an authored species"
    );
    conn.execute("INSERT INTO battle_pet_species VALUES(39)", [])
        .unwrap();
    let fact = load_battle_pet_type_fact(&conn, 9).unwrap().unwrap();
    let player = ReferencePlayer::default();
    assert!(fact.holds(
        &player,
        CriteriaEvent::BattlePetReachedLevel {
            species: 39,
            level: 25
        },
        &BTreeMap::new()
    ));
    assert!(!fact.holds(
        &player,
        CriteriaEvent::BattlePetReachedLevel {
            species: 1426,
            level: 25
        },
        &BTreeMap::new()
    ));
}

#[test]
fn achievements_criteria_killed_creature_type_requires_actual_known_victim() {
    let catalog = catalog();
    let conditions = PlayerConditions {
        player: Some(ReferencePlayer::default()),
        ..Default::default()
    };
    for kind in [None, Some(0), Some(1), Some(7), Some(16)] {
        let mut state = PlayerAchievementState::default();
        catalog.update_at(
            &mut state,
            CriteriaEvent::KillAnyCreature(kind),
            None,
            1,
            &conditions,
        );
        assert_eq!(state.criteria_progress.get(&4944), Some(&1));
        assert_eq!(
            state.criteria_progress.get(&4948),
            (kind == Some(1)).then_some(&1)
        );
        assert_eq!(
            state.criteria_progress.get(&4953),
            (kind == Some(7)).then_some(&1)
        );
        assert!(!state.criteria_progress.contains_key(&4950));
    }
    let conn = Connection::open_in_memory().unwrap();
    for kind in [0, 16, u32::MAX] {
        assert!(
            load_criteria_facts(&conn, &[(30, kind)], &HashMap::new())
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn achievements_criteria_node_world_state_facts_require_live_count_authority() {
    let catalog = catalog();
    for (counts, qualifies) in [
        (None, false),
        (Some([0, 0]), false),
        (Some([2, 0]), false),
        (Some([3, 0]), true),
        (Some([0, 3]), true),
        (Some([5, 0]), false),
    ] {
        let mut state = PlayerAchievementState::default();
        let conditions = PlayerConditions {
            player: Some(ReferencePlayer {
                map: 761,
                controlled_node_counts: counts,
                ..Default::default()
            }),
            ..Default::default()
        };
        catalog.update_at(
            &mut state,
            CriteriaEvent::WinBattleground(761),
            Some(761),
            1,
            &conditions,
        );
        assert_eq!(state.criteria_progress.get(&14943), qualifies.then_some(&1));
    }
    let conn = Connection::open_in_memory().unwrap();
    assert!(
        load_criteria_facts(&conn, &[(67, 999999)], &HashMap::new())
            .unwrap()
            .is_none()
    );
    assert!(
        load_criteria_facts(&conn, &[(67, 8656), (67, 999999)], &HashMap::new())
            .unwrap()
            .is_none()
    );
    let five = load_criteria_facts(&conn, &[(67, 5717)], &HashMap::new())
        .unwrap()
        .unwrap();
    for (counts, qualifies) in [
        (None, false),
        (Some([3, 0]), false),
        (Some([5, 0]), true),
        (Some([0, 5]), true),
    ] {
        let player = ReferencePlayer {
            controlled_node_counts: counts,
            ..Default::default()
        };
        assert_eq!(
            five[0].holds(&player, CriteriaEvent::WinAnyBattleground, &BTreeMap::new()),
            qualifies
        );
    }
}

#[test]
fn achievements_criteria_pet_level_species_requires_authored_species_and_exact_event_level() {
    let mut catalog = catalog();
    for (species, level, qualifies) in [
        (1426, 25, true),
        (1426, 24, false),
        (39, 25, false),
        (0, 25, false),
    ] {
        let mut state = PlayerAchievementState::default();
        catalog.update_at(
            &mut state,
            CriteriaEvent::BattlePetReachedLevel { species, level },
            None,
            1,
            &PlayerConditions::default(),
        );
        assert_eq!(state.criteria_progress.get(&25123), qualifies.then_some(&1));
    }
    let mut state = PlayerAchievementState::default();
    catalog.update_at(
        &mut state,
        CriteriaEvent::ActivelyEarnedPetLevel {
            species: 1974,
            level: 25,
        },
        None,
        1,
        &PlayerConditions::default(),
    );
    assert_eq!(state.criteria_progress.get(&34667), Some(&1));
    assert_eq!(
        state.criteria_progress.get(&34921),
        None,
        "type162 cannot impersonate160"
    );
    catalog.pet_species.remove(&1426);
    catalog.pet_species.remove(&1974);
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    load_criteria(&conn, &mut catalog).unwrap();
    assert!(
        catalog
            .snapshot(&PlayerAchievementState::default(), 1)
            .progress
            .iter()
            .all(|progress| progress.achievement_id != 9070),
        "missing authored species must remove9070 from the supported catalog"
    );
}

#[test]
fn achievements_criteria_earned_prerequisite_requires_known_id_and_scoped_player_awards() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE achievement (ID INTEGER PRIMARY KEY);")
        .unwrap();
    assert!(
        load_criteria_facts(&conn, &[(87, 10460)], &HashMap::new())
            .unwrap()
            .is_none()
    );
    conn.execute("INSERT INTO achievement VALUES (10460)", [])
        .unwrap();
    let facts = load_criteria_facts(&conn, &[(87, 10460)], &HashMap::new())
        .unwrap()
        .unwrap();
    let catalog = catalog();
    for (own, qualifies) in [(BTreeSet::new(), false), (BTreeSet::from([10460]), true)] {
        let state = PlayerAchievementState {
            earned_ids: own,
            account: Some(Box::new(PlayerAchievementState {
                earned_ids: BTreeSet::from([10460, 291]),
                ..Default::default()
            })),
            ..Default::default()
        };
        let player = ReferencePlayer {
            earned_achievements: Some(catalog.iter_earned_achievement_ids(&state).collect()),
            ..Default::default()
        };
        assert!(player.earned_achievements.as_ref().unwrap().contains(&291));
        assert_eq!(
            facts[0].holds(&player, CriteriaEvent::KillPlayer, &BTreeMap::new()),
            qualifies,
            "account-wide291 is known, but only this character can supply10460"
        );
    }
    assert!(!facts[0].holds(
        &ReferencePlayer::default(),
        CriteriaEvent::KillPlayer,
        &BTreeMap::new()
    ));
}

#[test]
fn achievements_criteria_pet_team_threshold_rejects_unknown_and_preserves_other_qualifiers() {
    let catalog = catalog();
    for (minimum, qualifies) in [(None, false), (Some(24), false), (Some(25), true)] {
        let mut state = PlayerAchievementState::default();
        let conditions = PlayerConditions {
            player: Some(ReferencePlayer {
                battle_pet_team_min_level: minimum,
                ..Default::default()
            }),
            ..Default::default()
        };
        catalog.update_at(
            &mut state,
            CriteriaEvent::WinPetBattle,
            None,
            1,
            &conditions,
        );
        assert_eq!(state.criteria_progress.get(&23257), qualifies.then_some(&1));
        assert!(
            !state.criteria_progress.contains_key(&23306),
            "NYI PvP90 remains required"
        );
        assert!(!state.earned_ids.contains(&8278));
    }
    let mut state = PlayerAchievementState::default();
    catalog.update_at(
        &mut state,
        CriteriaEvent::WinPetBattle,
        None,
        1,
        &PlayerConditions::default(),
    );
    assert!(
        !state.criteria_progress.contains_key(&23257),
        "map-wide credit has no pet journal"
    );
}

#[test]
fn achievements_criteria_known_spell_needs_player_knowledge_and_an_authored_spell_row() {
    let catalog = catalog();
    for (spells, qualifies) in [(vec![], false), (vec![432962], false), (vec![406106], true)] {
        let mut state = PlayerAchievementState::default();
        let conditions = PlayerConditions {
            player: Some(ReferencePlayer {
                known_spells: spells.into_iter().collect(),
                ..Default::default()
            }),
            ..Default::default()
        };
        catalog.update_at(
            &mut state,
            CriteriaEvent::LearnSpell(370465),
            None,
            1,
            &conditions,
        );
        assert_eq!(
            state.criteria_progress.get(&61271).copied(),
            qualifies.then_some(1)
        );
    }
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE spell (ID INTEGER PRIMARY KEY);")
        .unwrap();
    assert!(
        load_criteria_facts(&conn, &[(104, 406106)], &HashMap::new())
            .unwrap()
            .is_none()
    );
    conn.execute("INSERT INTO spell VALUES (406106)", [])
        .unwrap();
    let facts = load_criteria_facts(&conn, &[(104, 406106)], &HashMap::new())
        .unwrap()
        .unwrap();
    assert!(!facts[0].holds(
        &ReferencePlayer::default(),
        CriteriaEvent::LearnSpell(370465),
        &BTreeMap::new()
    ));
}

#[test]
fn achievements_criteria_item_template_predicates_match_ids_and_quality_without_dropping_missing_items()
 {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE content_item (ID INTEGER PRIMARY KEY, Quality INTEGER);
        INSERT INTO content_item VALUES (33999,4),(124101,1),(45199,0);",
    )
    .unwrap();
    let player = ReferencePlayer::default();
    let epic = load_item_quality_fact(&conn, 15, 4).unwrap().unwrap();
    assert!(epic.holds(
        &player,
        CriteriaEvent::ObtainAnyItem(33999),
        &BTreeMap::new()
    ));
    assert!(epic.holds(&player, CriteriaEvent::LootAnyItem(33999), &BTreeMap::new()));
    assert!(!epic.holds(
        &player,
        CriteriaEvent::ObtainAnyItem(124101),
        &BTreeMap::new()
    ));
    assert!(!epic.holds(
        &player,
        CriteriaEvent::ObtainAnyItem(999999),
        &BTreeMap::new()
    ));
    let rare_or_better = load_item_quality_fact(&conn, 14, 3).unwrap().unwrap();
    assert!(rare_or_better.holds(&player, CriteriaEvent::LootAnyItem(33999), &BTreeMap::new()));
    assert!(!rare_or_better.holds(&player, CriteriaEvent::LootAnyItem(45199), &BTreeMap::new()));
    assert!(
        load_event_item_alternatives(&conn, &[45199, 173032])
            .unwrap()
            .is_none()
    );
    conn.execute("INSERT INTO content_item VALUES (173032,1)", [])
        .unwrap();
    let alternatives = load_event_item_alternatives(&conn, &[45199, 173032])
        .unwrap()
        .unwrap();
    assert!(alternatives.holds(&player, CriteriaEvent::LootAnyItem(45199), &BTreeMap::new()));
    assert!(alternatives.holds(
        &player,
        CriteriaEvent::LootAnyItem(173032),
        &BTreeMap::new()
    ));
    assert!(!alternatives.holds(
        &player,
        CriteriaEvent::LootAnyItem(124101),
        &BTreeMap::new()
    ));
}

#[test]
fn achievements_criteria_item_receipts_sum_distinct_character_contributions() {
    let catalog = catalog();
    let contributions = [2, 3].map(|count| {
        let mut state = PlayerAchievementState::default();
        state.criteria_progress.insert(6140, count);
        state.criteria_progress.insert(6142, count);
        state
    });
    let aggregate = catalog.aggregate(contributions.into_iter());
    assert_eq!(aggregate.criteria_progress.get(&6140), Some(&5));
    assert_eq!(aggregate.criteria_progress.get(&6142), Some(&5));
}

#[test]
fn achievements_criteria_obtain_item_keeps_pinned_statistic_only_completion_contradictions_disabled()
 {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    let conditions = PlayerConditions {
        player: Some(ReferencePlayer::default()),
        ..Default::default()
    };
    for item in [163036, 169202] {
        catalog.update_at(
            &mut state,
            CriteriaEvent::ObtainAnyItem(item),
            None,
            1,
            &conditions,
        );
    }
    assert!(!state.criteria_progress.contains_key(&41244));
    assert!(!state.criteria_progress.contains_key(&45924));
    assert!(!state.earned_ids.contains(&12927));
    assert!(!state.earned_ids.contains(&13707));
}

#[test]
fn achievements_criteria_fall_alive_and_area_leaves_remain_required() {
    let catalog = catalog();
    for (alive, area, puddle) in [
        (Some(true), Some(6665), true),
        (Some(true), Some(1519), false),
        (Some(true), None, false),
        (Some(false), Some(6665), false),
        (None, Some(6665), false),
    ] {
        let mut state = PlayerAchievementState::default();
        let conditions = PlayerConditions {
            player: Some(ReferencePlayer {
                alive,
                area,
                ..Default::default()
            }),
            ..Default::default()
        };
        catalog.update_at(
            &mut state,
            CriteriaEvent::FallWithoutDying(3000),
            None,
            1,
            &conditions,
        );
        assert_eq!(
            state.criteria_progress.get(&2072).copied(),
            (alive == Some(true)).then_some(3000)
        );
        assert_eq!(
            state.criteria_progress.get(&23423).copied(),
            puddle.then_some(3000)
        );
        // Definition8354 also names completed map1105; this isolates the authored
        // fall predicates, not a live battleground-completion grant.
        assert!(!state.earned_ids.contains(&8354));
        assert!(
            !state.criteria_progress.contains_key(&3881),
            "world-state predicate cannot be silently dropped"
        );
    }
}

#[test]
fn achievements_criteria_player_aura_some_rejects_a_missing_spell_alternative_at_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("aura-some.db");
    let source = Connection::open(crate::test_data::world_db_path()).unwrap();
    source
        .execute("VACUUM INTO ?1", [path.to_str().unwrap()])
        .unwrap();
    let conn = Connection::open(&path).unwrap();
    conn.execute("DELETE FROM spell WHERE ID = 136778", [])
        .unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    let mut state = PlayerAchievementState::default();
    let conditions = PlayerConditions {
        player: Some(ReferencePlayer {
            auras: [136775].into_iter().collect(),
            map: 0,
            race: 1,
            class: 1,
            level: 1,
            ..Default::default()
        }),
        ..Default::default()
    };
    catalog.update_at(
        &mut state,
        CriteriaEvent::BeSpellTarget(136749),
        None,
        1,
        &conditions,
    );
    assert!(
        !state.criteria_progress.contains_key(&22840),
        "missing spell cannot be silently removed"
    );
}

#[test]
fn achievements_criteria_player_area_some_accepts_alternatives_and_authored_descendants() {
    let catalog = catalog();
    for (area, qualifies) in [
        (Some(1377), true),
        (Some(9310), true),
        (Some(2738), true),
        (Some(1519), false),
        (None, false),
    ] {
        let mut state = PlayerAchievementState::default();
        let conditions = PlayerConditions {
            player: Some(ReferencePlayer {
                area,
                map: 1,
                race: 1,
                class: 1,
                level: 30,
                ..Default::default()
            }),
            ..Default::default()
        };
        catalog.update_at(
            &mut state,
            CriteriaEvent::CastSpell(61818),
            None,
            1,
            &conditions,
        );
        assert_eq!(
            state.criteria_progress.get(&9142),
            qualifies.then_some(&1),
            "area {area:?}"
        );
    }
}

#[test]
fn achievements_criteria_player_area_some_rejects_a_missing_alternative_at_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("area-some.db");
    let source = Connection::open(crate::test_data::world_db_path()).unwrap();
    source
        .execute("VACUUM INTO ?1", [path.to_str().unwrap()])
        .unwrap();
    let conn = Connection::open(&path).unwrap();
    conn.execute("DELETE FROM area_table WHERE ID = 9310", [])
        .unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    let mut state = PlayerAchievementState::default();
    let conditions = PlayerConditions {
        player: Some(ReferencePlayer {
            area: Some(1377),
            map: 1,
            race: 1,
            class: 1,
            level: 30,
            ..Default::default()
        }),
        ..Default::default()
    };
    catalog.update_at(
        &mut state,
        CriteriaEvent::CastSpell(61818),
        None,
        1,
        &conditions,
    );
    assert!(
        !state.criteria_progress.contains_key(&9142),
        "missing alternative cannot be silently removed"
    );
}

#[test]
fn achievements_criteria_player_map_some_requires_an_authored_reference_map() {
    let catalog = catalog();
    for (map, qualifies) in [
        (Some(489), true),
        (Some(2106), true),
        (Some(0), false),
        (Some(726), false),
        (None, false),
    ] {
        let mut state = PlayerAchievementState::default();
        let conditions = PlayerConditions {
            player: map.map(|map| ReferencePlayer {
                map,
                race: 1,
                class: 1,
                level: 30,
                ..Default::default()
            }),
            ..Default::default()
        };
        catalog.update_at(
            &mut state,
            CriteriaEvent::DeliveredKillingBlow,
            None,
            1,
            &conditions,
        );
        assert_eq!(
            state.criteria_progress.get(&5538),
            qualifies.then_some(&1),
            "map {map:?}"
        );
    }
}

#[test]
fn achievements_criteria_player_map_some_rejects_a_missing_alternative_at_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("map-some.db");
    let source = Connection::open(crate::test_data::world_db_path()).unwrap();
    source
        .execute("VACUUM INTO ?1", [path.to_str().unwrap()])
        .unwrap();
    let conn = Connection::open(&path).unwrap();
    conn.execute("DELETE FROM map WHERE ID = 489", []).unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    let mut state = PlayerAchievementState::default();
    let conditions = PlayerConditions {
        player: Some(ReferencePlayer {
            map: 2106,
            race: 1,
            class: 1,
            level: 30,
            ..Default::default()
        }),
        ..Default::default()
    };
    catalog.update_at(
        &mut state,
        CriteriaEvent::DeliveredKillingBlow,
        None,
        1,
        &conditions,
    );
    assert!(
        !state.criteria_progress.contains_key(&5538),
        "missing alternative cannot be silently removed"
    );
}

#[test]
fn achievements_criteria_zone_pet_win_uses_one_flagged_parent_not_ancestry() {
    let catalog = catalog();
    for (area, qualifies) in [
        (Some(1519), true),
        (Some(5148), true),
        (Some(13862), false),
        (None, false),
    ] {
        let mut state = PlayerAchievementState::default();
        let conditions = PlayerConditions {
            player: Some(ReferencePlayer {
                area,
                map: 0,
                race: 1,
                class: 1,
                level: 1,
                ..Default::default()
            }),
            ..Default::default()
        };
        catalog.update_at(
            &mut state,
            CriteriaEvent::WinPetBattle,
            None,
            1,
            &conditions,
        );
        assert_eq!(
            state.criteria_progress.get(&19845),
            qualifies.then_some(&1),
            "area {area:?}"
        );
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("zone41.db");
    let source = Connection::open(crate::test_data::world_db_path()).unwrap();
    source
        .execute("VACUUM INTO ?1", [path.to_str().unwrap()])
        .unwrap();
    let copy = Connection::open(&path).unwrap();
    // A parent alone does not establish a zone; a flagged child uses only one parent.
    copy.execute_batch(
        "UPDATE area_table SET Flags_0 = Flags_0 & ~1073741824 WHERE ID=5148;
        UPDATE area_table SET ParentAreaID=5148 WHERE ID=5150;",
    )
    .unwrap();
    let catalog = load_achievement_catalog(&copy).unwrap();
    for area in [5148, 5150] {
        let mut state = PlayerAchievementState::default();
        let conditions = PlayerConditions {
            player: Some(ReferencePlayer {
                area: Some(area),
                map: 0,
                race: 1,
                class: 1,
                level: 1,
                ..Default::default()
            }),
            ..Default::default()
        };
        catalog.update_at(
            &mut state,
            CriteriaEvent::WinPetBattle,
            None,
            1,
            &conditions,
        );
        assert!(!state.criteria_progress.contains_key(&19845), "area {area}");
    }
}

#[test]
fn achievements_criteria_area_predicates_use_authored_ancestry_and_negative_leave_test() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    // Little Scales Daycare14096 belongs to Valdrakken13862, criterion56318.
    catalog.update(&mut state, CriteriaEvent::EnterArea(14096), None, 1);
    assert_eq!(state.criteria_progress.get(&56318), Some(&1));
    let mut outside = PlayerAchievementState::default();
    catalog.update(&mut outside, CriteriaEvent::EnterArea(11400), None, 1);
    assert!(!outside.criteria_progress.contains_key(&56318));
    // TC's LeaveArea predicate is !IsInArea(previous_area, requested_area), not equality.
    catalog.update(&mut outside, CriteriaEvent::LeaveArea(0), None, 1);
    catalog.update(&mut outside, CriteriaEvent::LeaveArea(11400), None, 1);
    assert!(!outside.criteria_progress.contains_key(&47848));
    catalog.update(&mut outside, CriteriaEvent::LeaveArea(13862), None, 1);
    assert_eq!(outside.criteria_progress.get(&47848), Some(&1));
}

#[test]
fn zero_amount_leaf_requires_a_real_positive_credit_and_reports_one_of_one() {
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    let mut state = PlayerAchievementState::default();
    // Authenticated tree3742/criterion6155: LearnOrKnowSpell32298, authored amount0.
    state.criteria_progress.insert(6155, 0);
    let before = catalog.tree_progress(3742, &state);
    assert!(!before.completed);
    assert_eq!((before.current, before.required), (0, 1));
    catalog.update(&mut state, CriteriaEvent::LearnSpell(32298), None, 1);
    let after = catalog.tree_progress(3742, &state);
    assert!(after.completed);
    assert_eq!((after.current, after.required), (1, 1));
}

use rusqlite::Connection;

fn catalog() -> AchievementCatalog {
    let db = Connection::open(crate::test_data::world_db_path()).unwrap();
    load_achievement_catalog(&db).unwrap()
}

#[test]
fn achievements_criteria_player_snapshot_never_claims_guild_actor_progress() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    state.account = Some(Box::new(PlayerAchievementState::default()));
    //Real guild5025 "Dust, Dust, and More Dust!" is not a character/account achievement.
    assert!(
        !catalog
            .snapshot(&state, 1)
            .progress
            .iter()
            .any(|row| row.achievement_id == 5025)
    );
    // Its cast criterion cannot let a single character earn the guild definition.
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let mut query = conn.prepare("WITH RECURSIVE tree(ID) AS (SELECT CriteriaTree FROM achievement WHERE ID=5025 UNION SELECT t.ID FROM criteria_tree t JOIN tree ON t.Parent=tree.ID) SELECT c.ID,c.Asset,t.Amount FROM tree JOIN criteria_tree t ON t.ID=tree.ID JOIN criteria c ON c.ID=t.CriteriaID WHERE c.Type=29").unwrap();
    let (criterion, spell, amount): (u32, u32, u64) = query
        .query_row([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap();
    state.criteria_progress.insert(criterion, amount - 1);
    catalog.update(&mut state, CriteriaEvent::CastSpell(spell), None, 1);
    assert!(!state.earned_ids.contains(&5025));
}

#[test]
fn achievements_criteria_mythic_predicates_require_authored_map_key_time_and_player_level() {
    let catalog = catalog();
    let valid = MythicCompletion {
        key_level: 10,
        challenge_map: 401,
        player_level: 70,
        in_time: true,
    };
    for completion in [
        MythicCompletion {
            key_level: 9,
            ..valid
        },
        MythicCompletion {
            challenge_map: 400,
            ..valid
        },
        MythicCompletion {
            player_level: 71,
            ..valid
        },
        MythicCompletion {
            in_time: false,
            ..valid
        },
    ] {
        let mut state = PlayerAchievementState::default();
        catalog.update(
            &mut state,
            CriteriaEvent::CompleteMythicPlus(completion),
            None,
            1,
        );
        assert!(!state.criteria_progress.contains_key(&56012));
    }
    let mut state = PlayerAchievementState::default();
    catalog.update(
        &mut state,
        CriteriaEvent::CompleteMythicPlus(valid),
        None,
        1,
    );
    assert_eq!(state.criteria_progress.get(&56012), Some(&1));
    // Seasonal criterion44064 remains disabled: no active season is invented.
    assert!(!state.criteria_progress.contains_key(&44064));
}

#[test]
fn achievements2_other_character_earned_dependencies_do_not_grant_character_meta() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    let mut account = PlayerAchievementState::default();
    account.earned_ids.extend([4788, 4789]);
    state.account = Some(Box::new(account));
    catalog.update(&mut state, CriteriaEvent::ReachLevel(1), None, 1);
    assert!(!state.earned_ids.contains(&1205));
}

#[test]
fn achievements2_holiday_and_missing_spell_assets_remain_untrackable() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    catalog.update(&mut state, CriteriaEvent::CastSpell(62049), None, 1);
    assert!(!state.criteria_progress.contains_key(&11118));
    catalog.update(&mut state, CriteriaEvent::BeSpellTarget(132022), None, 1);
    assert!(!state.criteria_progress.contains_key(&533));
}

#[test]
fn achievements_criteria_target_class_rejects_missing_target_even_when_caster_matches() {
    let catalog = catalog();
    let conditions = PlayerConditions {
        player: Some(ReferencePlayer {
            race: 2,
            class: 6,
            ..Default::default()
        }),
        ..Default::default()
    };
    for (target, qualifies) in [
        (None, false),
        (Some(SpellHitPlayer { race: 1, class: 6 }), false),
        (Some(SpellHitPlayer { race: 2, class: 1 }), false),
        (Some(SpellHitPlayer { race: 2, class: 6 }), true),
    ] {
        let mut state = PlayerAchievementState::default();
        catalog.update_at(
            &mut state,
            CriteriaEvent::LandTargetedSpell(27571, target, None, None),
            None,
            2,
            &conditions,
        );
        assert_eq!(state.criteria_progress.get(&6302), qualifies.then_some(&1));
    }
    let mut state = PlayerAchievementState::default();
    let target = Some(SpellHitPlayer { race: 2, class: 6 });
    catalog.update_at(
        &mut state,
        CriteriaEvent::LandTargetedSpell(27571, target, None, None),
        None,
        2,
        &PlayerConditions::default(),
    );
    assert!(
        !state.criteria_progress.contains_key(&6302),
        "missing caster authority stays rejected"
    );
}

#[test]
fn achievements_criteria_target_health_percent_is_inclusive_and_not_player_only() {
    let catalog = catalog();
    let conditions = PlayerConditions {
        player: Some(ReferencePlayer {
            race: 1,
            class: 1,
            level: 30,
            ..Default::default()
        }),
        ..Default::default()
    };
    for (percent, qualifies) in [
        (Some(0.0), true),
        (Some(4.99), true),
        (Some(5.0), true),
        (Some(5.01), false),
        (Some(100.0), false),
        (None, false),
    ] {
        let mut state = PlayerAchievementState::default();
        catalog.update_at(
            &mut state,
            CriteriaEvent::LandTargetedSpell(45544, None, percent, None),
            None,
            1,
            &conditions,
        );
        assert_eq!(
            state.criteria_progress.get(&6799),
            qualifies.then_some(&1),
            "actual unit health {percent:?}; no Player-only requirement exists"
        );
        assert_eq!(state.earned_ids.contains(&141), qualifies);
    }
    let mut state = PlayerAchievementState::default();
    catalog.update_at(
        &mut state,
        CriteriaEvent::LandTargetedSpell(27571, None, Some(1.0), None),
        None,
        1,
        &conditions,
    );
    assert!(
        !state.criteria_progress.contains_key(&6799),
        "wrong spell cannot credit bandaging"
    );
}

#[test]
fn achievements2_pinned_snapshot_trackable_union_matches_census() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    state.account = Some(Box::new(PlayerAchievementState::default()));
    let union: BTreeSet<_> = [1, 2]
        .into_iter()
        .flat_map(|race| {
            catalog
                .snapshot(&state, race)
                .progress
                .into_iter()
                .map(|row| row.achievement_id)
        })
        .collect();
    // Installed70391: 5494 + native CastSpell29 unearned-self guard Dastardly Devices41707.
    // Structural union, not prerequisite acquisition or complete-content proof.
    assert_eq!(union.len(), 5495);
    for achievement in [
        141, 291, 1699, 3559, 3579, 4436, 4437, 5253, 6609, 6610, 9070, 11154, 11298, 11419, 11665,
        11666, 11667, 11668, 41707, 63640,
    ] {
        assert!(union.contains(&achievement));
    }
    for achievement in [
        578, 579, 603, 604, 624, 699, 1206, 1723, 1869, 1870, 1877, 2180, 2181, 2557, 5548, 5865,
        6030, 6031, 6350, 6584, 6621, 6973, 13025, 13026, 13623, 14728, 14729, 14730, 14731, 16729,
        17852, 18361, 18905, 40475, 40624,
        // Map alternatives, then area alternatives: all ten new sourced definitions.
        155, 16614, 16615, 16616, 40437, 62352, 2436, 2497, 11802, 61456,
        // Complete current-aura alternatives.
        8111, 9601, 9838, 13014,
        // Authored2v2/3v3 personal-rating thresholds; retired5v5 excluded.
        399, 400, 401, 402, 403, 405, 1159, 1160, 5266, 5267,
        // Fall24; Alive16 retained, and8354 additionally requires area6665.
        964, 8354,
        // Positive PlayerKnowsSpell104; actual learned recipes plus marker ownership.
        18805, 19704, 19716,
    ] {
        assert!(
            union.contains(&achievement),
            "source-qualified achievement {achievement}"
        );
    }
}

#[test]
fn achievements_dependencies_complete_from_historical_earned_ids_once() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    assert!(
        catalog
            .update(&mut state, CriteriaEvent::ReachLevel(1), None, 1)
            .is_empty()
    );
    // Retail Hero of Shattrath references these historical earned IDs.
    state.earned_ids.extend([4788, 4789]);
    let toasts = catalog.update(&mut state, CriteriaEvent::ReachLevel(1), None, 1);
    for id in [1205] {
        assert!(
            state.earned_ids.contains(&id),
            "missing dependency achievement {id}"
        );
        assert_eq!(
            toasts
                .iter()
                .filter(|toast| toast.achievement_id == id)
                .count(),
            1
        );
    }
    assert!(
        catalog
            .update(&mut state, CriteriaEvent::ReachLevel(1), None, 1)
            .is_empty()
    );
}

#[test]
fn achievements_quest_asset_and_repeat_credit_preserve_boolean_progress() {
    let catalog = catalog();
    println!(
        "trackable achievement snapshot rows: {}",
        catalog
            .snapshot(&PlayerAchievementState::default(), 1)
            .progress
            .len()
    );
    let mut state = PlayerAchievementState::default();
    catalog.update(&mut state, CriteriaEvent::CompleteQuest(7630), None, 1);
    assert!(!state.earned_ids.contains(&2357));
    let toasts = catalog.update(&mut state, CriteriaEvent::CompleteQuest(7631), None, 1);
    assert!(toasts.iter().any(|toast| toast.achievement_id == 2357));
    assert_eq!(state.criteria_progress[&8905], 1);
    assert!(
        catalog
            .update(&mut state, CriteriaEvent::CompleteQuest(7631), None, 1)
            .is_empty()
    );
    assert_eq!(state.criteria_progress[&8905], 1);
}

#[test]
fn dungeonach_real_level_rows_grant_at_thresholds() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    for (level, id) in [
        (10, 6),
        (20, 7),
        (40, 9),
        (60, 14783),
        (70, 15805),
        (80, 19459),
    ] {
        catalog.update(&mut state, CriteriaEvent::ReachLevel(level - 1), None, 1);
        assert!(!state.earned_ids.contains(&id));
        let toasts = catalog.update(&mut state, CriteriaEvent::ReachLevel(level), None, 1);
        assert!(toasts.iter().any(|toast| toast.achievement_id == id));
        assert!(state.earned_ids.contains(&id));
        assert!(
            catalog
                .update(&mut state, CriteriaEvent::ReachLevel(level), None, 1)
                .is_empty()
        );
    }
    assert_eq!(state.criteria_progress[&5212], 80);
    let snapshot = catalog.snapshot(&state, 1);
    let progress = snapshot
        .progress
        .iter()
        .find(|progress| progress.achievement_id == 19459)
        .unwrap();
    assert_eq!(
        (progress.current, progress.required, progress.completed),
        (80, 80, true)
    );
}

#[test]
fn dungeonach_real_encounter_criteria_match_only_their_asset() {
    let catalog = catalog();
    let criterion = &catalog.criteria[&39291];
    assert_eq!((criterion.kind, criterion.asset), (165, 1141));
    assert!(criterion.supported);
    let mut state = PlayerAchievementState::default();
    catalog.update(&mut state, CriteriaEvent::DefeatEncounter(1140), None, 1);
    assert!(!state.criteria_progress.contains_key(&39291));
    catalog.update(&mut state, CriteriaEvent::DefeatEncounter(1141), None, 1);
    assert_eq!(state.criteria_progress[&39291], 1);
}

#[test]
fn dungeonach_tree_all_any_and_count_operators() {
    let mut catalog = catalog();
    // Concrete Retail Hogger kill and reach-level leaves, reused under fixture parents.
    assert_eq!(catalog.trees[&3386].criteria_id, 18527);
    assert_eq!(catalog.trees[&2051].criteria_id, 5212);
    let leaves = vec![3386, 2051];
    for (id, operator, amount) in [
        (900001, TreeOperator::All, 0),
        (900002, TreeOperator::Any, 1),
        (900003, TreeOperator::CountDirectChildren, 2),
    ] {
        catalog.trees.insert(
            id,
            CriteriaTree {
                criteria_id: 0,
                amount,
                operator,
            },
        );
        catalog.children.insert(id, leaves.clone());
    }
    let mut state = PlayerAchievementState::default();
    assert!(!catalog.tree_progress(900002, &state).completed);
    catalog.update(&mut state, CriteriaEvent::KillCreature(46254), None, 1);
    assert!(catalog.tree_progress(900002, &state).completed);
    assert!(!catalog.tree_progress(900001, &state).completed);
    assert!(!catalog.tree_progress(900003, &state).completed);
    catalog.update(&mut state, CriteriaEvent::ReachLevel(1), None, 1);
    // Count counts direct children with positive progress, not completed level thresholds.
    assert!(catalog.tree_progress(900003, &state).completed);
    assert!(!catalog.tree_progress(900001, &state).completed);
    catalog.update(&mut state, CriteriaEvent::ReachLevel(10), None, 1);
    assert!(catalog.tree_progress(900001, &state).completed);
}

#[test]
fn dungeonach_placeholder_ids_migrate_once_in_redb() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    crate::persistence::create_tables(&db);
    let table = redb::TableDefinition::<u64, &[u8]>::new("achievements");
    let txn = db.begin_write().unwrap();
    {
        let mut table = txn.open_table(table).unwrap();
        table
            .insert(42, br#"{"earned_ids":[1,6]}"#.as_slice())
            .unwrap();
    }
    txn.commit().unwrap();
    let first = crate::persistence::load_achievements(&db, 42);
    assert_eq!(first.earned_ids, BTreeSet::from([6, 19459]));
    let second = crate::persistence::load_achievements(&db, 42);
    assert_eq!(first, second);
}

#[test]
fn dungeonach_criteria_progress_persists_across_database_reopen() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    catalog.update(&mut state, CriteriaEvent::ReachLevel(20), None, 1);
    catalog.update(&mut state, CriteriaEvent::KillCreature(46254), None, 1);
    catalog.update(&mut state, CriteriaEvent::CompleteQuest(7631), None, 1);
    assert!(state.earned_ids.contains(&2357));
    {
        let db = redb::Database::create(file.path()).unwrap();
        crate::persistence::create_tables(&db);
        crate::persistence::save_achievements(&db, 42, &state);
    }
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(crate::persistence::load_achievements(&db, 42), state);
    // Existing placeholder rows migrate once, retaining the actual earned levels.
    let (old, migrated) =
        crate::achievement::decode_saved_achievement_state(br#"{"earned_ids":[1,6]}"#);
    assert!(migrated);
    assert_eq!(old.earned_ids, BTreeSet::from([6, 19459]));
    assert!(old.criteria_progress.is_empty());
    let bytes = serde_json::to_vec(&old).unwrap();
    let (again, migrated) = crate::achievement::decode_saved_achievement_state(&bytes);
    assert!(!migrated);
    assert_eq!(again, old);
}

#[test]
fn achievements_criteria_pet_battle_points_sum_earned_category_achievements_to_a_fixed_point() {
    let catalog = catalog();
    let mut account = PlayerAchievementState::default();
    // Nineteen real 10-point Pet Battles achievements plus Master Pet Battler6462 (5):
    // 195 points; Level 10 (6, General) is outside the category.
    account.earned_ids.extend([
        15, 1017, 1248, 1250, 2516, 5875, 5876, 5877, 6560, 6570, 6582, 6583, 6590, 6592, 6599,
        6607, 6608, 6610, 6611, 6462, 6,
    ]);
    catalog.grant_completed(&mut account, None, 1, true);
    // Trainer Extraordinaire7482 (100) adds its own 5 points: 200 earns Battle Master7483,
    // whose 5 more make 205; Ultimate Trainer6600 (300) stays open.
    assert!(account.earned_ids.contains(&7482));
    assert!(account.earned_ids.contains(&7483));
    assert!(!account.earned_ids.contains(&6600));
    assert_eq!(account.criteria_progress.get(&21256), Some(&205));
    let mut fresh = PlayerAchievementState::default();
    fresh.earned_ids.insert(6);
    catalog.grant_completed(&mut fresh, None, 1, true);
    assert_eq!(fresh.criteria_progress.get(&21256), None);
}
