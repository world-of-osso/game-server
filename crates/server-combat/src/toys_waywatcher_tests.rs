//! Source130249/195806: Val'sharah-only, source two-model disguise, ordinary15s expiry.
use super::*;

#[test]
fn toyfx64_waywatcher_real_learning_use_preserves_source_displays_and_expiry() {
    use crate::auras::{ActiveAuras, AuraStatMods};
    use shared::components::{Health, ModelDisplay, UnitFlags2};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("waywatcher.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    let caster = h.caster;
    h.app.world_mut().entity_mut(caster).insert(UnitFlags2(1));
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(caster).copied();
    learn_effect_toy(&mut h, character, 130249);
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    assert_eq!(
        activate(&mut h, 130249)[0].error.as_deref(),
        Some("Requires area group4273")
    );
    assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == 195806));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
    place_toy_caster_on_source_map_area(&mut h, 1220, 955950, [4584.0, 6450.666], 7558);
    assert_eq!(activate(&mut h, 130249)[0].error, None);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 195806));
    let transformed = h
        .app
        .world()
        .get::<ModelDisplay>(caster)
        .unwrap()
        .display_id;
    assert!(
        [63609, 63606].contains(&transformed),
        "source99572 model choice: {transformed}"
    );
    assert_eq!(
        h.app.world().get::<UnitFlags2>(caster),
        Some(&UnitFlags2(0x41))
    );
    assert_eq!(
        h.app
            .world()
            .get::<AuraStatMods>(caster)
            .unwrap()
            .movement_speed,
        3.0
    );
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 195806)
        .unwrap();
    assert_eq!(aura.duration_ms, 15_000);
    let expires_at = aura.expires_at.unwrap();
    let remaining = expires_at - crate::auras::aura_now(h.app.world());
    assert!(
        (14.0..=15.0).contains(&remaining),
        "authored15s remaining: {remaining}"
    );
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == 195806 && cd.duration_ms == 900_000 && !cd.is_gcd)
    );
    assert_eq!(
        activate(&mut h, 130249)[0].error.as_deref(),
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
            .any(|aura| aura.spell_id == 195806)
    );
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
    assert_eq!(
        h.app.world().get::<UnitFlags2>(caster),
        Some(&UnitFlags2(1))
    );
    assert_eq!(
        h.app
            .world()
            .get::<AuraStatMods>(caster)
            .unwrap()
            .movement_speed,
        1.0
    );
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(
        activate(&mut h, 130249)[0].error.as_deref(),
        Some("OnCooldown")
    );
}
