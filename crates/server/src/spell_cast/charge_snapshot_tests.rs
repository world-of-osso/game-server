use super::test_harness::*;
use bevy::prelude::DetectChangesMut;

#[test]
fn cooldown_assist_known_charge_spell_publishes_full_category_before_first_cast() {
    let mut h = harness(Vec::new(), &[108853], 8).unwrap();
    // Character progression supplies ClientId and KnownSpells on world entry.
    // Publish only after that spellbook's passives have been synchronized.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::persistence::ClientId(1));
    h.app
        .world_mut()
        .get_mut::<crate::class_progression::KnownSpells>(h.caster)
        .unwrap()
        .set_changed();
    h.advance(4);
    let update = h
        .inbox
        .charges
        .iter()
        .find(|update| update.category == 1500)
        .unwrap_or_else(|| panic!("initial Fire Blast charge message absent; categories={:?}; history={:?}; received={:?}", h.app.world().resource::<crate::spell_info::SpellStore>().get(108853).unwrap().categories, h.app.world().get::<super::SpellHistory>(h.caster), h.inbox.charges));
    assert_eq!((update.current, update.max, update.remaining_ms), (1, 1, 0));
    assert!(update.recharge_ms > 0);
    assert!(h.inbox.go.is_empty(), "no cast priming was used");
    assert_eq!(
        h.inbox.charges.len(),
        1,
        "one initial snapshot, not tick updates"
    );
    h.inbox.charges.clear();
    h.advance(4);
    assert!(h.inbox.charges.is_empty(), "idle frames send no updates");
}

#[test]
fn cooldown_assist_spellbook_snapshot_deduplicates_shared_category_after_passives() {
    let mut h = harness(Vec::new(), &[103827, 100, 198304], 1).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::persistence::ClientId(1));
    h.app
        .world_mut()
        .get_mut::<crate::class_progression::KnownSpells>(h.caster)
        .unwrap()
        .set_changed();
    h.advance(4);
    let charges: Vec<_> = h
        .inbox
        .charges
        .iter()
        .filter(|c| c.category == 1386)
        .collect();
    assert_eq!(charges.len(), 1);
    assert_eq!(
        (
            charges[0].current,
            charges[0].max,
            charges[0].recharge_ms,
            charges[0].remaining_ms
        ),
        (2, 2, 17_000, 0)
    );
}

#[test]
fn cooldown_assist_restoring_the_last_charge_clears_remaining_recovery() {
    let mut h = harness(Vec::new(), &[108853], 8).unwrap();
    h.cast_and_deliver(108853, Some(h.npc));
    h.advance(4);
    h.inbox.charges.clear();
    super::restore_charges(h.app.world_mut(), h.caster, 1500, 1);
    h.advance(1);
    assert_eq!(h.inbox.charges.len(), 1);
    let restored = &h.inbox.charges[0];
    assert_eq!(
        (restored.current, restored.max, restored.remaining_ms),
        (1, 1, 0)
    );
}
