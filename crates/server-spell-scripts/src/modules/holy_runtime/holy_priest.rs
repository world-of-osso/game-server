//! Native70448 Holy scripts. Numeric values remain in the local spell catalog.
use super::combat;
use bevy::prelude::*;
use combat::random as rand;

use combat::auras::procs::AuraEvent;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::{ActiveAuras, AuraInstance, has_aura};
use combat::spell_info::{SpellInfo, SpellStore};

const ULTIMATE_SERENITY: u32 = 1_246_517;
const SERENITY_SPLASH: u32 = 1_250_581;
const BENEDICTION_TALENT: u32 = 1_262_755;
const BENEDICTION_SERENITY: u32 = 1_262_760;
const BENEDICTION_BUFF: u32 = 1_262_766;
const BENEDICTION_HEAL: u32 = 1_262_763;
const COSMIC_RIPPLE: u32 = 243_241;

pub(super) fn serenity_hit(
    spell: u32,
) -> Option<combat::spell_cast::script_registry::ContextHitHook> {
    match spell {
        2050 => Some(cast_serenity_splash),
        BENEDICTION_HEAL => Some(cast_benediction_ripple),
        _ => None,
    }
}

fn cast_serenity_splash(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx,
    target: Entity,
) {
    if has_aura(world, ctx.caster, BENEDICTION_SERENITY)
        && rand::random::<f32>() * 100.0
            < super::script_effect_value(world, ctx.caster, BENEDICTION_SERENITY, 1)
    {
        combat::spell_cast::cast_triggered(
            world,
            ctx.caster,
            BENEDICTION_BUFF,
            Some(ctx.caster),
            ctx.depth + 1,
        );
    }
    if has_aura(world, ctx.caster, ULTIMATE_SERENITY) {
        combat::spell_cast::cast_triggered(
            world,
            ctx.caster,
            SERENITY_SPLASH,
            Some(target),
            ctx.depth + 1,
        );
    }
}

fn cast_benediction_ripple(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx,
    _target: Entity,
) {
    combat::spell_cast::cast_triggered(
        world,
        ctx.caster,
        COSMIC_RIPPLE,
        Some(ctx.caster),
        ctx.depth + 1,
    );
}

pub(super) fn spend_benediction(world: &mut World, priest: Entity) {
    let instance = world
        .get::<ActiveAuras>(priest)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == BENEDICTION_BUFF && aura.caster == priest)
        })
        .map(|aura| aura.instance_id);
    if let Some(instance) = instance {
        combat::auras::spend_stack(world, priest, instance);
    }
}

pub(super) fn benediction_from_mending(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx,
    _target: Entity,
    _raw: f32,
    _effective: f32,
) {
    if ctx.spell.id == 33_110
        && has_aura(world, ctx.caster, BENEDICTION_TALENT)
        && combat::auras::procs::try_native_policy_proc(world, ctx.caster, BENEDICTION_TALENT)
    {
        combat::spell_cast::cast_triggered(
            world,
            ctx.caster,
            BENEDICTION_BUFF,
            Some(ctx.caster),
            ctx.depth + 1,
        );
    }
}

pub(super) fn hymn_tick(
    spell: u32,
    index: u8,
) -> Option<combat::spell_cast::script_registry::ContextHitHook> {
    ((spell, index) == (64_843, 0)).then_some(cast_hymn_tick)
}

fn cast_hymn_tick(world: &mut World, ctx: &combat::spell_cast::effects::EffectCtx, target: Entity) {
    combat::spell_cast::cast_triggered(world, ctx.caster, 64_844, Some(target), ctx.depth + 1);
    if !has_aura(world, ctx.caster, BENEDICTION_SERENITY) {
        return;
    }
    let scale = super::script_effect_value(world, ctx.caster, BENEDICTION_SERENITY, 0) / 100.0;
    let Some(mut spell) =
        combat::spell_cast::load_triggered_spell(world, COSMIC_RIPPLE, ctx.depth + 1)
    else {
        return;
    };
    for effect in &mut spell.effects {
        effect.base_points *= scale;
        effect.bonus_coefficient *= scale;
        effect.bonus_coefficient_from_ap *= scale;
    }
    combat::spell_cast::launch_triggered_spell(
        world,
        ctx.caster,
        &spell,
        Some(ctx.caster),
        ctx.depth + 1,
    );
}

pub(super) fn serenity_area_filter(spell: u32, index: u8) -> Option<super::AreaTargetFilter> {
    ((spell, index) == (SERENITY_SPLASH, 0)).then_some(select_serenity_allies)
}

fn select_serenity_allies(
    world: &World,
    spell: &SpellInfo,
    primary: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    use shared::components::Health;
    units.retain(|unit| {
        Some(*unit) != primary
            && world
                .get::<Health>(*unit)
                .is_some_and(|h| h.current > 0.0 && h.current < h.max)
    });
    units.sort_by(|a, b| {
        let a = world.get::<Health>(*a).unwrap();
        let b = world.get::<Health>(*b).unwrap();
        (a.current / a.max).total_cmp(&(b.current / b.max))
    });
    units.truncate(spell.effects[1].base_points.max(0.0) as usize);
}

pub(super) fn piety(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx,
    _target: Entity,
    raw: f32,
    effective: f32,
) {
    use combat::spell_cast::{
        effects,
        targets::{self, Relation},
    };
    use shared::components::Health;
    const TALENT: u32 = 1_246_802;
    const HEAL: u32 = 443_126;
    if ctx.spell.id == HEAL || raw <= effective || !has_aura(world, ctx.caster, TALENT) {
        return;
    }
    let spell = world
        .resource::<SpellStore>()
        .get(HEAL)
        .expect("Piety heal missing")
        .clone();
    let Some(center) = targets::position(world, ctx.caster) else {
        return;
    };
    let mut allies = targets::area_units(
        world,
        ctx.caster,
        center,
        targets::radius(&spell.effects[0], 1),
        Relation::Ally,
        0,
    );
    allies.retain(|unit| {
        world
            .get::<Health>(*unit)
            .is_some_and(|h| h.current > 0.0 && h.current < h.max)
    });
    allies.sort_by(|a, b| {
        let a = world.get::<Health>(*a).unwrap();
        let b = world.get::<Health>(*b).unwrap();
        (a.current / a.max).total_cmp(&(b.current / b.max))
    });
    let count = super::script_effect_value(world, ctx.caster, TALENT, 1).max(0.0) as usize;
    allies.truncate(count);
    if allies.is_empty() {
        return;
    }
    let percent = super::script_effect_value(world, ctx.caster, TALENT, 0) / 100.0;
    let amount = (raw - effective) * percent / allies.len() as f32;
    let heal = effects::EffectCtx {
        caster: ctx.caster,
        spell: &spell,
        depth: ctx.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    for ally in allies {
        effects::apply_fixed_heal(world, &heal, ally, amount);
    }
}

pub(crate) const APOTHEOSIS: u32 = 200183;
const SURGE_TALENT: u32 = 109186;
const SURGE_BUFF: u32 = 114255;
const SMITE: u32 = 585;
const MIRACLE_WORKER: u32 = 235587;
const HOLY_WORDS: &[u32] = &[2050, 34861, 88625];

pub(crate) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        SURGE_TALENT,
        AuraScript {
            check_proc: Some(roll_surge_from_native_spell),
            on_proc: Some(grant_surge),
            ..Default::default()
        },
    );
}

fn roll_surge_from_native_spell(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    if event.actor != aura.caster {
        return false;
    }
    let eligible = event.spell_id.is_some_and(|id| {
        id == SMITE
            || world.resource::<SpellStore>().get(id).is_some_and(|spell| {
                spell
                    .effects
                    .iter()
                    .any(|effect| matches!(effect.effect, 10 | 67 | 136))
            })
    });
    if !eligible {
        return false;
    }
    let chance = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native109186 retains its chance effect")
        .amount;
    rand::random::<f32>() * 100.0 < chance
}

fn grant_surge(world: &mut World, priest: Entity, _aura: &AuraInstance, _other: Option<Entity>) {
    #[cfg(any(test, feature = "test-support"))]
    let before = read_surge_stacks(world, priest);
    combat::spell_cast::cast_triggered(world, priest, SURGE_BUFF, Some(priest), 1);
    #[cfg(any(test, feature = "test-support"))]
    {
        let after = read_surge_stacks(world, priest);
        combat::spell_cast::holy_metrics::record_surge_gain(world, priest, before, after);
    }
}

#[cfg(any(test, feature = "test-support"))]
fn read_surge_stacks(world: &World, priest: Entity) -> u8 {
    world
        .get::<ActiveAuras>(priest)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == SURGE_BUFF && aura.caster == priest)
        })
        .map_or(0, |aura| aura.stacks)
}

/// 200183: with235587 gain one charge; otherwise reset the Holy Words.
/// Unknown Words have never spent a charge, so ordinary restore is a no-op.
pub(super) fn restore_holy_words(world: &mut World, priest: Entity, _spell: &SpellInfo) {
    let count = if has_aura(world, priest, MIRACLE_WORKER) {
        1
    } else {
        u8::MAX
    };
    let categories: Vec<_> = HOLY_WORDS
        .iter()
        .filter_map(|id| {
            world
                .resource::<SpellStore>()
                .get(*id)?
                .categories
                .as_ref()
                .map(|category| category.charge_category)
        })
        .collect();
    for category in categories {
        combat::spell_cast::restore_charges(world, priest, category, count);
    }
}

/// Native200183s1 amplifies reductions, not the passage of recharge time.
pub(super) fn holy_word_reduction(world: &World, priest: Entity, seconds: f32) -> f64 {
    let bonus = world
        .get::<ActiveAuras>(priest)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == APOTHEOSIS && aura.caster == priest)
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount);
    f64::from(seconds * (1.0 + bonus / 100.0))
}

/// A successful Flash consumes one existing native use before queued heal events
/// can grant another proc. Failed/cancelled casts never reach AfterCast.
pub(super) fn spend_surge(world: &mut World, priest: Entity) {
    let instance = world
        .get::<ActiveAuras>(priest)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == SURGE_BUFF && aura.caster == priest)
        })
        .map(|aura| aura.instance_id);
    if let Some(instance) = instance {
        combat::auras::spend_stack(world, priest, instance);
        #[cfg(any(test, feature = "test-support"))]
        combat::spell_cast::holy_metrics::record_surge_use(world, priest);
    }
}

/// Native390994s2 names Prayer of Mending casts, not its damage-triggered bounces.
pub(super) fn reduce_word_on_mending(world: &mut World, priest: Entity, _spell: &SpellInfo) {
    const VOICE_OF_HARMONY: u32 = 390994;
    const SERENITY: u32 = 2050;
    if !has_aura(world, priest, VOICE_OF_HARMONY) {
        return;
    }
    let seconds = super::script_effect_value(world, priest, VOICE_OF_HARMONY, 1);
    let seconds = holy_word_reduction(world, priest, seconds);
    super::reduce_cooldown(world, priest, SERENITY, seconds);
}
