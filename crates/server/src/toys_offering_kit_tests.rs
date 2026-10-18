//! Source279: actual learned item creation and ordinary item recovery.
use super::*;

fn load_offering_kit_count(h: &Harness, character: u64) -> u32 {
    let db = &h.app.world().resource::<GameDatabase>().db;
    persistence::load_inventory(db, character)
        .items
        .iter()
        .filter(|item| item.item_id == 187342)
        .map(|item| item.stack_count)
        .sum()
}

#[test]
fn toyfx279_offering_kit_maker_creates_one_persisted_kit_and_preserves_recovery() {
    const TOY: u32 = 187344;
    const SPELL: u32 = 357020;
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("offering.redb"));
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::merchant::load_vendor_catalog(&conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::persistence::ClientId(character));
    learn_effect_toy(&mut h, character, TOY);
    assert_eq!(load_offering_kit_count(&h, character), 0);
    assert_eq!(activate(&mut h, TOY)[0].error, None);
    h.advance(20); // Only one second: the authored two-second cast is pending.
    assert_eq!(load_offering_kit_count(&h, character), 0);
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == SPELL));
    h.advance(30);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == SPELL));
    assert_eq!(load_offering_kit_count(&h, character), 1);
    let cooldown = h
        .inbox
        .cooldowns
        .iter()
        .find(|cooldown| cooldown.spell_id == SPELL && !cooldown.is_gcd)
        .unwrap();
    assert_eq!(cooldown.duration_ms, 30_000);
    assert_eq!(
        activate(&mut h, TOY)[0].error.as_deref(),
        Some("OnCooldown")
    );
    h.advance(500); // Ordinary 25 seconds, still inside source item recovery.
    assert_eq!(
        activate(&mut h, TOY)[0].error.as_deref(),
        Some("OnCooldown")
    );
    assert_eq!(load_offering_kit_count(&h, character), 1);
    h.advance(140); // Ordinary seven seconds, beyond the original recovery.
    assert_eq!(activate(&mut h, TOY)[0].error, None);
    h.advance(50);
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
    assert_eq!(load_offering_kit_count(&h, character), 2);
}
