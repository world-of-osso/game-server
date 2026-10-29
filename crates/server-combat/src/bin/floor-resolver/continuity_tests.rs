use super::*;

#[test]
fn prior_resolutions_use_hint_run_and_reject_conflicting_floors() {
    let previous: Request = serde_json::from_str(r#"{"file":"old.log","run_start":"run1","guid":"unit1","map":2519,"x":-250,"y":-550,"ui_map_id":1}"#).unwrap();
    let current: Request = serde_json::from_str(r#"{"file":"new.log","run_start":"run2","guid":"unit2","map":2519,"x":-251,"y":-550,"ui_map_id":1}"#).unwrap();
    let hint: Hint = serde_json::from_str(
        r#"{"file":"old.log","run_start":"run1","guid":"unit1","x":-250,"y":-550}"#,
    )
    .unwrap();
    let mut known = HashMap::new();
    insert_known(&mut known, &previous, 133.0);
    assert_eq!(find_known_hint(&known, &current, &hint), Some(133.0));
    insert_known(&mut known, &previous, 58.0);
    assert_eq!(find_known_hint(&known, &current, &hint), None);
}
