//! Source473: actual self-stun use, ordinary cast refusal and source expiry.
use super::*;

#[test]
fn toyfx473_stasis_sand_stuns_until_source_expiry_and_preserves_item_recovery() {
    use crate::auras::ActiveAuras;
    use shared::components::{Health, ModelDisplay, UnitAuras};
    use shared::spell_data::CastFailReason;
    const ITEM: u32 = 208415;
    const SPELL: u32 = 419271;
    const FROSTBOLT: u32 = 116;
    const DURATION_MS: u32 = 60_000;
    const RECOVERY_MS: u32 = 300_000;
    const FRAME_SECONDS: f64 = 0.05;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("stasis-sand.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    let caster = h.caster;
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(caster).copied();
    learn_effect_toy(&mut h, character, ITEM);
    assert_eq!(activate(&mut h, ITEM)[0].error, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|event| event.spell_id == SPELL));
    let auras = h.app.world().get::<ActiveAuras>(caster).unwrap();
    assert!(auras.is_stunned());
    let aura = auras.iter().find(|aura| aura.spell_id == SPELL).unwrap();
    assert_eq!(aura.duration_ms, DURATION_MS);
    let expiry = aura.expires_at.unwrap();
    assert!(h.inbox.cooldowns.iter().any(|cooldown| {
        cooldown.spell_id == SPELL && !cooldown.is_gcd && cooldown.duration_ms == RECOVERY_MS
    }));
    h.cast_and_deliver(FROSTBOLT, Some(h.npc));
    assert!(!h.inbox.go.iter().any(|event| event.spell_id == FROSTBOLT));
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(h.inbox.failed[0].spell_id, FROSTBOLT);
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::Stunned);
    let remaining = expiry - crate::auras::aura_now(h.app.world());
    h.advance((remaining / FRAME_SECONDS).floor() as usize - 2);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(caster)
            .unwrap()
            .is_stunned()
    );
    h.advance(6);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(caster)
            .is_none_or(|auras| !auras.is_stunned())
    );
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == SPELL)
    );
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    // Check out-of-combat toy recovery before the restored Frostbolt starts combat.
    assert_eq!(
        activate(&mut h, ITEM)[0].error.as_deref(),
        Some("OnCooldown")
    );
    assert_eq!(h.inbox.failed.len(), 2, "{:?}", h.inbox.failed);
    assert_eq!(h.inbox.failed[1].spell_id, SPELL);
    assert_eq!(h.inbox.failed[1].reason, CastFailReason::OnCooldown);
    h.cast_and_deliver(FROSTBOLT, Some(h.npc));
    h.advance(80);
    assert!(h.inbox.go.iter().any(|event| event.spell_id == FROSTBOLT));
    assert_eq!(h.inbox.failed.len(), 2, "{:?}", h.inbox.failed);
}
