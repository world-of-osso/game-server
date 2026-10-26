//! Pinned Survival producers; native spell/aura records retain every magnitude.
use super::cast_relationships::{decrement, has_talent};
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use combat::random::Rng;
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SuccessfulCastRelationships(after_cast),
    SpellScriptHook::ResolvedDamage(consume_tip),
];

const KILL_COMMAND: u32 = 259489;
const TIP_TALENT: u32 = 260285;
const TIP: u32 = 260286;
const PRIMAL_SURGE: u32 = 1272154;
const RAPTOR_STRIKE: u32 = 186270;
const RAPTOR_SWIPE: u32 = 1262293;
const SWIPE_BUFF: u32 = 1273155;
const SWIPE_RANK_ONE: u32 = 1259003;
const SWIPE_RANK_THREE: u32 = 1259019;

fn is_survival(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 255)
}

fn apply_self(world: &mut World, caster: Entity, spell_id: u32) {
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
}

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !is_survival(world, caster) {
        return;
    }
    match spell.id {
        KILL_COMMAND if has_talent(world, caster, TIP_TALENT) => grant_tip(world, caster),
        RAPTOR_STRIKE => roll_swipe(world, caster),
        RAPTOR_SWIPE => {
            // SimC76f8118 sc_hunter.cpp6188: the actual upgraded execution
            // consumes its replacement; it cannot earn itself again.
            auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
                aura.caster == caster && aura.spell_id == SWIPE_BUFF
            });
        }
        _ => {}
    }
}

fn grant_tip(world: &mut World, caster: Entity) {
    // SimC76f8118 sc_hunter.cpp6606–6608: one ordinary use plus the selected
    // native Primal Surge effect; normal aura application retains max/refresh.
    let extra = if has_talent(world, caster, PRIMAL_SURGE) {
        script_effect_value(world, caster, PRIMAL_SURGE, 0) as u8
    } else {
        0
    };
    for _ in 0..=extra {
        apply_self(world, caster, TIP);
    }
}

fn roll_swipe(world: &mut World, caster: Entity) {
    // SimC76f8118 sc_hunter.cpp6217–6236: rank3 supplies its own native
    // chance instead of modifying rank1. Only ordinary Strike earns Swipe.
    let rank = if has_talent(world, caster, SWIPE_RANK_THREE) {
        SWIPE_RANK_THREE
    } else if has_talent(world, caster, SWIPE_RANK_ONE) {
        SWIPE_RANK_ONE
    } else {
        return;
    };
    let chance = script_effect_value(world, caster, rank, 0);
    if combat::random::thread_rng().gen_range(0.0..100.0) < chance {
        apply_self(world, caster, SWIPE_BUFF);
    }
}

fn consume_tip(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    _critical: bool,
    _damage: f32,
) {
    // SimC76f8118 sc_hunter.cpp1359–1362: once at actual primary impact,
    // only when the native damage-family mask admits the action.
    if !is_survival(world, ctx.caster) || ctx.primary_target() != Some(target) {
        return;
    }
    let affected = world.get::<ActiveAuras>(ctx.caster).is_some_and(|auras| {
        auras
            .iter()
            .filter(|aura| aura.spell_id == TIP && aura.caster == ctx.caster)
            .any(|aura| {
                aura.effects.iter().any(|effect| {
                    combat::auras::spell_mods::class_mask_affects(aura, effect, ctx.spell)
                })
            })
    });
    if affected {
        decrement(world, ctx.caster, TIP);
        super::marksmanship_sentinel::queue_tip_mark(world, ctx.caster, target);
    }
}
