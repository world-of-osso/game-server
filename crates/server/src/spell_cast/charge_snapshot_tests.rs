use super::test_harness::*;

const HEARTHSTONE: u32 = 8690;
const HASTY_HEARTH: u32 = 83944;
const HEARTH_CHARGES: u32 = 2309;

fn hasty_hearth_fixture(member: bool) -> Harness {
    let mut h = harness(Vec::new(), &[HEARTHSTONE, HASTY_HEARTH, 458619], 8).unwrap();
    let mut known = crate::class_progression::KnownSpells::fixture(vec![HEARTHSTONE]);
    if member {
        known.set_guild_grants(&[HASTY_HEARTH]);
    }
    h.app.world_mut().entity_mut(h.caster).insert((
        known,
        crate::persistence::ClientId(1),
        shared::components::CombatStatus(false),
        super::homebind::HomeBind(crate::maps::WorldLocation {
            map_id: 0,
            position: bevy::prelude::Vec3::new(-8949.0, -132.0, 83.0),
            orientation: 1.2,
        }),
    ));
    h.advance(4);
    h
}

fn cast_hearthstone(h: &mut Harness) -> shared::protocol::SpellChargesUpdate {
    h.inbox.charges.clear();
    h.cast(HEARTHSTONE, None);
    h.advance(250);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|event| event.spell_id == HEARTHSTONE));
    h.inbox
        .charges
        .iter()
        .rev()
        .find(|update| update.category == HEARTH_CHARGES)
        .expect("Actual Hearthstone must consume category2309")
        .clone()
}

#[test]
fn guild_hasty_hearth_real_cast_uses_charge2309_and_revocation_restores_new_recovery() {
    // Active8690 charge2309:1 charge/1800000ms;83944 effect453:-900000ms.
    // TC a352b1fa SpellHistory.cpp982–995 adds453 before consuming a charge.
    let mut ordinary = hasty_hearth_fixture(false);
    let baseline = cast_hearthstone(&mut ordinary);
    assert_eq!(
        (baseline.current, baseline.max, baseline.recharge_ms),
        (0, 1, 1_800_000)
    );
    let mut member = hasty_hearth_fixture(true);
    let reduced = cast_hearthstone(&mut member);
    assert_eq!(
        (reduced.current, reduced.max, reduced.recharge_ms),
        (0, 1, 900_000)
    );
    let mut removed = hasty_hearth_fixture(true);
    removed
        .app
        .world_mut()
        .get_mut::<crate::class_progression::KnownSpells>(removed.caster)
        .unwrap()
        .set_guild_grants(&[]);
    removed.advance(4);
    let restored = cast_hearthstone(&mut removed);
    assert_eq!(
        (restored.current, restored.max, restored.recharge_ms),
        (0, 1, 1_800_000)
    );
}

#[test]
fn guild_hasty_hearth_join_does_not_retime_an_already_consumed_charge() {
    // TC SpellAuraEffects.cpp525:453 has no immediate effect. ConsumeCharge
    // stores the recovery at consumption; only148's handler updates old clocks.
    let mut h = hasty_hearth_fixture(false);
    let baseline = cast_hearthstone(&mut h);
    assert_eq!(baseline.recharge_ms, 1_800_000);
    h.inbox.charges.clear();
    h.app
        .world_mut()
        .get_mut::<crate::class_progression::KnownSpells>(h.caster)
        .unwrap()
        .set_guild_grants(&[HASTY_HEARTH]);
    h.advance(4);
    let joined = h
        .inbox
        .charges
        .iter()
        .rev()
        .find(|update| update.category == HEARTH_CHARGES)
        .expect("Membership snapshot must report the pending Hearthstone charge");
    assert!(
        (1_790_000..1_800_000).contains(&joined.remaining_ms),
        "{joined:?}"
    );
}

#[test]
fn guild_hasty_hearth_removal_preserves_consumed_charge_then_restores_next_use() {
    let mut h = hasty_hearth_fixture(true);
    assert_eq!(cast_hearthstone(&mut h).recharge_ms, 900_000);
    h.inbox.charges.clear();
    h.app
        .world_mut()
        .get_mut::<crate::class_progression::KnownSpells>(h.caster)
        .unwrap()
        .set_guild_grants(&[]);
    h.advance(4);
    let removed = h
        .inbox
        .charges
        .iter()
        .rev()
        .find(|update| update.category == HEARTH_CHARGES)
        .expect("Removal snapshot must report the pending Hearthstone charge");
    assert!(
        (890_000..900_000).contains(&removed.remaining_ms),
        "{removed:?}"
    );
    h.advance(18_010);
    assert_eq!(cast_hearthstone(&mut h).recharge_ms, 1_800_000);
}

#[test]
fn guild_hasty_hearth_human_keeps_mixed_consumption_durations_across_guild_changes() {
    // Actual native Human458619 (No Place Like Home):411/+1 charge and453/-300000ms.
    // TC ConsumeCharge records each use's own duration, not one category interval.
    let mut h = hasty_hearth_fixture(false);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            HEARTHSTONE,
            458619,
        ]));
    h.advance(4);
    let first = cast_hearthstone(&mut h);
    assert_eq!(
        (first.current, first.max, first.recharge_ms),
        (1, 2, 1_500_000)
    );
    h.app
        .world_mut()
        .get_mut::<crate::class_progression::KnownSpells>(h.caster)
        .unwrap()
        .set_guild_grants(&[HASTY_HEARTH]);
    h.advance(4);
    let second = cast_hearthstone(&mut h);
    assert_eq!(
        (second.current, second.max, second.recharge_ms),
        (0, 2, 1_500_000)
    );
    h.advance((second.remaining_ms / 50 + 5) as usize);
    let next = h
        .inbox
        .charges
        .iter()
        .rev()
        .find(|update| update.category == HEARTH_CHARGES)
        .expect("First Human charge recovers before the queued guild use");
    assert_eq!((next.current, next.max, next.recharge_ms), (1, 2, 600_000));
    assert!((599_000..=600_000).contains(&next.remaining_ms), "{next:?}");
    h.app
        .world_mut()
        .get_mut::<crate::class_progression::KnownSpells>(h.caster)
        .unwrap()
        .set_guild_grants(&[]);
    h.advance(4);
    let third = cast_hearthstone(&mut h);
    assert_eq!(
        (third.current, third.max, third.recharge_ms),
        (0, 2, 600_000)
    );
    h.advance((third.remaining_ms / 50 + 5) as usize);
    let restored = h
        .inbox
        .charges
        .iter()
        .rev()
        .find(|update| update.category == HEARTH_CHARGES)
        .expect("Queued guild use recovers before the ordinary Human use");
    assert_eq!(
        (restored.current, restored.max, restored.recharge_ms),
        (1, 2, 1_500_000)
    );
}

#[test]
fn cooldown_assist_known_charge_spell_publishes_full_category_before_first_cast() {
    let mut h = harness(Vec::new(), &[108853], 8).unwrap();
    // Character progression supplies ClientId and KnownSpells on world entry.
    // The harness already queued its initial spellbook for passive sync. Supply
    // the persisted identity before that first fixed tick, as progression does.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::persistence::ClientId(1));
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
