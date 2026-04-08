use super::*;

fn sample_achievement() -> AchievementData {
    AchievementData {
        id: 46,
        name: "Level 80".into(),
        description: "Reach level 80.".into(),
        criteria: vec![AchievementCriteria::ReachLevel { level: 80 }],
        points: 10,
        reward: None,
        account_wide: false,
        feat_of_strength: false,
    }
}

#[test]
fn achievement_construction() {
    let a = sample_achievement();
    assert_eq!(a.id, 46);
    assert_eq!(a.points, 10);
    assert_eq!(a.criteria.len(), 1);
    assert!(!a.feat_of_strength);
}

#[test]
fn achievement_with_reward() {
    let a = AchievementData {
        id: 614,
        name: "For the Alliance!".into(),
        description: "Slay the leaders of the Horde.".into(),
        criteria: vec![
            AchievementCriteria::Kill {
                creature_id: 1,
                count: 1,
            },
            AchievementCriteria::Kill {
                creature_id: 2,
                count: 1,
            },
            AchievementCriteria::Kill {
                creature_id: 3,
                count: 1,
            },
            AchievementCriteria::Kill {
                creature_id: 4,
                count: 1,
            },
        ],
        points: 20,
        reward: Some(AchievementReward::Title("of the Alliance".into())),
        account_wide: false,
        feat_of_strength: false,
    };
    assert_eq!(a.criteria.len(), 4);
    assert!(a.reward.is_some());
}

#[test]
fn feat_of_strength() {
    let a = AchievementData {
        id: 2336,
        name: "Realm First! Level 80".into(),
        description: "First player to reach level 80.".into(),
        criteria: vec![AchievementCriteria::ReachLevel { level: 80 }],
        points: 0,
        reward: Some(AchievementReward::Title("the Supreme".into())),
        account_wide: false,
        feat_of_strength: true,
    };
    assert!(a.feat_of_strength);
    assert_eq!(a.points, 0);
}

#[test]
fn criteria_all_variants() {
    let criteria: [AchievementCriteria; 5] = [
        AchievementCriteria::Kill {
            creature_id: 1,
            count: 10,
        },
        AchievementCriteria::CompleteQuest { quest_id: 100 },
        AchievementCriteria::ReachLevel { level: 80 },
        AchievementCriteria::CollectItem {
            item_id: 50,
            count: 100,
        },
        AchievementCriteria::VisitArea { area_id: 1 },
    ];
    assert_eq!(criteria.len(), 5);
}

#[test]
fn reward_all_variants() {
    let rewards: [AchievementReward; 3] = [
        AchievementReward::Title("Champion".into()),
        AchievementReward::Item {
            item_id: 100,
            count: 1,
        },
        AchievementReward::Spell { spell_id: 200 },
    ];
    assert_eq!(rewards.len(), 3);
}

#[test]
fn achievement_serialization_round_trip() {
    let a = sample_achievement();
    let json = serde_json::to_string(&a).unwrap();
    let decoded: AchievementData = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, a);
}

#[test]
fn kill_criteria_tracks_count() {
    let a = AchievementData {
        id: 1,
        name: "Kill 5 Wolves".into(),
        description: "".into(),
        criteria: vec![AchievementCriteria::Kill {
            creature_id: 10,
            count: 5,
        }],
        points: 10,
        reward: None,
        account_wide: false,
        feat_of_strength: false,
    };
    let mut progress = AchievementProgress::new(&a);
    assert!(!progress.completed);

    progress.record_kill(10, &a);
    progress.record_kill(10, &a);
    progress.record_kill(10, &a);
    assert!(!progress.update_completion());

    progress.record_kill(10, &a);
    progress.record_kill(10, &a);
    assert!(progress.update_completion());
    assert!(progress.completed);
}

#[test]
fn kill_wrong_creature_ignored() {
    let a = AchievementData {
        id: 1,
        name: "".into(),
        description: "".into(),
        criteria: vec![AchievementCriteria::Kill {
            creature_id: 10,
            count: 1,
        }],
        points: 5,
        reward: None,
        account_wide: false,
        feat_of_strength: false,
    };
    let mut progress = AchievementProgress::new(&a);
    progress.record_kill(99, &a);
    assert!(!progress.update_completion());
}

#[test]
fn level_criteria() {
    let a = sample_achievement();
    let mut progress = AchievementProgress::new(&a);
    progress.record_level(79, &a);
    assert!(!progress.update_completion());
    progress.record_level(80, &a);
    assert!(progress.update_completion());
}

#[test]
fn quest_criteria() {
    let a = AchievementData {
        id: 1,
        name: "".into(),
        description: "".into(),
        criteria: vec![AchievementCriteria::CompleteQuest { quest_id: 100 }],
        points: 5,
        reward: None,
        account_wide: false,
        feat_of_strength: false,
    };
    let mut progress = AchievementProgress::new(&a);
    progress.record_quest(100, &a);
    assert!(progress.update_completion());
}

#[test]
fn visit_criteria() {
    let a = AchievementData {
        id: 1,
        name: "".into(),
        description: "".into(),
        criteria: vec![AchievementCriteria::VisitArea { area_id: 42 }],
        points: 5,
        reward: None,
        account_wide: false,
        feat_of_strength: false,
    };
    let mut progress = AchievementProgress::new(&a);
    progress.record_visit(42, &a);
    assert!(progress.update_completion());
}

#[test]
fn collect_criteria() {
    let a = AchievementData {
        id: 1,
        name: "".into(),
        description: "".into(),
        criteria: vec![AchievementCriteria::CollectItem {
            item_id: 50,
            count: 10,
        }],
        points: 5,
        reward: None,
        account_wide: false,
        feat_of_strength: false,
    };
    let mut progress = AchievementProgress::new(&a);
    progress.record_collect(50, 7, &a);
    assert!(!progress.update_completion());
    progress.record_collect(50, 5, &a);
    assert!(progress.update_completion());
}

#[test]
fn multi_criteria_all_required() {
    let a = AchievementData {
        id: 614,
        name: "For the Alliance!".into(),
        description: "".into(),
        criteria: vec![
            AchievementCriteria::Kill {
                creature_id: 1,
                count: 1,
            },
            AchievementCriteria::Kill {
                creature_id: 2,
                count: 1,
            },
        ],
        points: 20,
        reward: None,
        account_wide: false,
        feat_of_strength: false,
    };
    let mut progress = AchievementProgress::new(&a);
    progress.record_kill(1, &a);
    assert!(!progress.update_completion());
    progress.record_kill(2, &a);
    assert!(progress.update_completion());
}

#[test]
fn already_completed_no_double() {
    let a = sample_achievement();
    let mut progress = AchievementProgress::new(&a);
    progress.record_level(80, &a);
    assert!(progress.update_completion());
    assert!(!progress.update_completion());
}

fn account_wide_achievement() -> AchievementData {
    AchievementData {
        id: 2000,
        name: "Account Level 80".into(),
        description: "Any character reaches 80.".into(),
        criteria: vec![AchievementCriteria::ReachLevel { level: 80 }],
        points: 10,
        reward: None,
        account_wide: true,
        feat_of_strength: false,
    }
}

#[test]
fn character_achievement_tracks_points() {
    let a = sample_achievement();
    let mut char_ach = CharacterAchievements::default();
    let mut acct = AccountAchievements::default();

    let progress = char_ach.get_or_create(&a);
    progress.record_level(80, &a);
    assert!(try_complete(&a, &mut char_ach, &mut acct));
    assert_eq!(char_ach.total_points, 10);
    assert!(char_ach.is_completed(a.id));
}

#[test]
fn account_wide_shared() {
    let a = account_wide_achievement();
    let mut char1 = CharacterAchievements::default();
    let mut acct = AccountAchievements::default();

    let progress = char1.get_or_create(&a);
    progress.record_level(80, &a);
    assert!(try_complete(&a, &mut char1, &mut acct));
    assert!(acct.is_completed(a.id));
    assert_eq!(acct.total_points, 10);

    let mut char2 = CharacterAchievements::default();
    let progress2 = char2.get_or_create(&a);
    progress2.record_level(80, &a);
    assert!(!try_complete(&a, &mut char2, &mut acct));
}

#[test]
fn per_character_not_shared() {
    let a = sample_achievement();
    let mut char1 = CharacterAchievements::default();
    let mut char2 = CharacterAchievements::default();
    let mut acct = AccountAchievements::default();

    let p1 = char1.get_or_create(&a);
    p1.record_level(80, &a);
    try_complete(&a, &mut char1, &mut acct);

    let p2 = char2.get_or_create(&a);
    p2.record_level(80, &a);
    assert!(try_complete(&a, &mut char2, &mut acct));
}

#[test]
fn feat_of_strength_no_points() {
    let a = AchievementData {
        id: 99,
        name: "Feat".into(),
        description: "".into(),
        criteria: vec![AchievementCriteria::ReachLevel { level: 1 }],
        points: 0,
        reward: None,
        account_wide: false,
        feat_of_strength: true,
    };
    let mut char_ach = CharacterAchievements::default();
    let mut acct = AccountAchievements::default();
    let p = char_ach.get_or_create(&a);
    p.record_level(1, &a);
    try_complete(&a, &mut char_ach, &mut acct);
    assert_eq!(char_ach.total_points, 0);
}

#[test]
fn incomplete_does_not_complete() {
    let a = sample_achievement();
    let mut char_ach = CharacterAchievements::default();
    let mut acct = AccountAchievements::default();
    let p = char_ach.get_or_create(&a);
    p.record_level(50, &a);
    assert!(!try_complete(&a, &mut char_ach, &mut acct));
    assert!(!char_ach.is_completed(a.id));
}

#[test]
fn combined_points_sums_both_scopes() {
    let char_only = sample_achievement();
    let acct_wide = account_wide_achievement();

    let mut char_ach = CharacterAchievements::default();
    let mut acct = AccountAchievements::default();

    let p = char_ach.get_or_create(&char_only);
    p.record_level(80, &char_only);
    try_complete(&char_only, &mut char_ach, &mut acct);

    let p = char_ach.get_or_create(&acct_wide);
    p.record_level(80, &acct_wide);
    try_complete(&acct_wide, &mut char_ach, &mut acct);

    assert_eq!(char_ach.total_points, 10);
    assert_eq!(acct.total_points, 10);
    assert_eq!(combined_points(&char_ach, &acct), 20);
}

#[test]
fn completed_count_both_scopes() {
    let char_only = sample_achievement();
    let acct_wide = account_wide_achievement();

    let mut char_ach = CharacterAchievements::default();
    let mut acct = AccountAchievements::default();

    let p = char_ach.get_or_create(&char_only);
    p.record_level(80, &char_only);
    try_complete(&char_only, &mut char_ach, &mut acct);

    let p = char_ach.get_or_create(&acct_wide);
    p.record_level(80, &acct_wide);
    try_complete(&acct_wide, &mut char_ach, &mut acct);

    assert_eq!(completed_count(&char_ach, &acct), 3);
}

#[test]
fn zero_points_when_empty() {
    let char_ach = CharacterAchievements::default();
    let acct = AccountAchievements::default();
    assert_eq!(combined_points(&char_ach, &acct), 0);
    assert_eq!(completed_count(&char_ach, &acct), 0);
}

fn feat_achievement() -> AchievementData {
    AchievementData {
        id: 2336,
        name: "Realm First! Level 80".into(),
        description: "First to reach 80.".into(),
        criteria: vec![AchievementCriteria::ReachLevel { level: 80 }],
        points: 0,
        reward: Some(AchievementReward::Title("the Supreme".into())),
        account_wide: false,
        feat_of_strength: true,
    }
}

#[test]
fn feat_completes_with_zero_points() {
    let feat = feat_achievement();
    let normal = sample_achievement();
    let mut char_ach = CharacterAchievements::default();
    let mut acct = AccountAchievements::default();

    let p = char_ach.get_or_create(&normal);
    p.record_level(80, &normal);
    try_complete(&normal, &mut char_ach, &mut acct);

    let p = char_ach.get_or_create(&feat);
    p.record_level(80, &feat);
    try_complete(&feat, &mut char_ach, &mut acct);

    assert_eq!(char_ach.total_points, 10);
    assert!(char_ach.is_completed(feat.id));
}

#[test]
fn feats_listed_separately() {
    let feat = feat_achievement();
    let normal = sample_achievement();
    let all = vec![normal.clone(), feat.clone()];

    let mut char_ach = CharacterAchievements::default();
    let mut acct = AccountAchievements::default();

    let p = char_ach.get_or_create(&normal);
    p.record_level(80, &normal);
    try_complete(&normal, &mut char_ach, &mut acct);

    let p = char_ach.get_or_create(&feat);
    p.record_level(80, &feat);
    try_complete(&feat, &mut char_ach, &mut acct);

    let feats = char_ach.completed_feats(&all);
    let normals = char_ach.completed_normal(&all);
    assert_eq!(feats, vec![2336]);
    assert_eq!(normals, vec![46]);
}

#[test]
fn no_feats_when_none_completed() {
    let char_ach = CharacterAchievements::default();
    assert!(char_ach.completed_feats(&[]).is_empty());
    assert!(char_ach.completed_normal(&[]).is_empty());
}
