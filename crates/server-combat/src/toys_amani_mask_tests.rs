//! Source Amani mask forbids voluntary cancellation; natural expiry still removes it.
use super::*;

const MASK: u32 = 279052;
const MASK_SPELL: u32 = 1303320;
const DURATION_MS: u32 = 600_000;
const RECOVERY_MS: u32 = 900_000;

#[test]
fn toy_coverage_amani_mask_native_no_cancel_expiry_and_recovery_reuse() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("amani-mask.redb"));
    install_visual_source(&mut h);
    learn_effect_toy(&mut h, character, MASK);
    let owner = h.caster;
    h.select(Some(h.npc));
    h.advance(2);
    let health = *h.app.world().get::<Health>(owner).unwrap();
    let model = h.app.world().get::<ModelDisplay>(owner).copied();
    let position = *h.app.world().get::<Position>(owner).unwrap();
    assert_eq!(activate(&mut h, MASK)[0].error, None);
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(owner)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == MASK_SPELL)
        .unwrap();
    assert_eq!(aura.duration_ms, DURATION_MS);
    assert_eq!(aura.effects[0].amount, 0.0);
    assert_eq!(aura.caster, owner);
    assert!(aura.expires_at.is_some());
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(owner)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == MASK_SPELL && aura.caster == Some(owner.to_bits()))
    );
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, MASK_SPELL));
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|cooldown| cooldown.spell_id == MASK_SPELL
                && cooldown.duration_ms == RECOVERY_MS
                && !cooldown.is_gcd)
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura {
            spell_id: MASK_SPELL,
        });
    h.advance(5);
    assert!(crate::auras::has_aura(h.app.world(), owner, MASK_SPELL));
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(owner)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == MASK_SPELL)
    );
    // Harness ticks are50ms. Cross each authored deadline without editing aura/cooldown state.
    h.advance(11_900);
    assert!(crate::auras::has_aura(h.app.world(), owner, MASK_SPELL));
    h.advance(110);
    assert!(!crate::auras::has_aura(h.app.world(), owner, MASK_SPELL));
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(owner)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == MASK_SPELL)
    );
    assert_eq!(
        activate(&mut h, MASK)[0].error.as_deref(),
        Some("OnCooldown")
    );
    h.advance(6_000);
    assert_eq!(activate(&mut h, MASK)[0].error, None);
    assert!(crate::auras::has_aura(h.app.world(), owner, MASK_SPELL));
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|go| go.spell_id == MASK_SPELL)
            .count(),
        2
    );
    assert_eq!(h.app.world().get::<Health>(owner), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(owner).copied(), model);
    assert_eq!(h.app.world().get::<Position>(owner), Some(&position));
}

#[test]
fn toy_coverage_amani_mask_missing_visual_or_payload_drift_refuses() {
    for absent_visual in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&dir.path().join("amani-mask-drift.redb"));
        install_visual_source(&mut h);
        learn_effect_toy(&mut h, character, MASK);
        let mut changed = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(MASK_SPELL)
            .unwrap()
            .clone();
        if absent_visual {
            changed.has_spell_visual = false;
        } else {
            changed.effects[0].base_points = 1.0;
        }
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(vec![changed]));
        assert_eq!(
            activate(&mut h, MASK)[0].error.as_deref(),
            Some("unsupported toy spell: aura 4")
        );
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, MASK_SPELL));
        assert!(!h.inbox.go.iter().any(|go| go.spell_id == MASK_SPELL));
        assert!(
            !h.inbox
                .cooldowns
                .iter()
                .any(|cooldown| cooldown.spell_id == MASK_SPELL)
        );
    }
}

#[test]
fn toy_coverage_amani_mask_census_retains_heating_sound_audience_and_dummy_gaps() {
    let report = report_toy_census(&crate::test_data::world_db_path(), "TOY_AMANI_MASK_REPORT");
    let rows = report["toys"].as_array().unwrap();
    for item in [MASK, 136928, 228789, 235041] {
        let row = rows.iter().find(|row| row["item_id"] == item).unwrap();
        assert!(row["unavailable"].is_null(), "{row}");
    }
    for item in [180873, 205936, 86565, 18986] {
        let row = rows.iter().find(|row| row["item_id"] == item).unwrap();
        assert!(!row["unavailable"].is_null(), "{row}");
    }
}
