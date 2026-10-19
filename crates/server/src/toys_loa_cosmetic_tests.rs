//! Build70448 Loa cosmetics: learning, cancellation and ordinary lifetime/reuse.
use super::*;
use shared::components::{Health, ModelDisplay, UnitAuras};

#[test]
fn toyfx70448_loa_victory_learns_applies_and_cancels_without_resetting_recovery() {
    assert_learned_loa_cosmetic_cancellation(280820, 1309827);
}

#[test]
fn toyfx70448_nalorakk_charm_learns_applies_and_cancels_without_resetting_recovery() {
    assert_learned_loa_cosmetic_cancellation(280822, 1309830);
}

#[test]
fn toyfx70448_akilzon_feathers_learns_applies_and_cancels_without_resetting_recovery() {
    assert_learned_loa_cosmetic_cancellation(280823, 1309831);
}

#[test]
fn toyfx70448_halazzi_trail_learns_applies_and_cancels_without_resetting_recovery() {
    assert_learned_loa_cosmetic_cancellation(280821, 1309828);
}

#[test]
fn toyfx70448_janalai_flame_learns_applies_and_cancels_without_resetting_recovery() {
    assert_learned_loa_cosmetic_cancellation(280825, 1309844);
}

#[test]
fn toyfx70448_loa_victory_ordinary_thirty_minute_expiry_and_hour_reuse() {
    assert_ordinary_loa_expiry_and_reuse(280820, 1309827);
}

#[test]
fn toyfx70448_nalorakk_charm_ordinary_thirty_minute_expiry_and_hour_reuse() {
    assert_ordinary_loa_expiry_and_reuse(280822, 1309830);
}

fn assert_ordinary_loa_expiry_and_reuse(item: u32, spell: u32) {
    const AURA_DURATION_MS: u32 = 1_800_000;
    const RECOVERY_MS: u32 = 3_600_000;
    const BEFORE_EXPIRY_SECONDS: f64 = 1799.0;
    const AFTER_EXPIRY_SECONDS: f64 = 1801.0;
    const BEFORE_RECOVERY_SECONDS: f64 = 3598.0;
    const AFTER_RECOVERY_SECONDS: f64 = 3601.0;
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("loa-ordinary-timers.redb"));
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    let caster = h.caster;
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(caster).copied();
    let position = *h.app.world().get::<Position>(caster).unwrap();
    learn_effect_toy(&mut h, character, item);
    let before_use = crate::auras::aura_now(h.app.world());
    assert_eq!(activate(&mut h, item)[0].error, None, "item{item}");
    let after_use = crate::auras::aura_now(h.app.world());
    assert!((0.0..1.0).contains(&(after_use - before_use)));
    assert_eq!(
        h.inbox.go.iter().filter(|go| go.spell_id == spell).count(),
        1
    );
    let recovery = h
        .inbox
        .cooldowns
        .iter()
        .find(|cooldown| cooldown.spell_id == spell && !cooldown.is_gcd)
        .unwrap();
    assert_eq!(recovery.duration_ms, RECOVERY_MS);

    // Bounds come from the observed cast interval, never an internal deadline.
    advance_ordinary_clock_to(&mut h, before_use + BEFORE_EXPIRY_SECONDS);
    assert!(crate::auras::has_aura(h.app.world(), caster, spell));
    advance_ordinary_clock_to(&mut h, after_use + AFTER_EXPIRY_SECONDS);
    assert!(!crate::auras::has_aura(h.app.world(), caster, spell));
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == spell),
        "natural expiry removes the replicated aura"
    );
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
    assert_eq!(
        activate(&mut h, item)[0].error.as_deref(),
        Some("OnCooldown"),
        "natural aura expiry must not clear one-hour recovery"
    );
    advance_ordinary_clock_to(&mut h, before_use + BEFORE_RECOVERY_SECONDS);
    assert_eq!(
        activate(&mut h, item)[0].error.as_deref(),
        Some("OnCooldown"),
        "ordinary recovery still refuses use just before one hour"
    );
    assert_eq!(
        h.inbox.go.iter().filter(|go| go.spell_id == spell).count(),
        1,
        "both recovery refusals produce no extra SpellGo"
    );
    advance_ordinary_clock_to(&mut h, after_use + AFTER_RECOVERY_SECONDS);
    assert_eq!(activate(&mut h, item)[0].error, None, "item{item} reuses");
    assert_eq!(
        h.inbox.go.iter().filter(|go| go.spell_id == spell).count(),
        2
    );
    let aura = h
        .app
        .world()
        .get::<UnitAuras>(caster)
        .unwrap()
        .auras
        .iter()
        .find(|aura| aura.spell_id == spell)
        .expect("ordinary recovery allows a fresh owner aura");
    assert_eq!(aura.caster, Some(caster.to_bits()));
    assert_eq!(aura.duration_ms, AURA_DURATION_MS);
    assert!((AURA_DURATION_MS - 1000..=AURA_DURATION_MS).contains(&aura.remaining_ms));
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
    assert_eq!(h.app.world().get::<Position>(caster), Some(&position));
}

fn advance_ordinary_clock_to(h: &mut Harness, target_seconds: f64) {
    const FRAME_SECONDS: f64 = 0.05;
    let before = crate::auras::aura_now(h.app.world());
    assert!(target_seconds > before, "ordinary time only moves forward");
    let frames = ((target_seconds - before) / FRAME_SECONDS).ceil() as usize;
    h.advance(frames);
    let after = crate::auras::aura_now(h.app.world());
    assert!((target_seconds..target_seconds + 2.0 * FRAME_SECONDS).contains(&after));
}

fn assert_learned_loa_cosmetic_cancellation(item: u32, spell: u32) {
    const AURA_DURATION_MS: u32 = 1_800_000;
    const RECOVERY_MS: u32 = 3_600_000;
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("loa.redb"));
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    let caster = h.caster;
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(caster).copied();
    let position = *h.app.world().get::<Position>(caster).unwrap();
    learn_effect_toy(&mut h, character, item);
    assert_eq!(activate(&mut h, item)[0].error, None, "item{item}");
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.inbox.go.iter().filter(|go| go.spell_id == spell).count(),
        1
    );
    let aura = h
        .app
        .world()
        .get::<UnitAuras>(caster)
        .unwrap()
        .auras
        .iter()
        .find(|aura| aura.spell_id == spell)
        .expect("successful toy use replicates its cosmetic aura");
    assert_eq!(aura.caster, Some(caster.to_bits()));
    assert_eq!(aura.duration_ms, AURA_DURATION_MS);
    assert!((AURA_DURATION_MS - 1000..=AURA_DURATION_MS).contains(&aura.remaining_ms));
    let recovery = h
        .inbox
        .cooldowns
        .iter()
        .find(|cooldown| cooldown.spell_id == spell && !cooldown.is_gcd)
        .expect("successful toy use publishes source item recovery");
    assert_eq!(recovery.duration_ms, RECOVERY_MS);
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura { spell_id: spell });
    h.advance(5);
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == spell)
    );
    assert_eq!(
        activate(&mut h, item)[0].error.as_deref(),
        Some("OnCooldown"),
        "cancelling a visual aura does not clear one-hour toy recovery"
    );
    assert_eq!(
        h.inbox.go.iter().filter(|go| go.spell_id == spell).count(),
        1
    );
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
    assert_eq!(h.app.world().get::<Position>(caster), Some(&position));
}
