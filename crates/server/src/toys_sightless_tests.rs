//! Sightless Eye153293: source detected-range -15, Argus/Broken Isles, ordinary10min aura.
use super::*;

#[test]
fn toyfx65_sightless_real_learning_use_preserves_source_lifetime_area_and_recovery() {
    use crate::auras::ActiveAuras;
    use shared::components::{Health, ModelDisplay};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("sightless.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    let caster = h.caster;
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(caster).copied();
    learn_effect_toy(&mut h, character, 153293);
    place_toy_caster_on_source_map_area(&mut h, 1220, 955950, [4584.0, 6450.666], 7558);
    assert_eq!(
        activate(&mut h, 153293)[0].error,
        None,
        "real source152 journal admission"
    );
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 254568));
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 254568)
        .unwrap();
    assert_eq!(aura.duration_ms, 600_000);
    let expires_at = aura.expires_at.unwrap();
    let remaining = expires_at - crate::auras::aura_now(h.app.world());
    assert!(
        (599.0..=600.0).contains(&remaining),
        "authored10min aura: {remaining}"
    );
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == 254568 && cd.duration_ms == 300_000 && !cd.is_gcd)
    );
    assert_eq!(
        activate(&mut h, 153293)[0].error.as_deref(),
        Some("OnCooldown")
    );
    let remaining = expires_at - crate::auras::aura_now(h.app.world());
    h.advance((remaining / 0.05).ceil() as usize + 2);
    assert!(
        !h.app
            .world()
            .get::<ActiveAuras>(caster)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 254568)
    );
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    assert_eq!(
        activate(&mut h, 153293)[0].error.as_deref(),
        Some("Requires area group5454")
    );
    place_toy_caster_on_source_map_area(&mut h, 1220, 955950, [4584.0, 6450.666], 7558);
    assert_eq!(
        activate(&mut h, 153293)[0].error,
        None,
        "source5min recovery expires normally before10min aura ends"
    );
}
