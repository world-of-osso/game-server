//! Source111–113: real transforms and item overrides versus spell-owned recovery.
use super::*;

#[test]
fn toyfx111_113_disguises_real_learning_models_expiry_and_source_recovery_precedence() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    const DURATION_MS: u32 = 600_000;
    const FRAME_SECONDS: f64 = 0.05;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let spells = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for (item, spell, entry, displays, recovery_ms) in [
        (164373, 281298, 121865, [11611, 976, 765, 4905], 7_200_000),
        (
            164374,
            281302,
            144261,
            [41083, 78767, 76947, 41724],
            3_600_000,
        ),
        (
            164375,
            281303,
            144262,
            [88011, 88009, 88016, 88012],
            7_200_000,
        ),
    ] {
        let (mut h, character) = setup(&directory.path().join(format!("disguise-{item}.redb")));
        h.app.insert_resource(toy_catalog::load(&conn).unwrap());
        h.app.insert_resource(shared_item_templates(&path));
        h.app.insert_resource(shared_world_data(&path));
        h.app.insert_resource(spells.clone());
        place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
        let caster = h.caster;
        let native_display = h.app.world().get::<ModelDisplay>(caster).copied();
        let health = *h.app.world().get::<Health>(caster).unwrap();
        learn_effect_toy(&mut h, character, item);
        assert_eq!(activate(&mut h, item)[0].error, None, "item{item}");
        h.advance(5);
        assert!(
            h.inbox.failed.is_empty(),
            "item{item}: {:?}",
            h.inbox.failed
        );
        assert!(
            h.inbox.go.iter().any(|go| go.spell_id == spell),
            "item{item}"
        );
        let transformed = *h.app.world().get::<ModelDisplay>(caster).unwrap();
        assert!(displays.contains(&transformed.display_id), "item{item}");
        let aura = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(caster)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == spell)
            .unwrap();
        assert_eq!(aura.duration_ms, DURATION_MS, "item{item}");
        assert_eq!(aura.effects[0].misc_values, [entry, 8], "item{item}");
        let expiry = aura.expires_at.unwrap();
        let remaining = expiry - crate::auras::aura_now(h.app.world());
        assert!((599.0..=600.0).contains(&remaining), "item{item}");
        let cooldown = h
            .inbox
            .cooldowns
            .iter()
            .find(|cooldown| cooldown.spell_id == spell && !cooldown.is_gcd)
            .unwrap();
        assert_eq!(cooldown.duration_ms, recovery_ms, "item{item}");
        assert_eq!(
            activate(&mut h, item)[0].error.as_deref(),
            Some("OnCooldown"),
            "item{item}"
        );
        let remaining = expiry - crate::auras::aura_now(h.app.world());
        h.advance((remaining / FRAME_SECONDS).floor() as usize - 2);
        assert_eq!(
            h.app.world().get::<ModelDisplay>(caster),
            Some(&transformed),
            "item{item} remains transformed before source expiry"
        );
        h.advance(6);
        assert_eq!(
            h.app.world().get::<ModelDisplay>(caster).copied(),
            native_display,
            "item{item} restores its previous model"
        );
        assert!(
            !h.app
                .world()
                .get::<UnitAuras>(caster)
                .unwrap()
                .auras
                .iter()
                .any(|aura| aura.spell_id == spell),
            "item{item} removes its expired replicated aura"
        );
        assert_eq!(
            h.app.world().get::<Health>(caster),
            Some(&health),
            "item{item}"
        );
        assert_eq!(
            activate(&mut h, item)[0].error.as_deref(),
            Some("OnCooldown"),
            "item{item} recovery outlives its appearance"
        );
    }
}
