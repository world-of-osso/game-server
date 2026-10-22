//! Source ring attachments; customization selectors change native appearance, not gameplay.
use super::*;

const COLDFLAME: u32 = 228789;
const COLDFLAME_SPELL: u32 = 465642;
const CYRCE: u32 = 235041;
const CYRCE_SPELL: u32 = 1215495;

#[test]
fn toy_coverage_rings_coldflame_owner_aura_cancels_and_retains_source_recovery() {
    assert_cancellable_source_visual(COLDFLAME, COLDFLAME_SPELL, 0, 120_000);
}

#[test]
fn toy_coverage_rings_cyrce_owner_aura_cancels_and_reuses_true_zero_recovery() {
    assert_cancellable_source_visual(CYRCE, CYRCE_SPELL, 0, 0);
}

#[test]
fn toy_coverage_rings_census_retains_heating_orb_audience_and_dummy_gaps() {
    let report = report_toy_census(
        &crate::test_data::world_db_path(),
        "TOY_RINGS_VISUAL_REPORT",
    );
    let rows = report["toys"].as_array().unwrap();
    for item in [COLDFLAME, CYRCE, 175140, 129111, 242323] {
        let row = rows.iter().find(|row| row["item_id"] == item).unwrap();
        assert!(row["unavailable"].is_null(), "{row}");
    }
    for item in [180873, 136928, 86565, 18986] {
        let row = rows.iter().find(|row| row["item_id"] == item).unwrap();
        assert!(!row["unavailable"].is_null(), "{row}");
    }
}
