//! Native owned-pet cast/hit relationships backed by equipped ItemSet auras.
use crate::auras::{ActiveAuras, AuraSource};
use crate::class_progression::{ActiveSpec, KnownSpells};
use crate::spell_cast::{self, effects::EffectCtx};
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;

struct StompCast {
    parent: u32,
    spec: u32,
    talent: u32,
    payload: u32,
    echo_set: u32,
    echo_effect: u8,
}

// SimC76f8118 sc_hunter.cpp5138–5145: each active pet executes selected
// Stomp, then the MID2 two-piece echo. Native1296631 owns its effectiveness.
const CASTS: &[StompCast] = &[StompCast {
    parent: 217200,
    spec: 253,
    talent: 199530,
    payload: 201754,
    echo_set: 1296631,
    echo_effect: 0,
}];

struct PetHitAura {
    parent: u32,
    set: u32,
    aura: u32,
}

// SimC76f8118 sc_hunter.cpp3120–3125: every Stomp impact increments the
// owner's Cobra Fang. Native1299389 supplies its amount and stack cap.
const HITS: &[PetHitAura] = &[PetHitAura {
    parent: 201754,
    set: 1296632,
    aura: 1299389,
}];

fn owns_set(world: &World, owner: Entity, spell: u32) -> bool {
    world.get::<ActiveAuras>(owner).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.spell_id == spell
                && aura.caster == owner
                && matches!(aura.source, AuraSource::ItemSet(_))
        })
    })
}

pub(super) fn has_cast(parent: u32) -> bool {
    CASTS.iter().any(|rule| rule.parent == parent)
}

pub(super) fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for rule in CASTS.iter().filter(|rule| rule.parent == spell.id) {
        cast_stomps(world, caster, spell, rule);
    }
}

fn read_echo(world: &World, caster: Entity, rule: &StompCast) -> Option<SpellInfo> {
    if !owns_set(world, caster, rule.echo_set) {
        return None;
    }
    let percent = super::super::script_effect_value(world, caster, rule.echo_set, rule.echo_effect);
    let mut spell = world
        .resource::<SpellStore>()
        .get(rule.payload)
        .expect("native pet-set echo payload")
        .clone();
    spell.cast_damage_multiplier = Some(percent / 100.0);
    Some(spell)
}

fn cast_stomps(world: &mut World, caster: Entity, spell: &SpellInfo, rule: &StompCast) {
    let selected = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == rule.spec)
        && world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(rule.talent));
    let echo = read_echo(world, caster, rule);
    if !selected && echo.is_none() {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        warn!(
            ?caster,
            spell = spell.id,
            "Native pet Stomp lacks selected primary"
        );
        return;
    };
    let depth = spell_cast::next_trigger_depth(world);
    for pet in crate::hunter_pet::active_owned_pets(world, caster) {
        if selected {
            spell_cast::cast_triggered(world, pet, rule.payload, Some(target), depth);
        }
        if let Some(echo) = &echo {
            spell_cast::launch_triggered_spell(world, pet, echo, Some(target), depth);
        }
    }
}

pub(super) fn has_hit(parent: u32) -> bool {
    HITS.iter().any(|rule| rule.parent == parent)
}

pub(super) fn on_hit(world: &mut World, ctx: &EffectCtx<'_>) {
    let Some(owner) = world
        .get::<crate::hunter_pet::HunterPet>(ctx.caster)
        .map(|pet| pet.owner)
    else {
        return;
    };
    for rule in HITS.iter().filter(|rule| rule.parent == ctx.spell.id) {
        if owns_set(world, owner, rule.set) {
            spell_cast::cast_triggered(world, owner, rule.aura, Some(owner), ctx.depth + 1);
        }
    }
}
