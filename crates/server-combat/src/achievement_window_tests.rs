use super::*;
use shared::protocol::{AchievementCatalogPage, QueryAchievementCatalog};

#[test]
fn achievements_comparison_request_carries_an_authentic_target_and_bounded_catalog_cursor() {
    let wire = serde_json::json!({"Comparison": {"target_entity": 42, "query": {"Category": {"category_id": 92, "after_id": 633}}}});
    let request: QueryAchievementCatalog =
        serde_json::from_value(wire.clone()).expect("authoritative comparison request");
    assert_eq!(serde_json::to_value(request).unwrap(), wire);
}

#[test]
fn achievements_criteria_pages_normalize_zero_quantity_boolean_progress() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    state.account = Some(Box::new(PlayerAchievementState::default()));
    let rows = |state: &PlayerAchievementState| {
        let AchievementCatalogPage::Criteria { criteria, .. } = catalog.page(
            state,
            QueryAchievementCatalog::Criteria {
                achievement_id: 2076,
                after_id: 0,
            },
            1,
        ) else {
            panic!("bear criteria reply")
        };
        criteria
    };
    let before = rows(&state);
    assert_eq!(before.len(), 2);
    assert!(
        before
            .iter()
            .all(|row| row.current == 0 && row.required == 1 && !row.completed)
    );
    catalog.update(
        state.account.as_deref_mut().unwrap(),
        CriteriaEvent::LearnSpell(60114),
        None,
        1,
    );
    let after = rows(&state);
    // Authenticated Alliance60114 is criterion7369; Horde60116 is7368.
    let learned = after.iter().find(|row| row.criteria_id == 7369).unwrap();
    assert_eq!(
        (learned.current, learned.required, learned.completed),
        (1, 1, true)
    );
    let other = after.iter().find(|row| row.criteria_id == 7368).unwrap();
    assert_eq!(
        (other.current, other.required, other.completed),
        (0, 1, false)
    );
}

#[test]
fn achievements_criteria_pages_present_skill_step_in_authored_rank_units() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    let row = |state: &PlayerAchievementState| {
        let AchievementCatalogPage::Criteria { criteria, .. } = catalog.page(
            state,
            QueryAchievementCatalog::Criteria {
                achievement_id: 1199,
                after_id: 0,
            },
            1,
        ) else {
            panic!("professions criteria reply")
        };
        criteria
            .into_iter()
            .find(|row| row.criteria_id == 5561)
            .unwrap()
    };
    let before = row(&state);
    assert_eq!(
        (before.current, before.required, before.completed),
        (0, 75, false)
    );
    catalog.update(&mut state, CriteriaEvent::SkillMaxRank(197, 75), None, 1);
    let after = row(&state);
    assert_eq!(
        (after.current, after.required, after.completed),
        (75, 75, true)
    );
}

#[test]
fn achievements_summary_counts_full_catalog_and_account_scope_not_loaded_pages() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    state.earned_ids.extend([633, 39958]);
    // Authentic 0x100000 tracking copy must not add visible Summary completions.
    state.earned_at.insert(633, 1_791_244_800);
    let mut account = PlayerAchievementState::default();
    account.earned_ids.insert(4899);
    account.earned_at.insert(4899, 1_791_158_400);
    state.account = Some(Box::new(account));
    let request = serde_json::from_value(serde_json::json!({"Summary": {"scope": "All"}}))
        .expect("Summary is a supported catalog request");
    let page = serde_json::to_value(catalog.page(&state, request, 1)).unwrap();
    let summary = &page["Summary"];
    assert_eq!(summary["points"], 20);
    assert_eq!(summary["completed"], 2);
    assert!(summary["total"].as_u64().unwrap() > 8);
    assert_eq!(summary["recent"][0]["achievement_id"], 633);
    assert_eq!(summary["recent"][1]["achievement_id"], 4899);
    assert!(
        summary["categories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| { row["category_id"] == 14808 && row["completed"] == 1 })
    );
    let request =
        serde_json::from_value(serde_json::json!({"Summary": {"scope": "Account"}})).unwrap();
    let page = serde_json::to_value(catalog.page(&state, request, 1)).unwrap();
    assert_eq!(page["Summary"]["completed"], 1);
    assert_eq!(page["Summary"]["recent"][0]["achievement_id"], 4899);
}

#[test]
fn achievementlinks_detail_uses_authoritative_state_and_rejects_other_faction() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    state.earned_ids.insert(633);
    state.earned_at.insert(633, 1_791_244_800);
    let request =
        serde_json::from_value(serde_json::json!({"Achievement": {"achievement_id": 633}}))
            .expect("single achievement requests are supported");
    let page = serde_json::to_value(catalog.page(&state, request, 1)).unwrap();
    assert_eq!(page["Achievement"]["achievement_id"], 633);
    assert_eq!(page["Achievement"]["entry"]["earned"], true);
    assert_eq!(page["Achievement"]["entry"]["earned_at"], 1_791_244_800i64);
    let request =
        serde_json::from_value(serde_json::json!({"Achievement": {"achievement_id": 4899}}))
            .unwrap();
    let page = serde_json::to_value(catalog.page(&state, request, 2)).unwrap();
    assert!(page["Achievement"]["entry"].is_null());
}

#[test]
fn achievements_catalog_queries_preserve_a_valid_multibyte_search_term() {
    let catalog = catalog();
    let text = "É".repeat(200);
    let request = serde_json::from_value(
        serde_json::json!({"Search":{"text":text,"scope":"All","after_id":0}}),
    )
    .unwrap();
    let page =
        serde_json::to_value(catalog.page(&PlayerAchievementState::default(), request, 1)).unwrap();
    assert_eq!(page["Search"]["text"].as_str().unwrap(), text);
}

#[test]
fn achievements_catalog_queries_do_not_browse_hidden_tracking_copies() {
    let catalog = catalog();
    let page = catalog.page(
        &PlayerAchievementState::default(),
        QueryAchievementCatalog::Category {
            category_id: 14861,
            after_id: 39957,
        },
        1,
    );
    let AchievementCatalogPage::Category { achievements, .. } = page else {
        panic!("category response")
    };
    assert!(achievements.iter().all(|row| row.achievement_id != 39958));
}

#[test]
fn achievements_alert_payload_has_the_authored_earned_icon() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    let toasts = catalog.update(&mut state, CriteriaEvent::ReachLevel(50), None, 1);
    let toast = toasts
        .iter()
        .find(|row| row.achievement_id == 14782)
        .expect("authentic Level 50 grant");
    assert_eq!(serde_json::to_value(toast).unwrap()["icon_fdid"], 236566);
}

#[test]
fn achievements_browse_search_full_catalog_and_statistics_raw_counters() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    state.criteria_progress.insert(3239, 7); // Authentic Keli'dan kill criterion.
    let request = serde_json::from_value(
        serde_json::json!({"Statistics": {"category_id":14822,"after_id":1067}}),
    )
    .expect("Statistics request");
    let page = serde_json::to_value(catalog.page(&state, request, 1)).unwrap();
    assert_eq!(
        page["Statistics"]["achievements"][0]["achievement_id"],
        1068
    );
    assert_eq!(page["Statistics"]["achievements"][0]["statistic_value"], 7);
    let request = serde_json::from_value(
        serde_json::json!({"Search": {"text":"stormwind stockade","scope":"All","after_id":0}}),
    )
    .expect("Search request");
    let page = serde_json::to_value(catalog.page(&state, request, 1)).unwrap();
    assert!(
        page["Search"]["achievements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["achievement_id"] == 633)
    );
    let request = serde_json::from_value(
        serde_json::json!({"Search": {"text":"level","scope":"Character","after_id":0}}),
    )
    .unwrap();
    let page = serde_json::to_value(catalog.page(&state, request, 1)).unwrap();
    let after = page["Search"]["next_id"]
        .as_u64()
        .expect("more than one page") as u32;
    let request = serde_json::from_value(
        serde_json::json!({"Search": {"text":"level","scope":"Character","after_id":after}}),
    )
    .unwrap();
    let next = serde_json::to_value(catalog.page(&state, request, 1)).unwrap();
    assert!(
        !next["Search"]["achievements"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        next["Search"]["achievements"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["achievement_id"].as_u64().unwrap() > u64::from(after))
    );
}

fn catalog() -> AchievementCatalog {
    let db = Connection::open(crate::test_data::world_db_path()).unwrap();
    load_achievement_catalog(&db).unwrap()
}

#[test]
fn achievements_dependency_criterion_pages_read_the_actual_earned_ids() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    state.earned_ids.extend([4788, 4789]);
    let AchievementCatalogPage::Criteria { criteria, .. } = catalog.page(
        &state,
        QueryAchievementCatalog::Criteria {
            achievement_id: 1205,
            after_id: 0,
        },
        1,
    ) else {
        panic!("criteria page")
    };
    assert_eq!(criteria.len(), 2);
    assert!(criteria.iter().all(|row| row.progress_supported
        && row.completed
        && row.current == 1
        && row.required == 1));
}

#[test]
fn achievements_faction_filters_catalog_criteria_and_live_snapshots() {
    let catalog = catalog();
    let state = PlayerAchievementState::default();
    for (race, allowed) in [(1, true), (2, false), (24, false), (52, true), (84, false)] {
        let AchievementCatalogPage::Criteria {
            criteria, next_id, ..
        } = catalog.page(
            &state,
            QueryAchievementCatalog::Criteria {
                achievement_id: 4899,
                after_id: 0,
            },
            race,
        )
        else {
            panic!("criteria page")
        };
        assert_eq!(
            !criteria.is_empty(),
            allowed,
            "Loch criteria for race {race}"
        );
        if !allowed {
            assert!(next_id.is_none());
        }
        let AchievementCatalogPage::Category { achievements, .. } = catalog.page(
            &state,
            QueryAchievementCatalog::Category {
                category_id: 14861,
                after_id: 4898,
            },
            race,
        ) else {
            panic!("category page")
        };
        assert_eq!(
            achievements.iter().any(|row| row.achievement_id == 4899),
            allowed
        );
        let snapshot = catalog.snapshot(&state, race);
        assert!(
            !snapshot
                .progress
                .iter()
                .any(|row| row.achievement_id == 4899)
        );
        assert_eq!(
            snapshot
                .progress
                .iter()
                .any(|row| row.achievement_id == 39958),
            allowed
        );
        assert!(snapshot.progress.iter().any(|row| row.achievement_id == 6));
    }
    let alliance = catalog.snapshot(&state, 1);
    let horde = catalog.snapshot(&state, 2);
    let neutral = catalog.snapshot(&state, 24);
    let union: BTreeSet<_> = alliance
        .progress
        .iter()
        .chain(&horde.progress)
        .map(|row| row.achievement_id)
        .collect();
    println!(
        "trackable census: alliance={} horde={} neutral={} union={}",
        alliance.progress.len(),
        horde.progress.len(),
        neutral.progress.len(),
        union.len()
    );
}

#[test]
fn achievements_account_scoped_rows_never_expose_character_progress_as_account_progress() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    catalog.update(&mut state, CriteriaEvent::CompleteQuest(26145), None, 1);
    // Preserve historical rows in persistence, but do not present them as account state.
    state.earned_ids.insert(4899);
    state.earned_at.insert(4899, 1_790_000_000);
    let snapshot = catalog.snapshot(&state, 1);
    assert!(
        !snapshot
            .progress
            .iter()
            .any(|row| row.achievement_id == 4899)
    );
    assert!(!snapshot.earned_ids.contains(&4899));
    let AchievementCatalogPage::Category { achievements, .. } = catalog.page(
        &state,
        QueryAchievementCatalog::Category {
            category_id: 14861,
            after_id: 4898,
        },
        1,
    ) else {
        panic!("category page")
    };
    let loch = achievements
        .iter()
        .find(|row| row.achievement_id == 4899)
        .unwrap();
    assert!(!loch.progress_supported && !loch.earned && loch.earned_at.is_none());
    assert!(
        loch.criteria
            .iter()
            .all(|row| !row.progress_supported && !row.completed && row.current == 0)
    );
    let AchievementCatalogPage::Criteria { criteria, .. } = catalog.page(
        &state,
        QueryAchievementCatalog::Criteria {
            achievement_id: 39958,
            after_id: 0,
        },
        1,
    ) else {
        panic!("character criteria")
    };
    assert!(
        criteria
            .iter()
            .any(|row| row.criteria_id == 38400 && row.completed && row.current == 1)
    );
}

#[test]
fn dungeon2_real_category_and_achievement_pages_have_continuations() {
    let catalog = catalog();
    let state = PlayerAchievementState::default();
    let AchievementCatalogPage::Categories {
        categories,
        next_id,
    } = catalog.page(
        &state,
        QueryAchievementCatalog::Categories { after_id: 0 },
        1,
    )
    else {
        panic!("categories");
    };
    assert_eq!(
        categories.len(),
        shared::protocol::ACHIEVEMENT_CATEGORY_PAGE_SIZE
    );
    let AchievementCatalogPage::Categories {
        categories: second, ..
    } = catalog.page(
        &state,
        QueryAchievementCatalog::Categories {
            after_id: next_id.unwrap(),
        },
        1,
    )
    else {
        panic!("categories");
    };
    assert!(!second.is_empty());
    assert!(
        second
            .iter()
            .all(|row| row.category_id > categories.last().unwrap().category_id)
    );
    let AchievementCatalogPage::Category {
        achievements,
        next_id,
        ..
    } = catalog.page(
        &state,
        QueryAchievementCatalog::Category {
            category_id: 14808,
            after_id: 0,
        },
        1,
    )
    else {
        panic!("category");
    };
    assert_eq!(achievements.len(), shared::protocol::ACHIEVEMENT_PAGE_SIZE);
    let AchievementCatalogPage::Category {
        achievements: second,
        ..
    } = catalog.page(
        &state,
        QueryAchievementCatalog::Category {
            category_id: 14808,
            after_id: next_id.unwrap(),
        },
        1,
    )
    else {
        panic!("category");
    };
    assert!(!second.is_empty());
    assert!(
        second
            .iter()
            .all(|row| row.achievement_id > achievements.last().unwrap().achievement_id)
    );
    let stockade = achievements
        .iter()
        .chain(second.iter())
        .find(|a| a.achievement_id == 633)
        .unwrap();
    assert!(!stockade.earned);
    assert_eq!(stockade.earned_at, None);
    assert_eq!(
        (stockade.criteria[0].current, stockade.criteria[0].required),
        (0, 1)
    );
}

#[test]
fn dungeon2_criteria_paging_and_unknown_requests_are_bounded() {
    let catalog = catalog();
    let state = PlayerAchievementState::default();
    // Eastern Kingdoms Explorer has 25 real leaves, enough for two pages.
    let request = QueryAchievementCatalog::Criteria {
        achievement_id: 42,
        after_id: 0,
    };
    let AchievementCatalogPage::Criteria {
        criteria, next_id, ..
    } = catalog.page(&state, request, 1)
    else {
        panic!("criteria");
    };
    assert_eq!(
        criteria.len(),
        shared::protocol::ACHIEVEMENT_CRITERIA_PAGE_SIZE
    );
    let AchievementCatalogPage::Criteria {
        criteria: second, ..
    } = catalog.page(
        &state,
        QueryAchievementCatalog::Criteria {
            achievement_id: 42,
            after_id: next_id.unwrap(),
        },
        1,
    )
    else {
        panic!("criteria");
    };
    assert!(!second.is_empty());
    assert!(
        second
            .iter()
            .all(|row| row.tree_id > criteria.last().unwrap().tree_id)
    );
    let AchievementCatalogPage::Category {
        achievements,
        next_id,
        ..
    } = catalog.page(
        &state,
        QueryAchievementCatalog::Category {
            category_id: u32::MAX,
            after_id: 0,
        },
        1,
    )
    else {
        panic!("category");
    };
    assert!(achievements.is_empty());
    assert_eq!(next_id, None);
    let AchievementCatalogPage::Criteria {
        criteria, next_id, ..
    } = catalog.page(
        &state,
        QueryAchievementCatalog::Criteria {
            achievement_id: u32::MAX,
            after_id: u32::MAX,
        },
        1,
    )
    else {
        panic!("criteria");
    };
    assert!(criteria.is_empty());
    assert_eq!(next_id, None);
}

#[test]
fn dungeon2_utf8_caps_preserve_character_boundaries() {
    assert_eq!(bound_text("Aé😀Z", 4), "Aé");
    assert_eq!(bound_text("Aé😀Z", 0), "");
    assert_eq!(bound_text("Aé😀Z", 8), "Aé😀Z");
}

#[test]
fn dungeon2_legacy_earned_dates_are_unknown() {
    let catalog = catalog();
    let (state, _) = crate::achievement::decode_saved_achievement_state(br#"{"earned_ids":[633]}"#);
    let page = catalog.page(
        &state,
        QueryAchievementCatalog::Category {
            category_id: 14808,
            after_id: 632,
        },
        1,
    );
    let AchievementCatalogPage::Category { achievements, .. } = page else {
        panic!("category");
    };
    let stockade = achievements
        .iter()
        .find(|row| row.achievement_id == 633)
        .unwrap();
    assert!(stockade.earned);
    assert_eq!(stockade.earned_at, None);
}

#[test]
fn achievements_presentation_catalog_retains_authored_flags_without_changing_quantities() {
    let catalog = catalog();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let mut state = PlayerAchievementState::default();
    state.account = Some(Box::new(PlayerAchievementState::default()));
    for (achievement, tree, flags, required) in [(1017, 22151, 1, 1), (155, 4716, 0, 100)] {
        let source: (u32, u64) = conn
            .query_row(
                "SELECT Flags,Amount FROM criteria_tree WHERE ID=?1",
                [tree],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(source, (flags, required));
        let AchievementCatalogPage::Criteria { criteria, .. } = catalog.page(
            &state,
            QueryAchievementCatalog::Criteria {
                achievement_id: achievement,
                after_id: 0,
            },
            1,
        ) else {
            panic!("criteria page");
        };
        let line = criteria.iter().find(|line| line.tree_id == tree).unwrap();
        assert_eq!(
            (line.flags, line.current, line.required, line.completed),
            (flags, 0, required, false)
        );
    }
    let AchievementCatalogPage::Criteria { criteria, .. } = catalog.page(
        &state,
        QueryAchievementCatalog::Criteria {
            achievement_id: 155,
            after_id: 4716,
        },
        1,
    ) else {
        panic!("criteria continuation");
    };
    assert!(!criteria.iter().any(|line| line.tree_id == 4716));
    let continued = criteria.iter().find(|line| line.tree_id == 79890).unwrap();
    assert_eq!(
        (continued.flags, continued.current, continued.required),
        (0, 0, 100)
    );
}
