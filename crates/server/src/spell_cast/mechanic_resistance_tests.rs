//! TC a352b1fa Unit.cpp2571–2592,2631–2635: physical mechanic resistance.
use super::test_harness::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use bevy::prelude::*;
use shared::protocol::{CombatLogKind, MissKind};

const CHEAP_SHOT: u32 = 1833;
const DUST_CLOUD: u32 = 26072;
const STUN_RESISTANCE: u32 = 40386;
const SILENCE_RESISTANCE: u32 = 26208;

fn resistance_duel() -> Harness {
    let mut h = harness_with(
        Vec::new(),
        &[CHEAP_SHOT, DUST_CLOUD, STUN_RESISTANCE, SILENCE_RESISTANCE],
        1,
        true,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        RemoveReason::Cancelled,
        |aura| [STUN_RESISTANCE, SILENCE_RESISTANCE].contains(&aura.spell_id),
    );
    h.advance(2);
    h
}

fn apply_native_aura(h: &mut Harness, target: Entity, spell_id: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: target,
            target,
            spell_id,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
}

fn cast_dusted_cheap_shot(h: &mut Harness) {
    // Refresh the actual short-lived hit debuff before every trial, not its amount.
    apply_native_aura(h, h.npc, DUST_CLOUD);
    h.advance(2);
    assert_eq!(
        crate::combat::creature_miss_chance(h.app.world().get::<crate::auras::AuraStatMods>(h.npc)),
        90.0,
    );
    h.inbox.log.clear();
    super::creature_cast(h.app.world_mut(), h.npc, CHEAP_SHOT, Some(h.caster), true)
        .expect("native NPC Cheap Shot against actual player peer");
    h.advance(2);
}

fn assert_native_stun_lands(h: &mut Harness) {
    for _ in 0..256 {
        cast_dusted_cheap_shot(h);
        assert!(!h.inbox.log.iter().any(|event| {
            event.spell_id == Some(CHEAP_SHOT)
                && event.kind == CombatLogKind::Miss(MissKind::Resist)
        }));
        if crate::auras::has_aura(h.app.world(), h.caster, CHEAP_SHOT) {
            assert!(
                h.app
                    .world()
                    .get::<ActiveAuras>(h.caster)
                    .unwrap()
                    .is_stunned()
            );
            return;
        }
        assert!(
            h.inbox.log.iter().any(|event| {
                event.spell_id == Some(CHEAP_SHOT)
                    && event.target == Some(h.caster.to_bits())
                    && event.kind == CombatLogKind::Miss(MissKind::Miss)
            }),
            "no actual Cheap Shot target receipt: {:?}",
            h.inbox.log
        );
    }
    panic!("native stun never landed in bounded nonmatching-resistance control");
}

#[test]
fn class_completeness_native_physical_mechanic_resistance_precedes_no_active_defense() {
    let mut h = resistance_duel();
    let victim = h.caster;
    apply_native_aura(&mut h, victim, STUN_RESISTANCE);
    h.advance(2);
    assert_eq!(
        h.app
            .world()
            .get::<ActiveAuras>(victim)
            .unwrap()
            .modifier(117),
        10.0
    );
    let health = h.health(victim);
    let mut resisted = 0;
    for attempt in 0..256 {
        assert_eq!(
            h.app
                .world()
                .get::<ActiveAuras>(victim)
                .unwrap()
                .modifier(117),
            10.0,
            "before trial {attempt}: {:?}",
            h.app.world().get::<ActiveAuras>(victim)
        );
        cast_dusted_cheap_shot(&mut h);
        assert!(
            !crate::auras::has_aura(h.app.world(), victim, CHEAP_SHOT),
            "native 90 miss +10 mechanic resistance admitted Cheap Shot at {attempt}: {:?}; victim auras {:?}; spell {:?}",
            h.inbox.log,
            h.app.world().get::<ActiveAuras>(victim),
            world_db().unwrap().store.get(CHEAP_SHOT)
        );
        assert!(
            !h.app
                .world()
                .get::<ActiveAuras>(victim)
                .unwrap()
                .is_stunned()
        );
        assert_eq!(h.health(victim), health);
        let outcome = h
            .inbox
            .log
            .iter()
            .find(|event| {
                event.spell_id == Some(CHEAP_SHOT)
                    && event.target == Some(victim.to_bits())
                    && matches!(
                        event.kind,
                        CombatLogKind::Miss(MissKind::Miss | MissKind::Resist)
                    )
            })
            .expect("actual target owner receives miss/resist");
        resisted += usize::from(outcome.kind == CombatLogKind::Miss(MissKind::Resist));
    }
    assert!(
        resisted > 0,
        "bounded trials must exercise the resistance bucket"
    );
    crate::auras::remove_auras(h.app.world_mut(), victim, RemoveReason::Cancelled, |aura| {
        aura.spell_id == STUN_RESISTANCE
    });
    h.advance(2);
    assert_native_stun_lands(&mut h);
}

#[test]
fn class_completeness_native_physical_mechanic_resistance_is_recipient_owned() {
    let mut h = resistance_duel();
    let foreign = spawn_player(&mut h.app, 1, 1.0);
    apply_native_aura(&mut h, foreign, STUN_RESISTANCE);
    h.advance(2);
    assert_native_stun_lands(&mut h);
    assert!(crate::auras::has_aura(
        h.app.world(),
        foreign,
        STUN_RESISTANCE
    ));
}

#[test]
fn class_completeness_native_physical_mechanic_resistance_rejects_silence_selector() {
    let mut h = resistance_duel();
    let victim = h.caster;
    apply_native_aura(&mut h, victim, SILENCE_RESISTANCE);
    h.advance(2);
    assert_native_stun_lands(&mut h);
    assert!(crate::auras::has_aura(
        h.app.world(),
        victim,
        SILENCE_RESISTANCE
    ));
}
