//! Native owned-pet cast/hit relationships backed by equipped ItemSet auras.
use crate::auras::{ActiveAuras, AuraSource};
use crate::class_progression::{ActiveSpec, KnownSpells};
use crate::spell_cast::{self, effects::EffectCtx};
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;

const COBRA_SHOT: u32 = 193455;
const COBRA_FANG: u32 = 1299389;
const COBRA_FANG_SET: u32 = 1296632;
const BEAST_CLEAVE: u32 = 268877;

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
    set: COBRA_FANG_SET,
    aura: COBRA_FANG,
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

/// SimC76f8118 sc_hunter.cpp4983–4985/5024–5025 snapshots the earned
/// stacks before execute4970 expires them. Native aura effect1 owns the boost.
pub(crate) fn capture_cobra_damage(world: &World, caster: Entity, spell: &mut SpellInfo) {
    if spell.id != COBRA_SHOT
        || !owns_set(world, caster, COBRA_FANG_SET)
        || crate::auras::has_aura(world, caster, BEAST_CLEAVE)
    {
        return;
    }
    let now = spell_cast::now(world);
    let Some(aura) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras.iter().find(|aura| {
            aura.spell_id == COBRA_FANG
                && aura.caster == caster
                && aura.expires_at.is_none_or(|at| at > now)
        })
    }) else {
        return;
    };
    let percent = aura
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("native Cobra Fang damage effect1")
        .amount
        * f32::from(aura.stacks);
    spell.cast_damage_multiplier =
        Some(spell.cast_damage_multiplier.unwrap_or(1.0) * (1.0 + percent / 100.0));
}

/// SimC76f8118 sc_hunter.cpp4970 consumes the current earned aura after a
/// successful Cobra launch, even while the native missile is still in flight.
pub(crate) fn finish_cobra(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != COBRA_SHOT || !owns_set(world, caster, COBRA_FANG_SET) {
        return;
    }
    crate::auras::remove_auras(
        world,
        caster,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == COBRA_FANG && aura.caster == caster,
    );
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
