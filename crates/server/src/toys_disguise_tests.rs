//! Source111–113,178–179,201,234,240,317,322,387 and402: real transforms and source recovery.
use super::*;

#[test]
fn toyfx_source_disguises_learning_expiry_and_recovery() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    const TEN_MINUTES_MS: u32 = 600_000;
    const FRAME_SECONDS: f64 = 0.05;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let spells = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for (item, spell, entry, displays, recovery_ms, duration_ms) in [
        (
            164373,
            281298,
            121865,
            [11611, 976, 765, 4905].as_slice(),
            7_200_000,
            TEN_MINUTES_MS,
        ),
        (
            164374,
            281302,
            144261,
            [41083, 78767, 76947, 41724].as_slice(),
            3_600_000,
            TEN_MINUTES_MS,
        ),
        (
            164375,
            281303,
            144262,
            [88011, 88009, 88016, 88012].as_slice(),
            7_200_000,
            TEN_MINUTES_MS,
        ),
        (
            170154,
            303994,
            155896,
            [92513, 92512, 92511, 91369, 32123, 32122, 32121, 31982].as_slice(),
            3_600_000,
            TEN_MINUTES_MS,
        ),
        (
            170155,
            303995,
            155897,
            [
                92514, 92515, 92516, 92517, 92518, 92519, 92520, 92521, 92522, 92523, 92524, 92525,
                92526, 92527, 92528, 92529,
            ]
            .as_slice(),
            3_600_000,
            TEN_MINUTES_MS,
        ),
        (
            174873,
            305123,
            156286,
            [92868, 92880, 92874, 92871].as_slice(),
            3_600_000,
            TEN_MINUTES_MS,
        ),
        (
            183903,
            343827,
            174859,
            [96239, 96240, 96241, 96242, 96243].as_slice(),
            3_600_000,
            TEN_MINUTES_MS,
        ),
        (
            184223,
            344795,
            175221,
            [
                92410, 92411, 92412, 92413, 92414, 92415, 92416, 92417, 92418,
            ]
            .as_slice(),
            3_600_000,
            TEN_MINUTES_MS,
        ),
        (
            190457,
            367568,
            185567,
            [105515].as_slice(),
            3_600_000,
            900_000,
        ),
        (
            191891,
            371470,
            187854,
            [104851].as_slice(),
            1_800_000,
            TEN_MINUTES_MS,
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
        assert_eq!(aura.duration_ms, duration_ms, "item{item}");
        assert_eq!(aura.effects[0].misc_values, [entry, 8], "item{item}");
        let expiry = aura.expires_at.unwrap();
        let remaining = expiry - crate::auras::aura_now(h.app.world());
        let expected_seconds = f64::from(duration_ms) / 1000.0;
        assert!(
            ((expected_seconds - 1.0)..=expected_seconds).contains(&remaining),
            "item{item}"
        );
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

#[test]
fn toyfx387_chewed_wand_self_polymorph_and_confusion_expire_without_health_change() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    const ITEM: u32 = 200249;
    const SPELL: u32 = 389324;
    const DURATION_MS: u32 = 10_000;
    const RECOVERY_MS: u32 = 1_800_000;
    const DAMAGED_HEALTH: f32 = 37.0;
    const SHEEP_DISPLAYS: &[u32] = &[856, 857];
    const FRAME_SECONDS: f64 = 0.05;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let spells = crate::spell_info::load_spell_data(&conn).unwrap().0;
    let frostbolt = spells.get(116).unwrap().clone();
    let (mut h, character) = setup(&directory.path().join("chewed-wand.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app.insert_resource(spells);
    let caster = h.caster;
    h.app.world_mut().get_mut::<Health>(caster).unwrap().current = DAMAGED_HEALTH;
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let native_display = h.app.world().get::<ModelDisplay>(caster).copied();
    learn_effect_toy(&mut h, character, ITEM);
    assert_eq!(activate(&mut h, ITEM)[0].error, None);
    h.advance(20); // One second after activation: source1700ms cast is still pending.
    assert!(!h.inbox.go.iter().any(|event| event.spell_id == SPELL));
    assert_eq!(
        h.app.world().get::<ModelDisplay>(caster).copied(),
        native_display
    );
    h.advance(20);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|event| event.spell_id == SPELL));
    let transformed = *h.app.world().get::<ModelDisplay>(caster).unwrap();
    assert!(SHEEP_DISPLAYS.contains(&transformed.display_id));
    let auras = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .unwrap();
    assert!(auras.is_confused());
    let aura = auras.iter().find(|aura| aura.spell_id == SPELL).unwrap();
    assert_eq!(aura.duration_ms, DURATION_MS);
    let expiry = aura.expires_at.unwrap();
    assert!(crate::auras::cast_blocker(h.app.world(), caster, &frostbolt).is_some());
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(
        h.inbox
            .cooldowns
            .iter()
            .find(|cooldown| cooldown.spell_id == SPELL && !cooldown.is_gcd)
            .unwrap()
            .duration_ms,
        RECOVERY_MS
    );
    let remaining = expiry - crate::auras::aura_now(h.app.world());
    h.advance((remaining / FRAME_SECONDS).floor() as usize - 2);
    assert_eq!(
        h.app.world().get::<ModelDisplay>(caster),
        Some(&transformed)
    );
    assert!(crate::auras::cast_blocker(h.app.world(), caster, &frostbolt).is_some());
    h.advance(6);
    assert_eq!(
        h.app.world().get::<ModelDisplay>(caster).copied(),
        native_display
    );
    assert!(crate::auras::cast_blocker(h.app.world(), caster, &frostbolt).is_none());
    let confused = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .is_some_and(|auras| auras.is_confused());
    assert!(!confused);
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
        Some("OnCooldown")
    );
}

#[test]
fn toyfx402_soulshape_reuses_before_appearance_expires_and_refreshes_naturally() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    const ITEM: u32 = 200960;
    const SPELL: u32 = 392700;
    const DISPLAY: u32 = 100634;
    const DURATION_MS: u32 = 600_000;
    const RECOVERY_MS: u32 = 300_000;
    const FRAME_SECONDS: f64 = 0.05;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("soulshape.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    let caster = h.caster;
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let native_display = h.app.world().get::<ModelDisplay>(caster).copied();
    let read_source_lifetime = |world: &World| {
        let aura = world
            .get::<crate::auras::ActiveAuras>(caster)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == SPELL)
            .unwrap();
        (aura.duration_ms, aura.expires_at.unwrap())
    };
    learn_effect_toy(&mut h, character, ITEM);
    assert_eq!(activate(&mut h, ITEM)[0].error, None);
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.app
            .world()
            .get::<ModelDisplay>(caster)
            .unwrap()
            .display_id,
        DISPLAY
    );
    let (duration, first_expiry) = read_source_lifetime(h.app.world());
    assert_eq!(duration, DURATION_MS);
    assert_eq!(
        activate(&mut h, ITEM)[0].error.as_deref(),
        Some("OnCooldown")
    );
    h.advance(5_960); // 298 seconds: still before the source five-minute recovery.
    assert_eq!(
        activate(&mut h, ITEM)[0].error.as_deref(),
        Some("OnCooldown")
    );
    assert_eq!(read_source_lifetime(h.app.world()).1, first_expiry);
    assert_eq!(
        h.app
            .world()
            .get::<ModelDisplay>(caster)
            .unwrap()
            .display_id,
        DISPLAY
    );
    h.advance(30); // Natural1.5 seconds crosses recovery while appearance is active.
    assert_eq!(activate(&mut h, ITEM)[0].error, None);
    h.advance(5);
    let (duration, refreshed_expiry) = read_source_lifetime(h.app.world());
    assert_eq!(duration, DURATION_MS);
    assert!(refreshed_expiry > first_expiry);
    assert_eq!(
        h.app
            .world()
            .get::<ModelDisplay>(caster)
            .unwrap()
            .display_id,
        DISPLAY
    );
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|event| event.spell_id == SPELL)
            .count(),
        2
    );
    let cooldowns: Vec<_> = h
        .inbox
        .cooldowns
        .iter()
        .filter(|cooldown| cooldown.spell_id == SPELL && !cooldown.is_gcd)
        .map(|cooldown| cooldown.duration_ms)
        .collect();
    assert_eq!(cooldowns, vec![RECOVERY_MS; 2]);
    let rejections: Vec<_> = h
        .inbox
        .failed
        .iter()
        .map(|failure| (failure.spell_id, failure.reason))
        .collect();
    assert_eq!(
        rejections,
        vec![(SPELL, shared::spell_data::CastFailReason::OnCooldown); 2]
    );
    let remaining = refreshed_expiry - crate::auras::aura_now(h.app.world());
    let source_seconds = f64::from(DURATION_MS) / 1000.0;
    assert!(((source_seconds - 1.0)..=source_seconds).contains(&remaining));
    h.advance((remaining / FRAME_SECONDS).floor() as usize - 2);
    assert_eq!(
        h.app
            .world()
            .get::<ModelDisplay>(caster)
            .unwrap()
            .display_id,
        DISPLAY
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
}
