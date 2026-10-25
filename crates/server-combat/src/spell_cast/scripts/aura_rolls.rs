//! Weighted exclusive aura states; native payloads retain all their effects and values.
use super::AfterCastHook;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use crate::class_progression::KnownSpells;
use crate::random::Rng;
use crate::spell_info::SpellInfo;
use bevy::prelude::*;

struct Outcome {
    aura: u32,
    weight: f64,
}
struct RollRule {
    cast: u32,
    outcomes: &'static [Outcome],
    upgrade_spell: u32,
    upgrade_effect: u8,
    upgrade_limit: usize,
    guaranteed_upgrade_aura: u32,
}

const RULES: &[RollRule] = &[
    // SimC76f8118 sc_rogue.cpp7628–7633/7667–7733: exactly one of four
    // states, with pinned55/30/10/5 odds; Sleight precedes Loaded Dice.
    RollRule {
        cast: 1214909,
        outcomes: &[
            Outcome {
                aura: 1214933,
                weight: 55.0,
            },
            Outcome {
                aura: 1214934,
                weight: 30.0,
            },
            Outcome {
                aura: 1214935,
                weight: 10.0,
            },
            Outcome {
                aura: 1214937,
                weight: 5.0,
            },
        ],
        upgrade_spell: 381839,
        upgrade_effect: 0,
        upgrade_limit: 3,
        guaranteed_upgrade_aura: 256171,
    },
];

pub(super) fn hook(parent: u32) -> Option<AfterCastHook> {
    RULES
        .iter()
        .any(|rule| rule.cast == parent)
        .then_some(apply_roll)
}

fn roll_state(world: &World, caster: Entity, rule: &RollRule) -> usize {
    let total: f64 = rule.outcomes.iter().map(|outcome| outcome.weight).sum();
    let roll = crate::random::thread_rng().gen_range(0.0..total);
    let mut cumulative = 0.0;
    let mut state = rule
        .outcomes
        .iter()
        .position(|outcome| {
            cumulative += outcome.weight;
            roll < cumulative
        })
        .expect("weighted aura roll must select a state")
        + 1;
    let upgrade_known = world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(rule.upgrade_spell));
    if state < rule.upgrade_limit && upgrade_known {
        let chance =
            super::script_effect_value(world, caster, rule.upgrade_spell, rule.upgrade_effect);
        if crate::random::thread_rng().gen_range(0.0..100.0) < chance {
            state += 1;
        }
    }
    if crate::auras::has_aura(world, caster, rule.guaranteed_upgrade_aura) {
        state += 1;
    }
    state.min(rule.outcomes.len()) - 1
}

fn read_container_remaining_ms(world: &World, caster: Entity, spell: u32) -> u32 {
    let now = super::super::now(world);
    let container = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == caster && aura.spell_id == spell)
        })
        .expect("native exclusive roll requires its applied container");
    let expiry = container
        .expires_at
        .expect("native roll container must have a deadline");
    ((expiry - now).max(0.0) * 1000.0).round() as u32
}

fn apply_roll(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for rule in RULES.iter().filter(|rule| rule.cast == spell.id) {
        let state = roll_state(world, caster, rule);
        let duration = read_container_remaining_ms(world, caster, spell.id);
        // sc_rogue.cpp7740–7752 replaces old states and consumes Loaded Dice.
        crate::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.caster == caster
                && rule
                    .outcomes
                    .iter()
                    .any(|outcome| outcome.aura == aura.spell_id)
        });
        let payload = rule.outcomes[state].aura;
        crate::auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: payload,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
        assert!(
            crate::auras::set_owned_duration(world, caster, caster, payload, duration),
            "native exclusive roll payload must be applied"
        );
        crate::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.caster == caster && aura.spell_id == rule.guaranteed_upgrade_aura
        });
    }
}
