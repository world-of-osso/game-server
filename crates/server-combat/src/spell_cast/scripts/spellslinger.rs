//! Shared Spellslinger conjures; native70448 owns payloads and talent amounts.
use super::{AfterCastHook, script_effect_value};
use crate::class_progression::TraitLoadout;
use crate::spell_cast::{effects::EffectCtx, queued_triggers, targets};
use crate::spell_info::SpellInfo;
use bevy::prelude::*;
use shared::components::ActiveSpec;

const SORCERY: u32 = 443739;
const SPLINTERSTORM: u32 = 443783;
const STORM_WINDOW: u32 = 1247908;
const SIGNATURE: u32 = 470021;
// SimC76f8118 sc_mage.cpp7340–7359: stagger individual launched Splinters.
const CONJURE_SPACING_MS: u32 = 100;

#[derive(Clone, Copy)]
struct Specialization {
    spec: u32,
    payload: u32,
    surge: Option<u32>,
    orb: u32,
    cooldown_effect: u8,
}
const SPECIALIZATIONS: &[Specialization] = &[
    Specialization {
        spec: 62,
        payload: 443763,
        surge: Some(365362),
        orb: 153626,
        cooldown_effect: 0,
    },
    Specialization {
        spec: 64,
        payload: 443722,
        surge: None,
        orb: 84714,
        cooldown_effect: 1,
    },
];

#[derive(Clone, Copy)]
enum Count {
    One,
    Talent(u32, u8),
    OnePlusTalent(u32, u8),
}
struct CastSource {
    spec: u32,
    spell: u32,
    count: Count,
}
// sc_mage.cpp2861/4017/4087/4284–4285/5185: successful source executions.
const CAST_SOURCES: &[CastSource] = &[
    CastSource {
        spec: 62,
        spell: 30451,
        count: Count::One,
    },
    CastSource {
        spec: 62,
        spell: 1295924,
        count: Count::Talent(SIGNATURE, 0),
    },
    CastSource {
        spec: 64,
        spell: 116,
        count: Count::One,
    },
    CastSource {
        spec: 64,
        spell: 44614,
        count: Count::One,
    },
    CastSource {
        spec: 64,
        spell: 199786,
        count: Count::OnePlusTalent(SIGNATURE, 1),
    },
];

fn selected(world: &World, caster: Entity, spell: u32) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == spell)
    })
}

fn specialization(world: &World, caster: Entity) -> Option<Specialization> {
    let spec = world.get::<ActiveSpec>(caster)?.0;
    SPECIALIZATIONS.iter().find(|row| row.spec == spec).copied()
}

fn talent_count(world: &World, caster: Entity, talent: u32, effect: u8) -> u32 {
    if selected(world, caster, talent) {
        script_effect_value(world, caster, talent, effect).max(0.0) as u32
    } else {
        0
    }
}

fn read_count(world: &World, caster: Entity, count: Count) -> u32 {
    match count {
        Count::One => 1,
        Count::Talent(talent, effect) => talent_count(world, caster, talent, effect),
        Count::OnePlusTalent(talent, effect) => 1 + talent_count(world, caster, talent, effect),
    }
}

pub(super) fn after_cast_hook(spell: u32) -> Option<AfterCastHook> {
    CAST_SOURCES
        .iter()
        .any(|row| row.spell == spell)
        .then_some(after_cast)
}

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let Some(spec) = specialization(world, caster) else {
        return;
    };
    let Some(source) = CAST_SOURCES
        .iter()
        .find(|row| row.spec == spec.spec && row.spell == spell.id)
    else {
        return;
    };
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    let count = read_count(world, caster, source.count);
    let ctx = EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    conjure(world, &ctx, target, spec, count);
}

fn double_chance(world: &World, caster: Entity, spec: Specialization) -> f64 {
    let window = crate::auras::has_aura(world, caster, STORM_WINDOW)
        || spec
            .surge
            .is_some_and(|aura| crate::auras::has_aura(world, caster, aura));
    if window && selected(world, caster, SPLINTERSTORM) {
        f64::from(script_effect_value(world, caster, SPLINTERSTORM, 1)) / 100.0
    } else {
        0.0
    }
}

fn conjure(world: &mut World, ctx: &EffectCtx, target: Entity, spec: Specialization, count: u32) {
    // sc_mage.cpp7326–7335: only selected Spellslingers and living enemies.
    let valid_target = targets::is_alive(world, target)
        && targets::same_map(world, ctx.caster, target)
        && targets::relation_matches(world, ctx.caster, target, targets::Relation::Enemy);
    if !selected(world, ctx.caster, SORCERY) || !valid_target {
        return;
    }
    let chance = double_chance(world, ctx.caster, spec);
    let mut delay = 0;
    for _ in 0..count {
        // sc_mage.cpp7354–7360: roll independently for each conjure, not each cast.
        let doubled = chance >= 1.0 || (chance > 0.0 && crate::random::random::<f64>() < chance);
        for _ in 0..if doubled { 2 } else { 1 } {
            queued_triggers::schedule_full_trigger(world, ctx, target, spec.payload, delay);
            delay += CONJURE_SPACING_MS;
        }
    }
}

pub(super) fn after_direct_damage(world: &mut World, ctx: &EffectCtx) {
    let Some(spec) = specialization(world, ctx.caster) else {
        return;
    };
    const SPELLFROST_TEACHINGS: u32 = 444986;
    if ctx.spell.id != spec.payload || !selected(world, ctx.caster, SPELLFROST_TEACHINGS) {
        return;
    }
    // SimC76f8118 sc_mage.cpp5404–5407: a landed Splinter adjusts the
    // specialization's actual Orb recharge. Native effects carry milliseconds.
    let milliseconds = script_effect_value(
        world,
        ctx.caster,
        SPELLFROST_TEACHINGS,
        spec.cooldown_effect,
    );
    super::reduce_cooldown(
        world,
        ctx.caster,
        spec.orb,
        f64::from(milliseconds) / 1000.0,
    );
}

struct PeriodicSource {
    spec: u32,
    spell: u32,
    count: Count,
    window: (u32, u32),
}
// SimC76f8118 sc_mage.cpp4992–4996/5021: each real Ray tick conjures
// the native Splinterstorm amount before opening/refreshing its window.
const PERIODIC_SOURCES: &[PeriodicSource] = &[PeriodicSource {
    spec: 64,
    spell: 205021,
    count: Count::Talent(SPLINTERSTORM, 2),
    window: (SPLINTERSTORM, STORM_WINDOW),
}];

pub(super) fn after_periodic_damage(world: &mut World, ctx: &EffectCtx, target: Entity) {
    let Some(spec) = specialization(world, ctx.caster) else {
        return;
    };
    let Some(source) = PERIODIC_SOURCES
        .iter()
        .find(|row| row.spec == spec.spec && row.spell == ctx.spell.id)
    else {
        return;
    };
    let count = read_count(world, ctx.caster, source.count);
    conjure(world, ctx, target, spec, count);
    let (talent, aura) = source.window;
    if selected(world, ctx.caster, talent) {
        crate::auras::apply_aura(
            world,
            &crate::auras::AuraApplication {
                caster: ctx.caster,
                target: ctx.caster,
                spell_id: aura,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
}
