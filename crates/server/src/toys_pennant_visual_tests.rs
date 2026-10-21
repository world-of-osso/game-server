//! Exact70448 seasonal pennants carry amount1, not generic zero-payload dummy spells.
use super::*;

const PENNANTS: &[(u32, u32)] = &[
    (206267, 412530),
    (206343, 412912),
    (210497, 425994),
    (211869, 430508),
    (225933, 456798),
    (227538, 460082),
    (227539, 460083),
    (235519, 1217166),
    (242636, 1233142),
];

#[test]
fn toy_coverage_pennant_obsidian_owner_aura_replicates_and_cancels() {
    assert_cancellable_source_visual(206267, 412530, 0, 120_000);
}
#[test]
fn toy_coverage_pennant_crimson_owner_aura_replicates_and_cancels() {
    assert_cancellable_source_visual(206343, 412912, 0, 120_000);
}
#[test]
fn toy_coverage_pennant_verdant_owner_aura_replicates_and_cancels() {
    assert_cancellable_source_visual(210497, 425994, 0, 120_000);
}
#[test]
fn toy_coverage_pennant_draconic_owner_aura_replicates_and_cancels() {
    assert_cancellable_source_visual(211869, 430508, 0, 120_000);
}
#[test]
fn toy_coverage_pennant_forged_owner_aura_replicates_and_cancels() {
    assert_cancellable_source_visual(225933, 456798, 0, 120_000);
}
#[test]
fn toy_coverage_pennant_unbound_legend_owner_aura_replicates_and_cancels() {
    assert_cancellable_source_visual(227538, 460082, 0, 120_000);
}
#[test]
fn toy_coverage_pennant_unbound_strategist_owner_aura_replicates_and_cancels() {
    assert_cancellable_source_visual(227539, 460083, 0, 120_000);
}
#[test]
fn toy_coverage_pennant_prized_owner_aura_replicates_and_cancels() {
    assert_cancellable_source_visual(235519, 1217166, 0, 120_000);
}
#[test]
fn toy_coverage_pennant_astral_owner_aura_replicates_and_cancels() {
    assert_cancellable_source_visual(242636, 1233142, 0, 120_000);
}

#[test]
fn toy_coverage_pennant_changed_amount_or_missing_visual_refuses_each_identity() {
    for &(item, spell) in PENNANTS {
        for absent_visual in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let (mut h, character) = setup(&dir.path().join("pennant-drift.redb"));
            install_visual_source(&mut h);
            learn_effect_toy(&mut h, character, item);
            let mut changed = h
                .app
                .world()
                .resource::<crate::spell_info::SpellStore>()
                .get(spell)
                .unwrap()
                .clone();
            assert_eq!(changed.effects[0].base_points, 1.0);
            if absent_visual {
                changed.has_spell_visual = false;
            } else {
                changed.effects[0].base_points = 2.0;
            }
            h.app
                .insert_resource(crate::spell_info::SpellStore::fixture(vec![changed]));
            assert!(activate(&mut h, item)[0].error.is_some());
            assert!(!crate::auras::has_aura(h.app.world(), h.caster, spell));
            assert!(!h.inbox.go.iter().any(|go| go.spell_id == spell));
            assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == spell));
        }
    }
}

#[test]
fn toy_coverage_pennant_census_retains_unrelated_dummy_and_zero_payload_gaps() {
    let report = report_toy_census(
        &crate::test_data::world_db_path(),
        "TOY_PENNANT_VISUAL_REPORT",
    );
    let rows = report["toys"].as_array().unwrap();
    for &(item, _) in PENNANTS {
        let row = rows.iter().find(|r| r["item_id"] == item).unwrap();
        assert!(row["unavailable"].is_null(), "{row}");
    }
    for item in [18986, 208092, 190926] {
        let row = rows.iter().find(|r| r["item_id"] == item).unwrap();
        assert!(!row["unavailable"].is_null(), "{row}");
    }
}
