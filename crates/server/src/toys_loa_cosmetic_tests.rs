//! Build70448 stationary Loa cosmetics: actual learning, owner aura and cancellation.
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
