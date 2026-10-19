//! Native source eligibility through ordinary NPC casts and real player kick transport.
use super::test_harness::*;
use shared::casting::CastState;
use shared::protocol::CombatLogKind;

const COUNTERSPELL: u32 = 2139;
const FIREBALL: u32 = 133;
const FROST_BOMB: u32 = 386781;
const LEY_LINE_SPROUTS: u32 = 374364;
const ERUPTING_FISSURE: u32 = 386660;

fn native_cast(spell: u32) -> Harness {
    let mut h = harness(Vec::new(), &[spell, COUNTERSPELL], 8).unwrap();
    let native = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .unwrap();
    let prevention = native
        .categories
        .as_ref()
        .map_or(0, |category| category.prevention_type);
    assert_eq!(prevention, if spell == FIREBALL { 1 } else { 0 });
    super::creature::creature_cast(h.app.world_mut(), h.npc, spell, Some(h.caster), false).unwrap();
    let cast = h
        .app
        .world()
        .get::<CastState>(h.npc)
        .expect("ordinary native cast bar");
    assert_eq!(cast.spell_id, spell);
    assert!(
        cast.duration > 0.5,
        "real cast remains in progress through kick delivery"
    );
    h
}

fn assert_native_cast_rejects_kick(spell: u32) {
    let mut h = native_cast(spell);
    h.cast_and_deliver(COUNTERSPELL, Some(h.npc));
    let cast = h
        .app
        .world()
        .get::<CastState>(h.npc)
        .expect("source-ineligible native NPC cast must survive a real Counterspell");
    assert_eq!(cast.spell_id, spell);
    assert!(cast.elapsed > 0.0 && cast.elapsed < cast.duration);
    assert!(
        !h.inbox.log.iter().any(|event| {
            event.kind == CombatLogKind::Interrupt && event.extra_spell_id == Some(spell)
        }),
        "no invented interrupt log for an ineligible native cast"
    );
}

#[test]
#[ignore = "RED: native Frost Bomb is incorrectly interruptible"]
fn native_interruptibility_frost_bomb_survives_counterspell() {
    assert_native_cast_rejects_kick(FROST_BOMB);
}

#[test]
#[ignore = "RED: native Sprout parent is incorrectly interruptible"]
fn native_interruptibility_sprout_parent_without_categories_survives_counterspell() {
    assert_native_cast_rejects_kick(LEY_LINE_SPROUTS);
}

#[test]
#[ignore = "RED: native Fissure is incorrectly interruptible"]
fn native_interruptibility_fissure_survives_counterspell() {
    assert_native_cast_rejects_kick(ERUPTING_FISSURE);
}

#[test]
#[ignore = "paired control for native interruptibility RED"]
fn native_interruptibility_silence_prevention_fireball_is_interrupted() {
    let mut h = native_cast(FIREBALL);
    h.cast_and_deliver(COUNTERSPELL, Some(h.npc));
    assert!(h.app.world().get::<CastState>(h.npc).is_none());
    assert!(h.inbox.log.iter().any(|event| {
        event.kind == CombatLogKind::Interrupt
            && event.source == Some(h.caster.to_bits())
            && event.target == Some(h.npc.to_bits())
            && event.spell_id == Some(COUNTERSPELL)
            && event.extra_spell_id == Some(FIREBALL)
    }));
}
