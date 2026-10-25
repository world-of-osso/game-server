//! Oomgut104: actual learned use, source transform choices and ordinary ten-minute expiry.
use super::*;

const OOMGUT_DISPLAYS: &[u32] = &[
    32087, 32090, 32089, 32088, 37141, 32093, 32096, 32095, 32094, 87994, 32102, 32100, 32099,
    32113, 32116, 32115, 32114,
];

#[test]
fn toyfx104_oomgut_real_learning_source_transform_expiry_and_item_recovery() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    const ITEM: u32 = 163795;
    const SPELL: u32 = 280308;
    const DURATION_MS: u32 = 600_000;
    const RECOVERY_MS: u32 = 7_200_000;
    const FRAME_SECONDS: f64 = 0.05;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("oomgut.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    let caster = h.caster;
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let native_display = h.app.world().get::<ModelDisplay>(caster).copied();
    learn_effect_toy(&mut h, character, ITEM);
    assert_eq!(activate(&mut h, ITEM)[0].error, None);
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == SPELL));
    let transformed = *h.app.world().get::<ModelDisplay>(caster).unwrap();
    assert!(OOMGUT_DISPLAYS.contains(&transformed.display_id));
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == SPELL)
        .unwrap();
    assert_eq!(aura.duration_ms, DURATION_MS);
    assert_eq!(aura.effects[0].misc_values, [132902, 8]);
    let expiry = aura.expires_at.unwrap();
    let remaining = expiry - crate::auras::aura_now(h.app.world());
    assert!((599.0..=600.0).contains(&remaining));
    let recovery = h
        .inbox
        .cooldowns
        .iter()
        .find(|cooldown| cooldown.spell_id == SPELL && !cooldown.is_gcd)
        .unwrap();
    assert_eq!(recovery.duration_ms, RECOVERY_MS);
    assert_eq!(
        activate(&mut h, ITEM)[0].error.as_deref(),
        Some("OnCooldown")
    );
    let remaining = expiry - crate::auras::aura_now(h.app.world());
    h.advance((remaining / FRAME_SECONDS).floor() as usize - 2);
    assert_eq!(
        h.app.world().get::<ModelDisplay>(caster),
        Some(&transformed)
    );
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == SPELL)
    );
    h.advance(6);
    assert_eq!(
        h.app.world().get::<ModelDisplay>(caster).copied(),
        native_display
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
    assert_eq!(
        activate(&mut h, ITEM)[0].error.as_deref(),
        Some("OnCooldown"),
        "ten-minute appearance expiry does not clear the two-hour item recovery"
    );
}
