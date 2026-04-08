use super::*;

#[test]
fn apply_talent_choice_adds_selected_talent() {
    let mut selected = shared::class_spec::SelectedTalents::default();

    let changed = apply_talent_choice(&mut selected, 101);

    assert!(changed);
    assert_eq!(selected.talent_ids, vec![101]);
}

#[test]
fn applying_same_talent_twice_is_ignored() {
    let mut selected = shared::class_spec::SelectedTalents {
        talent_ids: vec![101],
    };

    let changed = apply_talent_choice(&mut selected, 101);

    assert!(!changed);
    assert_eq!(selected.talent_ids, vec![101]);
}

#[test]
fn reset_talents_clears_all_selected_ids() {
    let mut selected = shared::class_spec::SelectedTalents {
        talent_ids: vec![101, 102, 103],
    };

    let changed = reset_talents(&mut selected);

    assert!(changed);
    assert!(selected.talent_ids.is_empty());
}

#[test]
fn build_talent_snapshot_marks_selected_nodes_and_points_remaining() {
    let selected = shared::class_spec::SelectedTalents {
        talent_ids: vec![101, 104],
    };

    let snapshot = build_talent_snapshot(&selected);

    assert_eq!(snapshot.spec_tabs.len(), 3);
    assert_eq!(snapshot.talents.len(), TALENT_DEFS.len());
    assert_eq!(snapshot.points_remaining, 49);
    assert!(
        snapshot
            .talents
            .iter()
            .find(|talent| talent.talent_id == 101)
            .expect("talent 101")
            .active
    );
    assert!(
        !snapshot
            .talents
            .iter()
            .find(|talent| talent.talent_id == 102)
            .expect("talent 102")
            .active
    );
}
