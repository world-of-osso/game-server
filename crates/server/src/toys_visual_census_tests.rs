//! Approved server-only visual rule: source contract/complete chain/zero payload,
//! then real learning, aura application, owner replication and cancellation.
use super::*;
use crate::auras::ActiveAuras;
use shared::components::{Health, ModelDisplay, UnitAuras};

#[path = "toys_pennant_visual_tests.rs"]
mod pennant_visual;
#[path = "toys_seed_visual_tests.rs"]
mod seed_visual;

fn install_visual_source(h: &mut Harness) {
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
}

fn assert_cancellable_source_visual(item: u32, spell: u32, duration_ms: u32, recovery_ms: u32) {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("visual.redb"));
    install_visual_source(&mut h);
    let owner = h.caster;
    let other = h.npc;
    learn_effect_toy(&mut h, character, item);
    h.select(Some(other));
    h.advance(2);
    let health = *h.app.world().get::<Health>(owner).unwrap();
    let model = h.app.world().get::<ModelDisplay>(owner).copied();
    let position = *h.app.world().get::<Position>(owner).unwrap();
    assert_eq!(activate(&mut h, item)[0].error, None);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(owner)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == spell)
        .unwrap();
    let source_amount = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .unwrap()
        .effects[0]
        .base_points;
    assert_eq!(aura.effects[0].amount, source_amount);
    assert_eq!(aura.duration_ms, duration_ms);
    assert_eq!(aura.expires_at.is_none(), duration_ms == 0);
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(owner)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == spell && a.caster == Some(owner.to_bits()))
    );
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(other)
            .is_some_and(|auras| auras.auras.iter().any(|a| a.spell_id == spell))
    );
    if recovery_ms == 0 {
        assert!(
            h.inbox
                .cooldowns
                .iter()
                .all(|cd| cd.spell_id != spell || cd.duration_ms == 0)
        );
    } else {
        assert!(
            h.inbox
                .cooldowns
                .iter()
                .any(|cd| cd.spell_id == spell && cd.duration_ms == recovery_ms && !cd.is_gcd)
        );
    }
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura { spell_id: spell });
    h.advance(5);
    assert!(!crate::auras::has_aura(h.app.world(), owner, spell));
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(owner)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == spell)
    );
    assert_eq!(h.app.world().get::<Health>(owner), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(owner).copied(), model);
    assert_eq!(h.app.world().get::<Position>(owner), Some(&position));
    let previous_casts = h.inbox.go.iter().filter(|go| go.spell_id == spell).count();
    let reused = activate(&mut h, item);
    if recovery_ms == 0 {
        assert_eq!(reused[0].error, None);
        assert!(crate::auras::has_aura(h.app.world(), owner, spell));
        assert!(
            h.app
                .world()
                .get::<UnitAuras>(owner)
                .unwrap()
                .auras
                .iter()
                .any(|a| a.spell_id == spell)
        );
        assert_eq!(
            h.inbox.go.iter().filter(|go| go.spell_id == spell).count(),
            previous_casts + 1
        );
        assert!(
            h.inbox
                .cooldowns
                .iter()
                .all(|cd| cd.spell_id != spell || cd.duration_ms == 0)
        );
    } else {
        assert_eq!(reused[0].error.as_deref(), Some("OnCooldown"));
    }
}

#[test]
fn toy_coverage_visual_crystal_applies_replicates_and_cancels_source_twenty_second_aura() {
    assert_cancellable_source_visual(201927, 395705, 20_000, 120_000);
}

#[test]
fn toy_coverage_visual_compass_applies_replicates_and_cancels_source_permanent_aura() {
    assert_cancellable_source_visual(202711, 399041, 0, 1000);
}

#[test]
fn toy_coverage_visual_popcorn_source_ten_second_aura_replicates_and_cancels() {
    assert_cancellable_source_visual(162539, 202432, 10_000, 60_000);
}

#[test]
fn toy_coverage_visual_burning_blade_banner_source_ten_minute_aura_replicates_and_cancels() {
    assert_cancellable_source_visual(168014, 295037, 600_000, 3_600_000);
}

#[test]
fn toy_coverage_visual_yennu_kite_source_five_minute_aura_and_one_hour_recovery() {
    assert_cancellable_source_visual(202022, 396236, 300_000, 3_600_000);
}

#[test]
fn toy_coverage_visual_chowdar_ribbon_source_ten_minute_aura_and_zero_cooldown_reuse() {
    assert_cancellable_source_visual(242323, 1232024, 600_000, 0);
}

#[test]
fn toy_coverage_visual_kite_ribbon_census_retains_unsourced_dummy_gaps() {
    let report = report_toy_census(
        &crate::test_data::world_db_path(),
        "TOY_VISUAL_KITE_RIBBON_REPORT",
    );
    let rows = report["toys"].as_array().unwrap();
    for item in [202022, 242323] {
        let row = rows.iter().find(|r| r["item_id"] == item).unwrap();
        assert!(row["unavailable"].is_null(), "{row}");
    }
    for item in [18986, 210411] {
        let row = rows.iter().find(|r| r["item_id"] == item).unwrap();
        assert!(!row["unavailable"].is_null(), "{row}");
    }
}

#[test]
fn toy_coverage_visual_payload_or_absent_visual_refuses_without_effects() {
    for (item, spell) in [
        (201927, 395705),
        (202711, 399041),
        (162539, 202432),
        (168014, 295037),
        (202022, 396236),
        (242323, 1232024),
    ] {
        for absent_visual in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let (mut h, character) = setup(&dir.path().join("visual-refusal.redb"));
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
