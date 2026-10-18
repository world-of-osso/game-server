//! Source111–113,178–179,201,234,240,317,322,387,402,450,540,542,548,553,573,638,644 and690: transforms and recovery.
//! Source618: misc0 Royal Visage;684/685/697/702/710: bounded shared2513 use/refusal.
use super::*;

const TEN_MINUTES_MS: u32 = 600_000;
// Item, spell, creature, displays, recovery ms, aura duration ms, cast ms.
const SOURCE_DISGUISES: &[(u32, u32, i32, &[u32], u32, u32, u32)] = &[
    (
        164373,
        281298,
        121865,
        [11611, 976, 765, 4905].as_slice(),
        7_200_000,
        TEN_MINUTES_MS,
        0,
    ),
    (
        164374,
        281302,
        144261,
        [41083, 78767, 76947, 41724].as_slice(),
        3_600_000,
        TEN_MINUTES_MS,
        0,
    ),
    (
        164375,
        281303,
        144262,
        [88011, 88009, 88016, 88012].as_slice(),
        7_200_000,
        TEN_MINUTES_MS,
        0,
    ),
    (
        170154,
        303994,
        155896,
        [92513, 92512, 92511, 91369, 32123, 32122, 32121, 31982].as_slice(),
        3_600_000,
        TEN_MINUTES_MS,
        0,
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
        0,
    ),
    (
        174873,
        305123,
        156286,
        [92868, 92880, 92874, 92871].as_slice(),
        3_600_000,
        TEN_MINUTES_MS,
        0,
    ),
    (
        183903,
        343827,
        174859,
        [96239, 96240, 96241, 96242, 96243].as_slice(),
        3_600_000,
        TEN_MINUTES_MS,
        0,
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
        0,
    ),
    (
        190457,
        367568,
        185567,
        [105515].as_slice(),
        3_600_000,
        900_000,
        0,
    ),
    (
        191891,
        371470,
        187854,
        [104851].as_slice(),
        1_800_000,
        TEN_MINUTES_MS,
        0,
    ),
    (
        205904,
        410931,
        205296,
        [112407, 112523, 112524, 112525].as_slice(),
        3_600_000,
        TEN_MINUTES_MS,
        0,
    ),
    (
        225641,
        455426,
        226283,
        [
            115492, 115491, 115488, 115487, 115483, 115482, 115479, 115478, 115473, 115472, 115469,
            115468, 115465, 115464, 115461, 115460, 115459, 115458, 115455, 115454, 115451, 115450,
            115447, 115446, 115443, 115442, 115439, 115438, 115435, 115434, 115431, 115430, 115428,
            115426, 115424, 115419, 115417, 115415, 115413, 115411, 115410,
        ]
        .as_slice(),
        3_600_000,
        1_800_000,
        1_500,
    ),
    (
        225910,
        456572,
        226964,
        [
            119251, 115218, 115217, 115216, 115215, 115214, 115213, 115212, 115211, 115210, 115209,
            115208, 115207, 115206, 115205, 115204, 115202, 115201, 115200, 115199, 115198, 115197,
            115196, 115195, 115194, 115192, 115190, 114563, 114562,
        ]
        .as_slice(),
        3_600_000,
        TEN_MINUTES_MS,
        0,
    ),
    (
        226810,
        459531,
        228081,
        [124738, 123949, 125914].as_slice(),
        3_600_000,
        1_200_000,
        1_500,
    ),
    (
        228705,
        462934,
        229772,
        [
            118286, 118292, 118293, 118283, 118276, 118290, 118295, 118284, 118289, 118278, 118291,
            118281, 118285, 118287, 119763, 118294,
        ]
        .as_slice(),
        3_600_000,
        TEN_MINUTES_MS,
        0,
    ),
    (
        233202,
        473407,
        234479,
        [124464, 124467, 124468, 124471, 124470, 124469].as_slice(),
        3_600_000,
        1_800_000,
        0,
    ),
    (
        252265,
        1254376,
        251887,
        [128024, 128022, 128026, 128025, 128023].as_slice(),
        1_200_000,
        TEN_MINUTES_MS,
        0,
    ),
    (
        256893,
        1260864,
        254071,
        [100712, 93612, 98057, 93614].as_slice(),
        900_000,
        TEN_MINUTES_MS,
        0,
    ),
    (
        274921,
        1295623,
        265732,
        [
            103316, 103317, 103857, 103318, 103319, 103320, 103321, 103324, 103325, 103860,
        ]
        .as_slice(),
        3_600_000,
        300_000,
        0,
    ),
];

#[test]
fn toyfx_source_disguises_learning_expiry_and_recovery() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    const FRAME_SECONDS: f64 = 0.05;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let spells = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for &(item, spell, entry, displays, recovery_ms, duration_ms, cast_ms) in SOURCE_DISGUISES {
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
        let cast_frames = (f64::from(cast_ms) / 1000.0 / FRAME_SECONDS).ceil() as usize;
        if cast_frames > 0 {
            let first_half = cast_frames / 2;
            h.advance(first_half);
            assert!(
                !h.inbox.go.iter().any(|go| go.spell_id == spell),
                "item{item}"
            );
            assert_eq!(
                h.app.world().get::<ModelDisplay>(caster).copied(),
                native_display
            );
            h.advance(cast_frames - first_half);
        }
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
        h.advance(30); // Let the source1500ms GCD expire before checking longer toy recovery.
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
fn toyfx618_royal_visage_learned_misc0_expiry_and_recovery_restore_model() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    const ITEM: u32 = 245631;
    const SPELL: u32 = 1239480;
    const CREATURE: i32 = 236348;
    const DISPLAY: u32 = 121918;
    const DURATION_MS: u32 = 600_000;
    const RECOVERY_MS: u32 = 1_800_000;
    const FRAME_SECONDS: f64 = 0.05;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("royal-visage.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    let caster = h.caster;
    let native = h.app.world().get::<ModelDisplay>(caster).copied();
    let health = *h.app.world().get::<Health>(caster).unwrap();
    learn_effect_toy(&mut h, character, ITEM);
    assert_eq!(activate(&mut h, ITEM)[0].error, None);
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|event| event.spell_id == SPELL));
    assert_eq!(
        h.app
            .world()
            .get::<ModelDisplay>(caster)
            .unwrap()
            .display_id,
        DISPLAY
    );
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == SPELL)
    );
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == SPELL)
        .unwrap();
    assert_eq!(aura.duration_ms, DURATION_MS);
    assert_eq!(aura.effects[0].misc_values, [CREATURE, 0]);
    let expiry = aura.expires_at.unwrap();
    let expected_seconds = f64::from(DURATION_MS) / 1000.0;
    let remaining = expiry - crate::auras::aura_now(h.app.world());
    assert!(((expected_seconds - 1.0)..=expected_seconds).contains(&remaining));
    let recovery = h
        .inbox
        .cooldowns
        .iter()
        .find(|update| update.spell_id == SPELL && !update.is_gcd)
        .unwrap();
    assert_eq!(recovery.duration_ms, RECOVERY_MS);
    h.advance(30);
    assert_eq!(
        activate(&mut h, ITEM)[0].error.as_deref(),
        Some("OnCooldown")
    );
    let remaining = expiry - crate::auras::aura_now(h.app.world());
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
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), native);
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
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|event| event.spell_id == SPELL)
            .count(),
        1
    );
}

#[test]
fn toyfx_shared_saptor_idol_badge_cancel_and_recover_without_bypassing_category() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    const PANGO: u32 = 268717;
    const PANGO_SPELL: u32 = 1283490;
    const CATEGORY: u32 = 2513;
    const RECOVERY_MS: u32 = 600_000;
    const AURA_DURATION_MS: u32 = 3_600_000;
    // Source685/702/710; pools remain weighted in real content, not rewritten here.
    const CASES: &[(u32, u32, i32, &[u32])] = &[
        (
            268728,
            1283502,
            260346,
            &[136118, 122805, 122804, 122803, 122802, 122801],
        ),
        (
            276925,
            1301489,
            268496,
            &[145498, 145499, 145500, 145503, 145504],
        ),
        (
            280419,
            1308693,
            271558,
            &[145838, 145885, 146680, 145886, 142345],
        ),
    ];
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let spells = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for &(item, spell, creature, displays) in CASES {
        let (mut h, character) = setup(&directory.path().join(format!("cancel-{item}.redb")));
        h.app.insert_resource(toy_catalog::load(&conn).unwrap());
        h.app.insert_resource(shared_item_templates(&path));
        h.app.insert_resource(shared_world_data(&path));
        h.app.insert_resource(spells.clone());
        place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
        let caster = h.caster;
        let native = h.app.world().get::<ModelDisplay>(caster).copied();
        let health = *h.app.world().get::<Health>(caster).unwrap();
        learn_effect_toy(&mut h, character, item);
        learn_effect_toy(&mut h, character, PANGO);
        let started = crate::auras::aura_now(h.app.world());
        assert_eq!(activate(&mut h, item)[0].error, None, "item{item}");
        h.advance(5);
        assert!(
            displays.contains(
                &h.app
                    .world()
                    .get::<ModelDisplay>(caster)
                    .unwrap()
                    .display_id
            )
        );
        let aura = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(caster)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == spell)
            .unwrap();
        assert_eq!(aura.duration_ms, AURA_DURATION_MS, "item{item}");
        assert_eq!(aura.effects[0].misc_values, [creature, 8], "item{item}");
        let recovery = h
            .inbox
            .cooldowns
            .iter()
            .find(|update| {
                update.spell_id == spell && update.category == CATEGORY && !update.is_gcd
            })
            .unwrap();
        assert_eq!(recovery.duration_ms, RECOVERY_MS, "item{item}");
        h.app
            .world_mut()
            .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
            .unwrap()
            .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura {
                spell_id: spell,
            });
        h.advance(5);
        assert_eq!(
            h.app.world().get::<ModelDisplay>(caster).copied(),
            native,
            "item{item}"
        );
        assert!(
            !h.app
                .world()
                .get::<UnitAuras>(caster)
                .unwrap()
                .auras
                .iter()
                .any(|aura| aura.spell_id == spell),
            "item{item}"
        );
        assert_eq!(
            activate(&mut h, item)[0].error.as_deref(),
            Some("OnCooldown"),
            "item{item}"
        );
        assert_eq!(
            activate(&mut h, PANGO)[0].error.as_deref(),
            Some("OnCooldown"),
            "item{item}"
        );
        let casts_before_recovery = h.inbox.go.len();
        h.advance(11_600); // Ordinary50ms frames; never alter the actual category clock.
        assert!(crate::auras::aura_now(h.app.world()) - started < 600.0);
        assert_eq!(
            activate(&mut h, PANGO)[0].error.as_deref(),
            Some("OnCooldown"),
            "item{item}"
        );
        assert_eq!(h.inbox.go.len(), casts_before_recovery, "item{item}");
        assert_eq!(
            h.app.world().get::<ModelDisplay>(caster).copied(),
            native,
            "item{item}"
        );
        h.advance(420);
        assert!(crate::auras::aura_now(h.app.world()) - started > 600.0);
        assert_eq!(activate(&mut h, PANGO)[0].error, None, "item{item}");
        h.advance(5);
        assert_eq!(
            h.inbox.go.iter().filter(|go| go.spell_id == spell).count(),
            1,
            "item{item}"
        );
        assert_eq!(
            h.inbox
                .go
                .iter()
                .filter(|go| go.spell_id == PANGO_SPELL)
                .count(),
            1,
            "item{item}"
        );
        assert!(
            [143136, 143137, 143138, 143139, 143140].contains(
                &h.app
                    .world()
                    .get::<ModelDisplay>(caster)
                    .unwrap()
                    .display_id
            )
        );
        let aura = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(caster)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == PANGO_SPELL)
            .unwrap();
        assert_eq!(aura.duration_ms, AURA_DURATION_MS, "item{item}");
        assert_eq!(aura.effects[0].misc_values, [260345, 8], "item{item}");
        assert_eq!(
            h.app.world().get::<Health>(caster),
            Some(&health),
            "item{item}"
        );
    }
}

#[test]
fn toyfx697_masquerade_learned_cast_defers_transform_and_shared_recovery_until_completion() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    const ITEM: u32 = 276207;
    const SPELL: u32 = 1298957;
    const SIBLING: u32 = 276925;
    const CATEGORY: u32 = 2513;
    const RECOVERY_MS: u32 = 600_000;
    const AURA_DURATION_MS: u32 = 3_600_000;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("masquerade-cast.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    let caster = h.caster;
    let native = h.app.world().get::<ModelDisplay>(caster).copied();
    let health = *h.app.world().get::<Health>(caster).unwrap();
    learn_effect_toy(&mut h, character, ITEM);
    learn_effect_toy(&mut h, character, SIBLING);

    // activate pumps seven50ms frames, still short of the source600ms cast.
    assert_eq!(activate(&mut h, ITEM)[0].error, None);
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == SPELL));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), native);
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == SPELL)
    );
    assert!(!h.inbox.cooldowns.iter().any(|update| {
        update.spell_id == SPELL && update.category == CATEGORY && !update.is_gcd
    }));
    h.advance(20);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.inbox.go.iter().filter(|go| go.spell_id == SPELL).count(),
        1
    );
    assert_eq!(
        h.app
            .world()
            .get::<ModelDisplay>(caster)
            .unwrap()
            .display_id,
        145134
    );
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == SPELL)
        .unwrap();
    assert_eq!(aura.duration_ms, AURA_DURATION_MS);
    assert_eq!(aura.effects[0].misc_values, [267626, 8]);
    let expiry = aura.expires_at.unwrap();
    let recovery = h
        .inbox
        .cooldowns
        .iter()
        .find(|update| update.spell_id == SPELL && update.category == CATEGORY && !update.is_gcd)
        .unwrap();
    assert_eq!(recovery.duration_ms, RECOVERY_MS);
    let cast_count = h.inbox.go.len();
    assert_eq!(
        activate(&mut h, SIBLING)[0].error.as_deref(),
        Some("OnCooldown")
    );
    assert_eq!(h.inbox.go.len(), cast_count);
    assert_eq!(
        h.app
            .world()
            .get::<ModelDisplay>(caster)
            .unwrap()
            .display_id,
        145134
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(caster)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == SPELL)
            .unwrap()
            .expires_at,
        Some(expiry)
    );
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
}

#[test]
fn toyfx684_pango_cancel_restores_model_without_bypassing_shared_recovery() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    const PANGO: u32 = 268717;
    const PANGO_SPELL: u32 = 1283490;
    const SAPTOR: u32 = 268728;
    const SAPTOR_SPELL: u32 = 1283502;
    const CATEGORY: u32 = 2513;
    const RECOVERY_MS: u32 = 600_000;
    const AURA_DURATION_MS: u32 = 3_600_000;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("pango-cancel.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    let caster = h.caster;
    let native = h.app.world().get::<ModelDisplay>(caster).copied();
    let health = *h.app.world().get::<Health>(caster).unwrap();
    learn_effect_toy(&mut h, character, PANGO);
    learn_effect_toy(&mut h, character, SAPTOR);
    let started = crate::auras::aura_now(h.app.world());
    assert_eq!(activate(&mut h, PANGO)[0].error, None);
    h.advance(5);
    let transformed = h.app.world().get::<ModelDisplay>(caster).unwrap();
    assert!([143136, 143137, 143138, 143139, 143140].contains(&transformed.display_id));
    let recovery = h
        .inbox
        .cooldowns
        .iter()
        .find(|update| {
            update.spell_id == PANGO_SPELL && update.category == CATEGORY && !update.is_gcd
        })
        .unwrap();
    assert_eq!(recovery.duration_ms, RECOVERY_MS);
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura {
            spell_id: PANGO_SPELL,
        });
    h.advance(5);
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), native);
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == PANGO_SPELL)
    );
    for item in [PANGO, SAPTOR] {
        assert_eq!(
            activate(&mut h, item)[0].error.as_deref(),
            Some("OnCooldown")
        );
    }
    let casts_before_recovery = h.inbox.go.len();

    // Ordinary50ms frames only: cancellation and failed uses must not clear/restart recovery.
    h.advance(11_600);
    assert!(crate::auras::aura_now(h.app.world()) - started < 600.0);
    assert_eq!(
        activate(&mut h, SAPTOR)[0].error.as_deref(),
        Some("OnCooldown")
    );
    assert_eq!(h.inbox.go.len(), casts_before_recovery);
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), native);
    h.advance(420);
    assert!(crate::auras::aura_now(h.app.world()) - started > 600.0);
    assert_eq!(activate(&mut h, SAPTOR)[0].error, None);
    h.advance(5);
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|go| go.spell_id == PANGO_SPELL)
            .count(),
        1
    );
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|go| go.spell_id == SAPTOR_SPELL)
            .count(),
        1
    );
    let transformed = h.app.world().get::<ModelDisplay>(caster).unwrap();
    assert!([136118, 122805, 122804, 122803, 122802, 122801].contains(&transformed.display_id));
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == SAPTOR_SPELL)
        .unwrap();
    assert_eq!(aura.duration_ms, AURA_DURATION_MS);
    assert_eq!(aura.effects[0].misc_values, [260346, 8]);
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
}

#[test]
fn toyfx_shared_transform_category_learned_use_preserves_hour_aura_and_blocks_sibling() {
    use shared::components::{Health, ModelDisplay};
    const AURA_DURATION_MS: u32 = 3_600_000;
    const CATEGORY_RECOVERY_MS: u32 = 600_000;
    const CATEGORY: u32 = 2513;
    // Item, spell, creature, display pool, learned sibling sharing category2513.
    const CASES: &[(u32, u32, i32, &[u32], u32)] = &[
        (
            268717,
            1283490,
            260345,
            [143136, 143137, 143138, 143139, 143140].as_slice(),
            268728,
        ),
        (
            268728,
            1283502,
            260346,
            [136118, 122805, 122804, 122803, 122802, 122801].as_slice(),
            276207,
        ),
        (276207, 1298957, 267626, [145134].as_slice(), 276925),
        (
            276925,
            1301489,
            268496,
            [145498, 145499, 145500, 145503, 145504].as_slice(),
            280419,
        ),
        (
            280419,
            1308693,
            271558,
            [145838, 145885, 146680, 145886, 142345].as_slice(),
            268717,
        ),
    ];
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let spells = crate::spell_info::load_spell_data(&conn).unwrap().0;
    for &(item, spell, entry, displays, sibling) in CASES {
        let (mut h, character) = setup(&directory.path().join(format!("category2513-{item}.redb")));
        h.app.insert_resource(toy_catalog::load(&conn).unwrap());
        h.app.insert_resource(shared_item_templates(&path));
        h.app.insert_resource(shared_world_data(&path));
        h.app.insert_resource(spells.clone());
        place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
        let caster = h.caster;
        let health = *h.app.world().get::<Health>(caster).unwrap();
        learn_effect_toy(&mut h, character, item);
        learn_effect_toy(&mut h, character, sibling);
        assert_eq!(activate(&mut h, item)[0].error, None, "item{item}");
        h.advance(20); // Includes Masquerade's authored600ms cast, not its one-hour expiry.
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
        assert_eq!(aura.duration_ms, AURA_DURATION_MS, "item{item}");
        assert_eq!(aura.effects[0].misc_values, [entry, 8], "item{item}");
        let expiry = aura.expires_at.unwrap();
        let recovery = h
            .inbox
            .cooldowns
            .iter()
            .find(|update| {
                update.spell_id == spell && update.category == CATEGORY && !update.is_gcd
            })
            .unwrap();
        assert_eq!(recovery.duration_ms, CATEGORY_RECOVERY_MS, "item{item}");
        let cast_count = h.inbox.go.len();
        assert_eq!(
            activate(&mut h, sibling)[0].error.as_deref(),
            Some("OnCooldown"),
            "item{item} blocks learned sibling{sibling}"
        );
        assert_eq!(h.inbox.go.len(), cast_count, "sibling{sibling}");
        assert_eq!(
            h.app.world().get::<ModelDisplay>(caster),
            Some(&transformed)
        );
        assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
        assert_eq!(
            h.app
                .world()
                .get::<crate::auras::ActiveAuras>(caster)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == spell)
                .unwrap()
                .expires_at,
            Some(expiry),
            "sibling rejection does not refresh item{item}"
        );
    }
}

#[test]
fn toyfx387_chewed_wand_self_transform_and_confusion_preserve_ordinary_health_regeneration() {
    use crate::spell_cast::powers::PlayerHealthRegen;
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
    let regen = *h.app.world().get::<PlayerHealthRegen>(caster).unwrap();
    let level = *h.app.world().get::<Level>(caster).unwrap();
    let combat = *h.app.world().get::<CombatStatus>(caster).unwrap();
    // Same clock and regen phase, but never targeted by the toy.
    let regen_control = h.app.world_mut().spawn((health, regen, level, combat)).id();
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
    assert_eq!(
        h.app.world().get::<Health>(caster),
        h.app.world().get::<Health>(regen_control)
    );
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
    assert_eq!(
        h.app.world().get::<Health>(caster),
        h.app.world().get::<Health>(regen_control)
    );
    assert!(h.app.world().get::<Health>(regen_control).unwrap().current > DAMAGED_HEALTH);
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
