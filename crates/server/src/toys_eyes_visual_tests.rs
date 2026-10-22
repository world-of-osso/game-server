//! Source318571 is only an owner attachment; its visual selectors do not add gameplay.
use super::*;

const EYES: u32 = 175140;
const EYES_SPELL: u32 = 318571;

#[test]
fn toy_coverage_eyes_visual_owner_aura_and_source_recovery_are_cancellable() {
    assert_cancellable_source_visual(EYES, EYES_SPELL, 600_000, 1_800_000);
}

#[test]
fn toy_coverage_eyes_visual_payload_or_absent_visual_refuses_without_effects() {
    for absent_visual in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&dir.path().join("eyes-drift.redb"));
        install_visual_source(&mut h);
        learn_effect_toy(&mut h, character, EYES);
        let mut changed = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(EYES_SPELL)
            .unwrap()
            .clone();
        if absent_visual {
            changed.has_spell_visual = false;
        } else {
            changed.effects[0].base_points = 1.0;
        }
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(vec![changed]));
        assert!(activate(&mut h, EYES)[0].error.is_some());
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, EYES_SPELL));
        assert!(!h.inbox.go.iter().any(|go| go.spell_id == EYES_SPELL));
        assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == EYES_SPELL));
    }
}

#[test]
fn toy_coverage_eyes_visual_census_retains_heating_orb_audience_and_dummy_gaps() {
    let report = report_toy_census(&crate::test_data::world_db_path(), "TOY_EYES_VISUAL_REPORT");
    let rows = report["toys"].as_array().unwrap();
    for item in [EYES, 129111, 210411, 202711] {
        let row = rows.iter().find(|row| row["item_id"] == item).unwrap();
        assert!(row["unavailable"].is_null(), "{row}");
    }
    for item in [180873, 136928, 86565, 18986] {
        let row = rows.iter().find(|row| row["item_id"] == item).unwrap();
        assert!(!row["unavailable"].is_null(), "{row}");
    }
}
