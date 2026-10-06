use super::*;
use shared::protocol::{AchievementCatalogPage, QueryAchievementCatalog};

fn catalog() -> AchievementCatalog {
    let db = Connection::open(crate::test_data::world_db_path()).unwrap();
    load_achievement_catalog(&db).unwrap()
}

#[test]
fn dungeon2_real_category_and_achievement_pages_have_continuations() {
    let catalog = catalog();
    let state = PlayerAchievementState::default();
    let AchievementCatalogPage::Categories {
        categories,
        next_id,
    } = catalog.page(&state, QueryAchievementCatalog::Categories { after_id: 0 })
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
    } = catalog.page(&state, request)
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
