//! An item's `proc.any_dps.default_value` from native item effects.
use server_combat::item_instance::effects::ItemEffect;
use server_combat::spell_cast::test_harness::world_db;
use server_follower_apl::literal_rotation::equipment::metadata::Metadata;

fn default_value(driver: u32) -> Result<f64, String> {
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
    Metadata::load(store, &[effect])?.proc_default_value()
}

/// Treemouth's Festering Splinter (395175) only absorbs: SimC's any_dps
/// expression has no stat buff to read, so it is 0 despite its set default.
#[test]
fn absorb_only_item_proc_default_value_is_zero() {
    assert_eq!(default_value(395175), Ok(0.0));
}

/// A stat-granting driver without a qualified SimC default stays an explicit gap.
#[test]
fn unqualified_stat_item_proc_default_value_is_explicit() {
    let error = default_value(383781).unwrap_err();
    assert!(
        error.contains("NATIVE_ITEM_PROC_DEFAULT_VALUE_UNSOURCED"),
        "{error}"
    );
}
