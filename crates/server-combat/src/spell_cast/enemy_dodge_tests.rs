//! TC a352b1fa Unit.cpp2775–2785: signed attacker251 before expertise.
use super::test_harness::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use crate::stats::CharacterStats;
use bevy::prelude::*;
use shared::components::{Npc, Rotation, UnitFactionTemplate};
use shared::protocol::{CombatLogKind, MissKind};

const STRIKE: u32 = 38598;
const DUST_CLOUD: u32 = 26072;
const RISKY_FEINT: u32 = 48250;
const WORD_OF_SHA: u32 = 110294;
const MOD_ENEMY_DODGE: u32 = 251;

fn enemy_dodge_duel() -> Harness {
    let mut h = harness_with(
        Vec::new(),
        &[STRIKE, DUST_CLOUD, RISKY_FEINT, WORD_OF_SHA],
        4,
        true,
    )
    .unwrap();
    let native = load_native_creature_world(&[40]);
    let template = &native.templates[&40];
    h.app.world_mut().entity_mut(h.npc).insert((
        Npc {
            template_id: 40,
            name: template.name.clone(),
        },
        UnitFactionTemplate(u32::from(template.faction)),
    ));
    h.app.insert_resource(native);
    h.app.world_mut().entity_mut(h.caster).insert(Rotation {
        x: 0.0,
        y: std::f32::consts::FRAC_PI_2,
        z: 0.0,
    });
    h.advance(2);
    assert!(super::targets::is_hostile(h.app.world(), h.npc, h.caster));
    assert_eq!(
        h.app
            .world()
            .get::<CharacterStats>(h.caster)
            .unwrap()
            .defense
            .dodge,
        0.0
    );
    h
}

fn apply_native_effect(h: &mut Harness, target: Entity, spell_id: u32, effect: u8) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: target,
            target,
            spell_id,
            effect_mask: 1 << effect,
            source: AuraSource::Cast,
        },
    );
}

fn cast_dusted_strike(h: &mut Harness) {
    apply_native_effect(h, h.npc, DUST_CLOUD, 0);
    h.advance(2);
    assert!(super::targets::is_hostile(h.app.world(), h.npc, h.caster));
    assert_eq!(
        crate::combat::creature_miss_chance(h.app.world().get::<crate::auras::AuraStatMods>(h.npc)),
        90.0
    );
    h.inbox.log.clear();
    super::creature_cast(h.app.world_mut(), h.npc, STRIKE, Some(h.caster), true).unwrap();
    h.advance(2);
}

fn assert_native_strike_lands(h: &mut Harness, refresh: &[(Entity, u32, u8)]) {
    let before = h.health(h.caster);
    for _ in 0..256 {
        for &(target, spell, effect) in refresh {
            apply_native_effect(h, target, spell, effect);
        }
        cast_dusted_strike(h);
        let hits: Vec<_> = h
            .inbox
            .log
            .iter()
            .filter(|event| {
                event.spell_id == Some(STRIKE)
                    && event.target == Some(h.caster.to_bits())
                    && event.kind == CombatLogKind::Damage
            })
            .collect();
        if let Some(hit) = hits.first() {
            assert_eq!(hits.len(), 1);
            assert!(hit.amount > 0);
            assert_eq!(before - h.health(h.caster), hit.amount as f32);
            return;
        }
        assert!(
            h.inbox.log.iter().any(|event| {
                event.spell_id == Some(STRIKE)
                    && event.target == Some(h.caster.to_bits())
                    && event.kind == CombatLogKind::Miss(MissKind::Miss)
            }),
            "expected actual native miss while awaiting damage: {:?}",
            h.inbox.log
        );
    }
    panic!("native Strike never landed without positive attacker dodge modifier");
}

#[test]
fn class_completeness_native_enemy_dodge_uses_caster_modifier_until_removed() {
    let mut h = enemy_dodge_duel();
    let before = h.health(h.caster);
    let mut dodged = 0;
    for attempt in 0..256 {
        // Native effect2 only: isolate251 without certifying Risky Feint's other effects.
        let npc = h.npc;
        apply_native_effect(&mut h, npc, RISKY_FEINT, 2);
        cast_dusted_strike(&mut h);
        assert_eq!(
            h.app
                .world()
                .get::<ActiveAuras>(npc)
                .unwrap()
                .modifier(MOD_ENEMY_DODGE),
            20.0
        );
        assert_eq!(
            h.health(h.caster),
            before,
            "90miss+20dodge admitted native damage at{attempt}: {:?}",
            h.inbox.log
        );
        let outcome = h
            .inbox
            .log
            .iter()
            .find(|event| {
                event.spell_id == Some(STRIKE)
                    && event.target == Some(h.caster.to_bits())
                    && matches!(
                        event.kind,
                        CombatLogKind::Miss(MissKind::Miss | MissKind::Dodge)
                    )
            })
            .expect("actual player owner receives native miss/dodge");
        dodged += usize::from(outcome.kind == CombatLogKind::Miss(MissKind::Dodge));
    }
    assert!(dodged > 0);
    let npc = h.npc;
    crate::auras::remove_auras(h.app.world_mut(), npc, RemoveReason::Cancelled, |aura| {
        aura.spell_id == RISKY_FEINT
    });
    h.advance(2);
    assert_native_strike_lands(&mut h, &[]);
}

#[test]
fn class_completeness_native_enemy_dodge_sums_signed_caster_effects_before_clamp() {
    let mut h = enemy_dodge_duel();
    let npc = h.npc;
    apply_native_effect(&mut h, npc, RISKY_FEINT, 2);
    apply_native_effect(&mut h, npc, WORD_OF_SHA, 0);
    h.advance(2);
    assert_eq!(
        h.app
            .world()
            .get::<ActiveAuras>(npc)
            .unwrap()
            .modifier(MOD_ENEMY_DODGE),
        -5.0
    );
    assert_native_strike_lands(&mut h, &[(npc, RISKY_FEINT, 2), (npc, WORD_OF_SHA, 0)]);
}

#[test]
fn class_completeness_native_enemy_dodge_on_recipient_does_not_modify_attacker() {
    let mut h = enemy_dodge_duel();
    let victim = h.caster;
    apply_native_effect(&mut h, victim, RISKY_FEINT, 2);
    h.advance(2);
    assert_native_strike_lands(&mut h, &[(victim, RISKY_FEINT, 2)]);
}
