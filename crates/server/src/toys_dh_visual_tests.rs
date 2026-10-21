//! Exact70448 DH-only skull/crystal: source30s recovery and permanent cosmetic aura.
use super::*;

const DH_VISUALS: &[(u32, u32)] = &[(143544, 219708), (143545, 219871)];

#[test]
fn toy_coverage_dh_skull_source_owner_aura_replicates_and_cancels() {
    assert_cancellable_source_visual_on(143544, 219708, 0, 30_000, (4, 12));
}

#[test]
fn toy_coverage_dh_fel_crystal_source_owner_aura_replicates_and_cancels() {
    assert_cancellable_source_visual_on(143545, 219871, 0, 30_000, (4, 12));
}

#[test]
fn toy_coverage_dh_visual_wrong_class_refuses_without_effect_or_cooldown() {
    for &(item, spell) in DH_VISUALS {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) = setup_for_race_class(&dir.path().join("mage-visual.redb"), 4, 12);
        install_visual_source(&mut h);
        learn_effect_toy(&mut h, character, item);
        // Account ownership must not bypass another character class's item restriction.
        h.app
            .world_mut()
            .get_mut::<shared::components::Player>(h.caster)
            .unwrap()
            .class = 8;
        assert_eq!(
            activate(&mut h, item)[0].error.as_deref(),
            Some("toy race/class restriction")
        );
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, spell));
        assert!(!h.inbox.go.iter().any(|go| go.spell_id == spell));
        assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == spell));
    }
}

#[test]
fn toy_coverage_dh_visual_payload_or_absent_visual_refuses_eligible_character() {
    for &(item, spell) in DH_VISUALS {
        for absent_visual in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let (mut h, character) = setup_for_race_class(&dir.path().join("dh-drift.redb"), 4, 12);
            install_visual_source(&mut h);
            learn_effect_toy(&mut h, character, item);
            let mut changed = h
                .app
                .world()
                .resource::<crate::spell_info::SpellStore>()
                .get(spell)
                .unwrap()
                .clone();
            if absent_visual {
                changed.has_spell_visual = false;
            } else {
                changed.effects[0].base_points = 1.0;
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
fn toy_coverage_dh_visual_census_retains_unsourced_siblings() {
    let report = report_toy_census(&crate::test_data::world_db_path(), "TOY_DH_VISUAL_REPORT");
    let rows = report["toys"].as_array().unwrap();
    for &(item, _) in DH_VISUALS {
        let row = rows.iter().find(|r| r["item_id"] == item).unwrap();
        assert!(row["unavailable"].is_null(), "{row}");
    }
    for item in [180873, 228789, 235041, 18986] {
        let row = rows.iter().find(|r| r["item_id"] == item).unwrap();
        assert!(!row["unavailable"].is_null(), "{row}");
    }
}
