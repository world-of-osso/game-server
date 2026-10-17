//! Source118–123: actual dragon costumes and ordinary fifteen-minute recovery.
use super::*;

#[test]
fn toyfx118_dragon_blue_head_real_learning_expiry_and_natural_reuse() {
    assert_costume_learning_expiry_and_reuse(165671, 285519, 147018, 89489);
}

#[test]
fn toyfx119_dragon_blue_body_real_learning_expiry_and_natural_reuse() {
    assert_costume_learning_expiry_and_reuse(165672, 285521, 147020, 89491);
}

#[test]
fn toyfx120_dragon_blue_tail_real_learning_expiry_and_natural_reuse() {
    assert_costume_learning_expiry_and_reuse(165673, 285523, 147022, 89493);
}

#[test]
fn toyfx121_dragon_green_head_real_learning_expiry_and_natural_reuse() {
    assert_costume_learning_expiry_and_reuse(165674, 285520, 147019, 89490);
}

#[test]
fn toyfx122_dragon_green_body_real_learning_expiry_and_natural_reuse() {
    assert_costume_learning_expiry_and_reuse(165675, 285522, 147021, 89492);
}

#[test]
fn toyfx123_dragon_green_tail_real_learning_expiry_and_natural_reuse() {
    assert_costume_learning_expiry_and_reuse(165676, 285524, 147023, 89494);
}

fn assert_costume_learning_expiry_and_reuse(item: u32, spell: u32, entry: u32, display: u32) {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    const DURATION_MS: u32 = 900_000;
    const RECOVERY_MS: u32 = 900_000;
    const FRAME_SECONDS: f64 = 0.05;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join(format!("costume-{item}.redb")));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    let caster = h.caster;
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let native_display = h.app.world().get::<ModelDisplay>(caster).copied();
    learn_effect_toy(&mut h, character, item);
    assert_eq!(activate(&mut h, item)[0].error, None);
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
    let transformed = *h.app.world().get::<ModelDisplay>(caster).unwrap();
    assert_eq!(transformed.display_id, display);
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell)
        .unwrap();
    assert_eq!(aura.duration_ms, DURATION_MS);
    assert_eq!(aura.effects[0].misc_values, [entry, 8]);
    let expiry = aura.expires_at.unwrap();
    let remaining = expiry - crate::auras::aura_now(h.app.world());
    assert!((899.0..=900.0).contains(&remaining));
    let recovery = h
        .inbox
        .cooldowns
        .iter()
        .find(|cooldown| cooldown.spell_id == spell && !cooldown.is_gcd)
        .unwrap();
    assert_eq!(recovery.duration_ms, RECOVERY_MS);
    assert_eq!(
        activate(&mut h, item)[0].error.as_deref(),
        Some("OnCooldown")
    );
    h.advance(2);
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(h.inbox.failed[0].spell_id, spell);
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::OnCooldown
    );
    h.inbox.failed.clear();
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
            .any(|aura| aura.spell_id == spell)
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
            .any(|aura| aura.spell_id == spell)
    );
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(
        activate(&mut h, item)[0].error,
        None,
        "the fifteen-minute item recovery naturally completes with appearance expiry"
    );
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.app
            .world()
            .get::<ModelDisplay>(caster)
            .unwrap()
            .display_id,
        display
    );
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == spell)
    );
}
