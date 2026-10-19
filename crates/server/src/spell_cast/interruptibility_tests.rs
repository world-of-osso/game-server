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
    let mut h = harness(Vec::new(), &[spell, COUNTERSPELL, 104773, 387152], 8).unwrap();
    // Reuse the harness Mage's actual class resources for the native Fireball control.
    // The bare target NPC has no mana pool; do not replace native costs or grant amounts.
    let stats = h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .clone();
    let mana = *h
        .app
        .world()
        .get::<shared::components::Mana>(h.caster)
        .unwrap();
    let pools = h
        .app
        .world()
        .get::<super::powers::PowerPools>(h.caster)
        .unwrap()
        .clone();
    let level = *h
        .app
        .world()
        .get::<shared::components::Level>(h.caster)
        .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert((stats, mana, pools, level));
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
fn native_interruptibility_live_immunity_changes_after_normal_cast_starts() {
    let mut h = native_cast(FIREBALL);
    let enemy = h.npc;
    assert!(super::can_interrupt_cast(h.app.world(), h.caster, enemy));
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(104773)
        .unwrap();
    let effect_mask = crate::auras::aura_effect_mask(spell);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: enemy,
            target: enemy,
            spell_id: 104773,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(!super::can_interrupt_cast(h.app.world(), h.caster, enemy));
    h.cast_and_deliver(COUNTERSPELL, Some(enemy));
    assert!(h.app.world().get::<CastState>(enemy).is_some());
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt)
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        enemy,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 104773,
    );
    assert!(super::can_interrupt_cast(h.app.world(), h.caster, enemy));
    // Native triggered kick boundary ignores the first kick's cooldown, not eligibility.
    super::creature::creature_cast(h.app.world_mut(), h.caster, COUNTERSPELL, Some(enemy), true)
        .unwrap();
    assert!(h.app.world().get::<CastState>(enemy).is_none());
    h.advance(3);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt
                && event.extra_spell_id == Some(FIREBALL))
    );
}

#[test]
fn native_interruptibility_icy_devastator_channel_survives_counterspell() {
    let mut h = native_cast(387151);
    h.advance(42);
    let cast = h
        .app
        .world()
        .get::<CastState>(h.npc)
        .expect("native Icy channel after cast time");
    assert_eq!(cast.cast_type, shared::casting::CastType::Channel);
    h.cast_and_deliver(COUNTERSPELL, Some(h.npc));
    assert!(h.app.world().get::<CastState>(h.npc).is_some_and(
        |cast| cast.spell_id == 387151 && cast.cast_type == shared::casting::CastType::Channel
    ));
    assert!(!h.inbox.log.iter().any(
        |event| event.kind == CombatLogKind::Interrupt && event.extra_spell_id == Some(387151)
    ));
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
