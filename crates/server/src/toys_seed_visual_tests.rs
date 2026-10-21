//! Exact70448 Seed contract: peaceful3s cast, ordinary growth visual,30s aura/60s recovery.
use super::*;

const SEED: u32 = 210411;
const SEED_SPELL: u32 = 425503;

#[test]
fn toy_coverage_seed_visual_cast_completes_then_expires_and_recovers_naturally() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("seed-lifecycle.redb"));
    install_visual_source(&mut h);
    learn_effect_toy(&mut h, character, SEED);
    let owner = h.caster;
    let health = *h.app.world().get::<Health>(owner).unwrap();
    let model = h.app.world().get::<ModelDisplay>(owner).copied();
    let position = *h.app.world().get::<Position>(owner).unwrap();
    assert_eq!(activate(&mut h, SEED)[0].error, None);
    h.advance(30); // Pump350ms +1500ms remains below source3000ms cast time.
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(owner)
            .is_some()
    );
    assert!(!crate::auras::has_aura(h.app.world(), owner, SEED_SPELL));
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == SEED_SPELL));
    h.advance(40);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(owner)
            .is_none()
    );
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(owner)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == SEED_SPELL)
        .unwrap();
    assert_eq!(aura.duration_ms, 30_000);
    assert!(aura.expires_at.is_some());
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(owner)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == SEED_SPELL && a.caster == Some(owner.to_bits()))
    );
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, SEED_SPELL));
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == SEED_SPELL && cd.duration_ms == 60_000 && !cd.is_gcd)
    );
    h.advance(540); // Keep the real30s aura until just before its expiry.
    assert!(crate::auras::has_aura(h.app.world(), owner, SEED_SPELL));
    h.advance(60);
    assert!(!crate::auras::has_aura(h.app.world(), owner, SEED_SPELL));
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(owner)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == SEED_SPELL)
    );
    assert_eq!(
        activate(&mut h, SEED)[0].error.as_deref(),
        Some("OnCooldown")
    );
    h.advance(600); // Natural60s recovery; no cooldown clearing or aura shortcuts.
    assert_eq!(activate(&mut h, SEED)[0].error, None);
    h.advance(70);
    assert!(crate::auras::has_aura(h.app.world(), owner, SEED_SPELL));
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|go| go.spell_id == SEED_SPELL)
            .count(),
        2
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura {
            spell_id: SEED_SPELL,
        });
    h.advance(5);
    assert!(!crate::auras::has_aura(h.app.world(), owner, SEED_SPELL));
    assert_eq!(h.app.world().get::<Health>(owner), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(owner).copied(), model);
    assert_eq!(h.app.world().get::<Position>(owner), Some(&position));
}

#[test]
fn toy_coverage_seed_visual_combat_refusal_preserves_cast_and_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("seed-combat.redb"));
    install_visual_source(&mut h);
    learn_effect_toy(&mut h, character, SEED);
    let owner = h.caster;
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(shared::components::CombatStatus(true));
    assert_eq!(
        activate(&mut h, SEED)[0].error.as_deref(),
        Some("AffectingCombat")
    );
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(owner)
            .is_none()
    );
    assert!(!crate::auras::has_aura(h.app.world(), owner, SEED_SPELL));
    assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == SEED_SPELL));
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == SEED_SPELL));
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(shared::components::CombatStatus(false));
    assert_eq!(activate(&mut h, SEED)[0].error, None);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(owner)
            .is_some()
    );
    h.advance(70);
    assert!(crate::auras::has_aura(h.app.world(), owner, SEED_SPELL));
}

#[test]
fn toy_coverage_seed_visual_changed_payload_or_missing_visual_refuses() {
    for absent_visual in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&dir.path().join("seed-drift.redb"));
        install_visual_source(&mut h);
        learn_effect_toy(&mut h, character, SEED);
        let mut changed = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(SEED_SPELL)
            .unwrap()
            .clone();
        if absent_visual {
            changed.has_spell_visual = false;
        } else {
            changed.effects[0].base_points = 1.0;
        }
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(vec![changed]));
        assert!(activate(&mut h, SEED)[0].error.is_some());
        assert!(
            h.app
                .world()
                .get::<shared::casting::CastState>(h.caster)
                .is_none()
        );
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, SEED_SPELL));
        assert!(!h.inbox.go.iter().any(|go| go.spell_id == SEED_SPELL));
        assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == SEED_SPELL));
    }
}

#[test]
fn toy_coverage_seed_visual_census_keeps_previous_visuals_and_unsourced_dummy_gap() {
    let report = report_toy_census(&crate::test_data::world_db_path(), "TOY_SEED_VISUAL_REPORT");
    let rows = report["toys"].as_array().unwrap();
    for item in [201927, 202711, 162539, 168014, 202022, 242323, SEED] {
        let row = rows.iter().find(|r| r["item_id"] == item).unwrap();
        assert!(row["unavailable"].is_null(), "{row}");
    }
    let gadgetzan = rows.iter().find(|r| r["item_id"] == 18986).unwrap();
    assert!(!gadgetzan["unavailable"].is_null());
}
