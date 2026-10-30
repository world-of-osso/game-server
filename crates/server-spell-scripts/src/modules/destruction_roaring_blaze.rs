//! Roaring Blaze spreads the primary's existing damage-over-time aura.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::class_progression::TraitLoadout;
use combat::random::seq::SliceRandom;
use combat::spell_cast::{
    effects::EffectCtx,
    script_registry::SpellScriptHook,
    scripts::script_effect_value,
    targets::{self, Relation},
};
use shared::components::{ActiveSpec, Position};

const ROARING_BLAZE: u32 = 1244310;
const CONFLAGRATE: u32 = 17962;
const WITHER: u32 = 445474;
const IMMOLATE: u32 = 157736;
// The existing Roaring Blaze recipient query models an 8-yard circle.
const SPREAD_RADIUS: f32 = 8.0;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::ResolvedDamage(spread_dot),
];

fn declared_reach(
    world: &World,
    caster: Entity,
    spell: &combat::spell_info::SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    let selected = world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == ROARING_BLAZE)
    });
    if spell.id == CONFLAGRATE && selected {
        combat::spell_cast::script_registry::ScriptReach::target(SPREAD_RADIUS)
    } else {
        Default::default()
    }
}

fn spread_dot(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    _critical: bool,
    amount: f32,
) {
    if ctx.spell.id != CONFLAGRATE
        || ctx.depth != 0
        || ctx.spell.cast_primary_target != Some(target)
        || amount <= 0.0
        || world
            .get::<ActiveSpec>(ctx.caster)
            .is_none_or(|spec| spec.0 != 267)
        || !world.get::<TraitLoadout>(ctx.caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == ROARING_BLAZE)
        })
    {
        return;
    }
    let Some(dot) = [WITHER, IMMOLATE]
        .into_iter()
        .find(|&dot| has_owned_dot(world, ctx.caster, target, dot))
    else {
        return;
    };
    let count = script_effect_value(world, ctx.caster, ROARING_BLAZE, 1) as usize;
    let mut recipients = nearby_enemies(world, ctx.caster, target);
    recipients.shuffle(&mut combat::random::thread_rng());
    recipients.sort_by_key(|&recipient| has_owned_dot(world, ctx.caster, recipient, IMMOLATE));
    // SimC76f8118 actions4440-4500 and warlock.cpp1272-1319: bugs mode
    // permits one additional target only when it already has the DoT.
    for (index, recipient) in recipients.into_iter().take(count + 1).enumerate() {
        if index == count
            && ![WITHER, IMMOLATE]
                .into_iter()
                .any(|aura| has_owned_dot(world, ctx.caster, recipient, aura))
        {
            break;
        }
        if !combat::spell_cast::try_cast_triggered(world, ctx.caster, dot, Some(recipient), 1) {
            warn!(caster = ?ctx.caster, ?recipient, dot, "Roaring Blaze background DoT refused");
        }
    }
}

fn has_owned_dot(world: &World, caster: Entity, target: Entity, dot: u32) -> bool {
    let now = combat::auras::aura_now(world);
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.caster == caster
                && aura.spell_id == dot
                && aura.expires_at.is_none_or(|expiry| expiry > now)
        })
    })
}

fn nearby_enemies(world: &mut World, caster: Entity, primary: Entity) -> Vec<Entity> {
    let Some(origin) = targets::position(world, primary) else {
        return Vec::new();
    };
    world
        .query::<(Entity, &Position)>()
        .iter(world)
        .filter_map(|(candidate, _)| {
            (candidate != primary
                && targets::is_alive(world, candidate)
                && targets::same_map(world, caster, candidate)
                && targets::relation_matches(world, caster, candidate, Relation::Enemy)
                && targets::position(world, candidate)
                    .is_some_and(|position| position.distance(origin) <= SPREAD_RADIUS))
            .then_some(candidate)
        })
        .collect()
}
