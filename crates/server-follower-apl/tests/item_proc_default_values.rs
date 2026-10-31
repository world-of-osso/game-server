//! An item's `proc.any_dps.default_value` from native item effects.
use server_combat::item_instance::effects::ItemEffect;
use server_combat::spell_cast::test_harness::world_db;
use server_follower_apl::literal_rotation::equipment::metadata::Metadata;

fn metadata(driver: u32) -> Metadata {
    let store = world_db().unwrap().spell_store();
    // Native ItemEffect rows: on-use (trigger0) drivers of journal trinkets.
    let effect = ItemEffect {
        id: 0,
        spell_id: driver,
        trigger: 0,
        cooldown_ms: -1,
        category_cooldown_ms: -1,
        category: 0,
    };
    Metadata::load(store, &[effect]).unwrap()
}

/// Treemouth's Festering Splinter (395175) only absorbs: SimC's any_dps
/// expression has no stat buff to read, so it is 0 despite its set default.
#[test]
fn absorb_only_item_proc_default_value_is_zero() {
    assert_eq!(metadata(395175).proc_default_value(), Ok(0.0));
}

/// Follower Journal stat procs whose SimC callbacks never set a default read
/// the unset value, 0: Ara-Kara Sacbrood, Improvised Seaforium Pacemaker's
/// extension driver, Idol of the War Loa, Emberwing Feather.
#[test]
fn stat_item_proc_default_value_without_a_simc_default_is_zero() {
    for driver in [443541, 1218712, 1250567, 1250508] {
        let metadata = metadata(driver);
        assert!(
            !metadata.stats.is_empty(),
            "driver{driver} grants no any_dps stat"
        );
        assert_eq!(metadata.proc_default_value(), Ok(0.0), "driver{driver}");
    }
}

/// Blazebinder's Hoof (383926) sets its stat buff's default in SimC; native
/// does not compute it, so it stays an explicit gap.
#[test]
fn stat_item_proc_default_value_set_by_simc_is_explicit() {
    let metadata = metadata(383926);
    assert!(!metadata.stats.is_empty());
    let error = metadata.proc_default_value().unwrap_err();
    assert!(
        error.contains("NATIVE_ITEM_PROC_DEFAULT_VALUE_UNSOURCED"),
        "{error}"
    );
}
