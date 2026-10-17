//! Source151: actual map-ground cast, source item recovery and ordinary-frame reuse.
use super::*;

#[test]
fn toyfx151_azerite_firework_real_ground_learning_and_natural_item_recovery() {
    use shared::components::{Health, ModelDisplay, UnitAuras};
    const ITEM: u32 = 166877;
    const SPELL: u32 = 290627;
    const FRAME_SECONDS: f64 = 0.05;
    let directory = tempfile::tempdir().unwrap();
    let path = crate::test_data::world_db_path();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("azerite-firework.redb"));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    let caster = h.caster;
    let position = *h.app.world().get::<Position>(caster).unwrap();
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let model = h.app.world().get::<ModelDisplay>(caster).copied();
    learn_effect_toy(&mut h, character, ITEM);
    let requested_at = h.app.world().resource::<Time>().elapsed_secs_f64();
    assert_eq!(activate(&mut h, ITEM)[0].error, None);
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.inbox.go.iter().filter(|go| go.spell_id == SPELL).count(),
        1
    );
    assert_eq!(
        h.inbox
            .cooldowns
            .iter()
            .find(|cooldown| cooldown.spell_id == SPELL && !cooldown.is_gcd)
            .unwrap()
            .duration_ms,
        600_000
    );
    let first_refusal_at = h.app.world().resource::<Time>().elapsed_secs_f64();
    for boundary in [first_refusal_at, requested_at + 599.0] {
        let remaining = boundary - h.app.world().resource::<Time>().elapsed_secs_f64();
        h.advance((remaining.max(0.0) / FRAME_SECONDS).floor() as usize);
        assert_eq!(
            activate(&mut h, ITEM)[0].error.as_deref(),
            Some("OnCooldown")
        );
        h.advance(2);
        assert!(
            h.inbox.failed.iter().all(|failed| failed.spell_id == SPELL
                && failed.reason == shared::spell_data::CastFailReason::OnCooldown),
            "{:?}",
            h.inbox.failed
        );
        h.inbox.failed.clear();
        assert_eq!(
            h.inbox.go.iter().filter(|go| go.spell_id == SPELL).count(),
            1
        );
    }
    // The rejected request's delivery frames plus1.5s cross the original600s deadline.
    // No clock, cooldown or source duration is reset or accelerated.
    h.advance(30);
    assert_eq!(activate(&mut h, ITEM)[0].error, None);
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.inbox.go.iter().filter(|go| go.spell_id == SPELL).count(),
        2
    );
    assert_eq!(h.app.world().get::<Position>(caster), Some(&position));
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), model);
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(caster)
            .is_none_or(|auras| auras.auras.iter().all(|aura| aura.spell_id != SPELL))
    );
    assert_eq!(
        h.app
            .world_mut()
            .query::<&crate::spell_cast::summon::SummonedCreature>()
            .iter(h.app.world())
            .count(),
        0
    );
}
