//! SimC76f8118 Ret damage relationships; native70448 owns every amount.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use crate::auras::{self, ActiveAuras, RemoveReason, spell_mods};
use crate::class_progression::{KnownSpells, TraitLoadout};
use crate::spell_cast::{self, PlannedEffect, TargetPlan, effects::EffectCtx, targets};
use crate::spell_info::{CastDamageRole, SpellInfo, SpellStore};

const BLADE: u32 = 184575;
const SENTENCE: u32 = 343527;
const SENTENCE_INIT: u32 = 1260251;
const MASTERY: u32 = 267316;
const HIDDEN_MASTERY: u32 = 449198;
const HIGHLORDS_WRATH: u32 = 404512;
const LIGHT_WITHIN: u32 = 1261160;
const LIGHT_WITHIN_TALENT: u32 = 1261159;
const STORM: u32 = 53385;
const TEMPEST: u32 = 224239;
const TEMPEST_TALENT: u32 = 383396;
const TRUTHS_WAKE: u32 = 403695;
const BURN_TO_ASH: u32 = 446663;
const SUN_SEAR_TALENT: u32 = 431413;
const AVATAR_TALENT: u32 = 431425;

fn is_ret(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 70)
}

fn selected(world: &World, caster: Entity, talent: u32) -> bool {
    is_ret(world, caster)
        && world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == talent)
        })
}

/// Called after admission, before native charged modifiers are consumed.
/// Ret393–430 snapshots Art of War/Righteous Cause before Blade executes.
pub(in crate::spell_cast) fn before_cast(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    depth: u8,
) {
    if !is_ret(world, caster) || spell.cast_damage_role == CastDamageRole::Secondary {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    if spell.id == BLADE {
        schedule_light_within(world, caster, spell, target, depth);
        cast_consecrated_blade(world, caster);
    }
    if spell.id == STORM && selected(world, caster, TEMPEST_TALENT) {
        queue_tempest(world, caster, spell, target, depth);
    }
}

fn schedule_light_within(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    target: Entity,
    depth: u8,
) {
    let buff_up = auras::has_aura(world, caster, 406086) || auras::has_aura(world, caster, 402916);
    if !selected(world, caster, LIGHT_WITHIN_TALENT) || !buff_up {
        return;
    }
    let ctx = EffectCtx {
        caster,
        spell,
        depth,
        dest: None,
        chain_multiplier: 1.0,
    };
    // Ret412–414: a350ms delayed background1261160, not another paid Blade.
    super::super::queued_triggers::schedule_full_trigger(world, &ctx, target, LIGHT_WITHIN, 350);
    // Ret416–430: the selected PTR stacking talent consumes one Righteous
    // Cause stack; without it the buff expires. This child does not reset cooldowns.
    let righteous = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == 402916)
            .map(|aura| aura.instance_id)
    });
    if selected(world, caster, 1261113) {
        if let Some(instance) = righteous {
            auras::spend_stack(world, caster, instance);
        }
    } else {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == 402916
        });
    }
}

#[derive(Component)]
struct ConsecratedBladeReadyAt(f64);

fn cast_consecrated_blade(world: &mut World, caster: Entity) {
    let known = world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(404834));
    let now = spell_cast::now(world);
    if !known
        || world
            .get::<ConsecratedBladeReadyAt>(caster)
            .is_some_and(|ready| now < ready.0)
    {
        return;
    }
    // sc_paladin.cpp67–68, Ret407–410:10s scripted ICD, native26573 field.
    world
        .entity_mut(caster)
        .insert(ConsecratedBladeReadyAt(now + 10.0));
    let Some(spell) = super::super::load_triggered_spell(world, 26573, 1) else {
        warn!("Consecrated Blade requires native26573");
        return;
    };
    if super::super::launch_triggered_spell(world, caster, &spell, Some(caster), 1) {
        super::batch9::spawn_consecration(world, caster, &spell);
    }
}

fn queue_tempest(world: &mut World, caster: Entity, parent: &SpellInfo, target: Entity, depth: u8) {
    // Ret441–493/526–532/609–614: ordinary Tempest uses224239; an echo's
    // Tempest uses53385. Both use the talent percentage, not the echo multiplier.
    let id = if parent.cast_damage_role == CastDamageRole::Echo {
        STORM
    } else {
        TEMPEST
    };
    let Some(mut child) = super::super::load_triggered_spell(world, id, depth + 1) else {
        return;
    };
    child.cast_damage_multiplier =
        Some(super::script_effect_value(world, caster, TEMPEST_TALENT, 0) / 100.0);
    child.cast_damage_role = CastDamageRole::Secondary;
    world.commands().queue(move |world: &mut World| {
        if !super::super::launch_triggered_spell(world, caster, &child, Some(target), depth + 1) {
            warn!("Ret Tempest native child refused");
        }
    });
}

/// Ret35–75 adds accumulated damage BEFORE normal source/target multipliers.
/// Native387113 suppresses those multipliers; preserve its native no-crit flag.
pub(in crate::spell_cast) fn prepare_stored_damage(spell: &mut SpellInfo) {
    if spell.id != 387113 {
        return;
    }
    const IGNORE_CASTER_DAMAGE_MODIFIERS: u32 = 0x2000_0000;
    const IGNORE_DAMAGE_TAKEN_MODIFIERS: u32 = 0x8000;
    let misc = spell
        .misc
        .as_mut()
        .expect("Execution Sentence requires native spell misc");
    misc.attributes[3] &= !IGNORE_CASTER_DAMAGE_MODIFIERS;
    misc.attributes[6] &= !IGNORE_CASTER_DAMAGE_MODIFIERS;
    misc.attributes[4] &= !IGNORE_DAMAGE_TAKEN_MODIFIERS;
}

/// Ret183–212: initial damage and gather debuffs use the actual enemy center.
pub(super) fn sentence_init(world: &mut World, ctx: &EffectCtx, target: Entity) {
    auras::track_ret_sentence_cast(world, ctx.caster, target);
    let Some(position) = targets::position(world, target) else {
        return;
    };
    let Some(mut child) = super::super::load_triggered_spell(world, SENTENCE_INIT, ctx.depth + 1)
    else {
        return;
    };
    child.cast_destination = Some(position.to_array());
    if !super::super::launch_triggered_spell(world, ctx.caster, &child, Some(target), ctx.depth + 1)
    {
        warn!("Execution Sentence initial child refused");
    }
}

/// sc_paladin.cpp1383–1406/1443–1456: one roll per Judgment-family impact.
pub(super) fn judgment_mastery(world: &mut World, ctx: &EffectCtx, target: Entity) {
    if !is_ret(world, ctx.caster) || (ctx.spell.id == 24275 && ctx.depth > 0) {
        return;
    }
    let mut chance = super::script_effect_value(world, ctx.caster, MASTERY, 3);
    if selected(world, ctx.caster, 383876) {
        chance *= 1.0 + super::script_effect_value(world, ctx.caster, 383876, 0) / 100.0;
    }
    if selected(world, ctx.caster, HIGHLORDS_WRATH) {
        chance *= wrath_mastery_factor(world, ctx.caster);
    }
    if crate::random::random::<f32>() * 100.0 < chance {
        spell_cast::cast_triggered(world, ctx.caster, 383921, Some(target), ctx.depth + 1);
    }
}

pub(super) fn after_direct_damage(world: &mut World, ctx: &EffectCtx, target: Entity, crit: bool) {
    if !crit
        || !selected(world, ctx.caster, SUN_SEAR_TALENT)
        || ctx.spell.cast_damage_role != CastDamageRole::Primary
    {
        return;
    }
    // Ret633–637 and sc_paladin.cpp1767–1771: critical Storm/HoW only.
    if matches!(ctx.spell.id, STORM | 24275 | 1279408) {
        spell_cast::cast_triggered(world, ctx.caster, 431414, Some(target), ctx.depth + 1);
    }
}

pub(in crate::spell_cast) fn after_periodic_damage(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    crit: bool,
) {
    if ctx.spell.id == 431380 && selected(world, ctx.caster, AVATAR_TALENT) {
        // sc_paladin.cpp3048–3055: one native Avatar action per Dawnlight tick.
        spell_cast::cast_triggered(world, ctx.caster, 431911, Some(target), ctx.depth + 1);
    }
    if ctx.spell.id == TRUTHS_WAKE && crit && selected(world, ctx.caster, BURN_TO_ASH) {
        extend_truths_wake(world, ctx.caster, target);
    }
}

fn extend_truths_wake(world: &mut World, caster: Entity, target: Entity) {
    let seconds = super::script_effect_value(world, caster, BURN_TO_ASH, 0);
    let now = spell_cast::now(world);
    // Ret752–759: extend only a still-ticking owned aura; preserve tick phase.
    let ticking = world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.spell_id == TRUTHS_WAKE
                && aura.caster == caster
                && aura.expires_at.is_some_and(|expiry| expiry > now)
        })
    });
    if ticking {
        auras::extend_owned_duration(
            world,
            caster,
            target,
            &[TRUTHS_WAKE],
            (seconds * 1000.0) as u32,
        );
    }
}

fn wrath_mastery_factor(world: &World, caster: Entity) -> f32 {
    1.0 + super::script_effect_value(world, caster, HIGHLORDS_WRATH, 2)
        / 100.0
        / super::script_effect_value(world, caster, HIGHLORDS_WRATH, 1)
}

fn effect_affects(source: &SpellInfo, index: u8, spell: &SpellInfo) -> bool {
    source
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .is_some_and(|effect| {
            spell_mods::class_mask_matches(
                source
                    .class_options
                    .as_ref()
                    .map_or(0, |options| options.spell_class_set),
                &effect.spell_class_mask,
                spell,
            )
        })
}

/// sc_paladin.hpp1258–1271: replace the native mastery factor, never stack a
/// second base mastery multiplier on already-whitelisted Final Verdict/Blade.
pub(in crate::spell_cast) fn damage_multiplier(
    world: &World,
    ctx: &EffectCtx,
    target: Entity,
    periodic: bool,
) -> f32 {
    let mut multiplier = mastery_multiplier(world, ctx, periodic);
    if ctx.spell.id == LIGHT_WITHIN && ctx.spell.cast_primary_target == Some(target) {
        multiplier *=
            1.0 + super::script_effect_value(world, ctx.caster, LIGHT_WITHIN_TALENT, 0) / 100.0;
    }
    multiplier
}

fn mastery_multiplier(world: &World, ctx: &EffectCtx, periodic: bool) -> f32 {
    if !is_ret(world, ctx.caster) || !auras::has_aura(world, ctx.caster, MASTERY) {
        return 1.0;
    }
    let store = world.resource::<SpellStore>();
    let Some(mastery) = store.get(MASTERY) else {
        return 1.0;
    };
    if !effect_affects(mastery, 0, ctx.spell) && !matches!(ctx.spell.id, BLADE | SENTENCE) {
        return 1.0;
    }
    let mut amount = super::script_effect_value(world, ctx.caster, MASTERY, 0) / 100.0;
    let hidden = store.get(HIDDEN_MASTERY).is_some_and(|hidden| {
        effect_affects(hidden, 0, ctx.spell) || effect_affects(hidden, 2, ctx.spell)
    });
    if hidden && auras::has_aura(world, ctx.caster, HIGHLORDS_WRATH) {
        amount *= wrath_mastery_factor(world, ctx.caster);
    }
    let op = if periodic { 22 } else { 0 };
    let native_product = world.get::<ActiveAuras>(ctx.caster).map_or(1.0, |auras| {
        auras
            .iter()
            .filter(|a| matches!(a.spell_id, MASTERY | HIDDEN_MASTERY))
            .flat_map(|a| {
                a.effects
                    .iter()
                    .filter(move |e| {
                        e.aura_type == 108
                            && e.misc_values[0] == op
                            && spell_mods::class_mask_affects(a, e, ctx.spell)
                    })
                    .map(move |e| 1.0 + e.amount * a.stack_multiplier() / 100.0)
            })
            .product::<f32>()
    });
    (1.0 + amount) / native_product
}

/// SimC action AoE selects one damage effect per recipient, with primary first.
/// Ret328–344/468–493; sc_paladin.cpp3004–3011.
pub(super) fn area_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let target = explicit.expect("Ret damage child requires its actual primary");
    let effect = &spell.effects[0];
    let (radius, cap) = match spell.id {
        LIGHT_WITHIN => (targets::radius(effect, 0), 8.0),
        431911 => (
            targets::radius(effect, 1),
            super::script_effect_value(world, caster, AVATAR_TALENT, 5),
        ),
        TEMPEST => {
            let storm = world
                .resource::<SpellStore>()
                .get(STORM)
                .expect("Tempest requires native Divine Storm");
            (targets::effect_radius(&storm.effects[0]), 0.0)
        }
        _ => unreachable!("only registered Ret area children"),
    };
    let center = targets::position(world, target).expect("Ret child primary position");
    let mut units = targets::area_units(world, caster, center, radius, targets::Relation::Enemy, 0);
    units.retain(|&unit| unit != target);
    units.insert(0, target);
    let reduced = if cap > 0.0 && units.len() as f32 > cap {
        (cap / units.len().min(20) as f32).sqrt()
    } else {
        1.0
    };
    let chain_multipliers = units.iter().map(|&unit| (unit, reduced)).collect();
    vec![PlannedEffect {
        index: 0,
        units,
        dest: None,
        chain_multipliers,
        selected_units: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}

pub(super) fn exclude_primary_cleave(
    _world: &World,
    _spell: &SpellInfo,
    explicit: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    // Ret301–310: effect2 is the coefficient for OTHER targets only.
    units.retain(|unit| Some(*unit) != explicit);
}
