//! Source Obsidian Battle Horn: peaceful500ms cast,5s cosmetic aura,60s recovery.
use super::*;

const HORN_ITEM: u32 = 204687;
const HORN_SPELL: u32 = 406077;

#[test]
fn toy_coverage_horn_visual_real_cast_expiry_and_natural_recovery_reuse() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("horn-lifecycle.redb"));
    install_visual_source(&mut h);
    learn_effect_toy(&mut h, character, HORN_ITEM);
    let owner = h.caster;
    let health = *h.app.world().get::<Health>(owner).unwrap();
    let model = h.app.world().get::<ModelDisplay>(owner).copied();
    let position = *h.app.world().get::<Position>(owner).unwrap();
    assert_eq!(activate(&mut h, HORN_ITEM)[0].error, None); // Pump350ms < source500ms.
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(owner)
            .is_some()
    );
    assert!(!crate::auras::has_aura(h.app.world(), owner, HORN_SPELL));
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == HORN_SPELL));
    h.advance(6);
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(owner)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == HORN_SPELL)
        .unwrap();
    assert_eq!(aura.duration_ms, 5000);
    assert_eq!(aura.effects[0].amount, 0.0);
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(owner)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == HORN_SPELL && a.caster == Some(owner.to_bits()))
    );
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, HORN_SPELL));
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == HORN_SPELL && cd.duration_ms == 60_000 && !cd.is_gcd)
    );
    h.advance(80);
    assert!(crate::auras::has_aura(h.app.world(), owner, HORN_SPELL));
    h.advance(40);
    assert!(!crate::auras::has_aura(h.app.world(), owner, HORN_SPELL));
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(owner)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == HORN_SPELL)
    );
    assert_eq!(
        activate(&mut h, HORN_ITEM)[0].error.as_deref(),
        Some("OnCooldown")
    );
    h.advance(1120);
    assert_eq!(activate(&mut h, HORN_ITEM)[0].error, None);
    h.advance(6);
    assert!(crate::auras::has_aura(h.app.world(), owner, HORN_SPELL));
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|go| go.spell_id == HORN_SPELL)
            .count(),
        2
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura {
            spell_id: HORN_SPELL,
        });
    h.advance(5);
    assert!(!crate::auras::has_aura(h.app.world(), owner, HORN_SPELL));
    assert_eq!(h.app.world().get::<Health>(owner), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(owner).copied(), model);
    assert_eq!(h.app.world().get::<Position>(owner), Some(&position));
}

#[test]
fn toy_coverage_horn_visual_combat_refuses_without_starting_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("horn-combat.redb"));
    install_visual_source(&mut h);
    learn_effect_toy(&mut h, character, HORN_ITEM);
    let owner = h.caster;
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(shared::components::CombatStatus(true));
    assert_eq!(
        activate(&mut h, HORN_ITEM)[0].error.as_deref(),
        Some("AffectingCombat")
    );
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(owner)
            .is_none()
    );
    assert!(!crate::auras::has_aura(h.app.world(), owner, HORN_SPELL));
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == HORN_SPELL));
    assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == HORN_SPELL));
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(shared::components::CombatStatus(false));
    assert_eq!(activate(&mut h, HORN_ITEM)[0].error, None);
    h.advance(6);
    assert!(crate::auras::has_aura(h.app.world(), owner, HORN_SPELL));
}

#[test]
fn toy_coverage_horn_visual_missing_visual_or_gameplay_payload_refuses() {
    for absent_visual in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&dir.path().join("horn-drift.redb"));
        install_visual_source(&mut h);
        learn_effect_toy(&mut h, character, HORN_ITEM);
        let mut changed = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(HORN_SPELL)
            .unwrap()
            .clone();
        if absent_visual {
            changed.has_spell_visual = false;
        } else {
            changed.effects[0].base_points = 1.0;
        }
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(vec![changed]));
        assert!(activate(&mut h, HORN_ITEM)[0].error.is_some());
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, HORN_SPELL));
        assert!(!h.inbox.go.iter().any(|go| go.spell_id == HORN_SPELL));
        assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == HORN_SPELL));
    }
}

#[test]
fn toy_coverage_horn_visual_census_retains_unsourced_sound_and_affinity_actions() {
    let report = report_toy_census(&crate::test_data::world_db_path(), "TOY_HORN_VISUAL_REPORT");
    let rows = report["toys"].as_array().unwrap();
    let horn = rows.iter().find(|r| r["item_id"] == HORN_ITEM).unwrap();
    assert!(horn["unavailable"].is_null(), "{horn}");
    for item in [136928, 205936, 18986] {
        let row = rows.iter().find(|r| r["item_id"] == item).unwrap();
        assert!(!row["unavailable"].is_null(), "{row}");
    }
}
