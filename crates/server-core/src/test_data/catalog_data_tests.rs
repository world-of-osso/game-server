//! Mutable App copies retain their ordinary collection behavior and serialized contract.
#[path = "../catalog_data.rs"]
mod catalog_data;

use catalog_data::CatalogData;
use std::collections::HashMap;

#[test]
fn nested_catalog_edits_leave_retained_input_and_sibling_unchanged() {
    let original: CatalogData<HashMap<u32, Vec<String>>> =
        HashMap::from([(133, vec!["Fireball".into(), "Rank 1".into()])]).into();
    let sibling = original.clone();
    let mut app = original.clone();
    app.get_mut(&133).unwrap().push("local annotation".into());
    app.insert(585, vec!["Smite".into()]);
    assert_eq!(original[&133], ["Fireball", "Rank 1"]);
    assert_eq!(sibling[&133], ["Fireball", "Rank 1"]);
    assert!(!original.contains_key(&585));
    assert!(!sibling.contains_key(&585));
    assert_eq!(app[&133].len(), 3);
}

#[test]
fn owned_and_mutable_iteration_do_not_modify_another_app() {
    let retained: CatalogData<Vec<String>> = vec!["Fireball".into(), "Smite".into()].into();
    let mut app = retained.clone();
    for name in &mut app {
        name.push_str(" local");
    }
    let mut owned: Vec<_> = retained.clone().into_iter().collect();
    owned[0] = "owned elsewhere".into();
    assert_eq!(
        retained.iter().map(String::as_str).collect::<Vec<_>>(),
        ["Fireball", "Smite"]
    );
    assert_eq!(app[0], "Fireball local");
    assert_eq!(owned[1], "Smite");
}

#[test]
fn shared_definition_storage_preserves_serialized_catalog_values() {
    let input: CatalogData<Vec<(u32, String)>> = [(133, "Fireball".into()), (585, "Smite".into())]
        .into_iter()
        .collect();
    assert_eq!(
        serde_json::to_value(&input).unwrap(),
        serde_json::json!([[133, "Fireball"], [585, "Smite"]])
    );
}
