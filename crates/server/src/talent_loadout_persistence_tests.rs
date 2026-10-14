use super::*;
use crate::persistence::temp_database;

#[derive(bitcode::Encode)]
struct LegacyTraits {
    specs: Vec<(u32, Vec<Purchase>)>,
}
fn seed_legacy(db: &Database, specs: Vec<(u32, Vec<Purchase>)>) {
    let bytes = bitcode::encode(&LegacyTraits { specs });
    let txn = db.begin_write().unwrap();
    txn.open_table(CHARACTER_TRAITS)
        .unwrap()
        .insert(42, bytes.as_slice())
        .unwrap();
    crate::persistence_commit::commit(txn).unwrap();
}
#[test]
fn talent_loadouts_migration_preserves_both_specs_and_runs_once() {
    let db = temp_database();
    let ret = vec![Purchase {
        node_id: 81544,
        entry_id: 102514,
        ranks: 1,
    }];
    let holy = vec![Purchase {
        node_id: 81497,
        entry_id: 102465,
        ranks: 1,
    }];
    seed_legacy(&db, vec![(70, ret.clone()), (65, holy.clone())]);
    let migrated = load_trait_configs(&db, 42);
    assert_eq!(migrated.spec(70), ret);
    assert_eq!(migrated.spec(65), holy);
    // A migrated character must never read its obsolete allocation again.
    seed_legacy(&db, vec![(70, vec![])]);
    let reloaded = load_trait_configs(&db, 42);
    assert_eq!(reloaded.spec(70), ret);
    assert_eq!(reloaded.spec(65), holy);
    assert_eq!(
        reloaded
            .specs
            .iter()
            .find(|spec| spec.spec_id == 70)
            .unwrap()
            .selected_id,
        0
    );
}

#[test]
fn talent_loadouts_crud_persist_selected_allocations_and_isolate_specs() {
    let db = temp_database();
    let raid = vec![Purchase {
        node_id: 81544,
        entry_id: 102514,
        ranks: 1,
    }];
    let mut data = load_trait_configs(&db, 42);
    let raid_id = data.create(70, "Raid".into(), raid.clone()).unwrap();
    let dungeon_id = data.create(70, "Dungeons".into(), vec![]).unwrap();
    let holy_id = data.create(65, "Healing".into(), vec![]).unwrap();
    assert_ne!(raid_id, holy_id);
    data.activate(70, raid_id).unwrap();
    data.rename(70, raid_id, "Single Target".into()).unwrap();
    assert!(data.activate(70, holy_id).is_err());
    assert!(data.rename(70, raid_id, " ".into()).is_err());
    save_trait_configs(&db, 42, &data);
    let mut reloaded = load_trait_configs(&db, 42);
    assert_eq!(reloaded, data);
    assert_eq!(reloaded.spec(70), raid);
    reloaded.delete(70, dungeon_id).unwrap();
    reloaded.delete(70, raid_id).unwrap();
    assert_eq!(reloaded.spec(70), raid); // deleting a saved copy retains current talents
    assert_eq!(reloaded.spec_mut(70).selected_id, 0);
    save_trait_configs(&db, 42, &reloaded);
    assert_eq!(load_trait_configs(&db, 42), reloaded);
    let next_id = reloaded.create(70, "New".into(), vec![]).unwrap();
    assert!(next_id > holy_id); // deleting never reuses an ID
}
