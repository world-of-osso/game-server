//! SIMC_BM_HOWL_OF_THE_PACK_LEADER_CYCLE: native combat entry, clock and KC.
//! SimC76f8118 hunter3776–3840/8240–8290 supplies linkage/order/consumption.
//! DB2 supplies quantities; actual native actors earn Cry. Boar >5-target
//! reduction and Bear AI are unqualified, not replaced with SimC numerics.
use std::collections::HashSet;

use bevy::prelude::*;

use super::effects::EffectCtx;
use super::summon::SummonedCreature;
use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::auras::{self, ActiveAuras, AuraInstance, RemoveReason};

const HOWL: u32 = 471876;
const COOLDOWN: u32 = 471877;
const WYVERN_READY: u32 = 471878;
const BOAR_READY: u32 = 472324;
const BEAR_READY: u32 = 472325;
const READY_SPELLS: &[u32] = &[WYVERN_READY, BOAR_READY, BEAR_READY];
const WYVERN_SUMMON: u32 = 1222271;
const WYVERN_CRY: u32 = 471881;
const BOAR_IMPACT: u32 = 471936;
const BOAR_CLEAVE: u32 = 471938;
const BEAR_LINKED_AURA: u32 = 471993;
const BEAR_SUMMON: u32 = 471990;
const AURA_TRIGGER_DEPTH: u8 = 1;

#[derive(Component, Default)]
struct HowlCycle {
    next: usize,
}

pub(crate) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        HOWL,
        AuraScript {
            on_proc: Some(start_combat_cycle),
            on_remove: Some(remove_cycle),
            ..Default::default()
        },
    );
    scripts.0.insert(
        COOLDOWN,
        AuraScript {
            on_remove: Some(grant_next_ready),
            ..Default::default()
        },
    );
    for &spell in READY_SPELLS {
        scripts.0.insert(
            spell,
            AuraScript {
                on_remove: Some(rearm_after_ready),
                ..Default::default()
            },
        );
    }
}

pub(crate) fn owns_ready(spell: u32) -> bool {
    READY_SPELLS.contains(&spell)
}

fn start_combat_cycle(world: &mut World, owner: Entity, _: &AuraInstance, _: Option<Entity>) {
    if world.get::<HowlCycle>(owner).is_none() {
        world.entity_mut(owner).insert(HowlCycle::default());
    }
    start_native_cooldown(world, owner);
}

fn remove_cycle(world: &mut World, owner: Entity, _: &AuraInstance, _: RemoveReason) {
    world.entity_mut(owner).remove::<HowlCycle>();
}

fn start_native_cooldown(world: &mut World, owner: Entity) {
    if !auras::has_aura(world, owner, HOWL) || auras::has_aura(world, owner, COOLDOWN) {
        return;
    }
    cast_native_child(world, owner, COOLDOWN, owner, AURA_TRIGGER_DEPTH);
}

fn grant_next_ready(world: &mut World, owner: Entity, _: &AuraInstance, reason: RemoveReason) {
    if reason != RemoveReason::Expired || !auras::has_aura(world, owner, HOWL) {
        return;
    }
    let Some(mut cycle) = world.get_mut::<HowlCycle>(owner) else {
        warn!(?owner, "Howl cooldown expired without its combat cycle");
        return;
    };
    let ready = READY_SPELLS[cycle.next];
    cycle.next = (cycle.next + 1) % READY_SPELLS.len();
    cast_native_child(world, owner, ready, owner, AURA_TRIGGER_DEPTH);
}

fn rearm_after_ready(world: &mut World, owner: Entity, _: &AuraInstance, reason: RemoveReason) {
    if matches!(reason, RemoveReason::Consumed | RemoveReason::Expired) {
        start_native_cooldown(world, owner);
    }
}

pub(crate) fn consume_on_kill_command(world: &mut World, owner: Entity, target: Entity, depth: u8) {
    if !auras::has_aura(world, owner, HOWL) {
        return;
    }
    for &ready in READY_SPELLS {
        let id = world.get::<ActiveAuras>(owner).and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == ready && aura.caster == owner)
                .map(|aura| aura.instance_id)
        });
        let Some(id) = id else { continue };
        if cast_ready_action(world, owner, target, ready, depth.saturating_add(1)) {
            auras::spend_charge(world, owner, id);
        }
    }
}

fn cast_ready_action(
    world: &mut World,
    owner: Entity,
    target: Entity,
    ready: u32,
    depth: u8,
) -> bool {
    match ready {
        WYVERN_READY => {
            cast_and_confirm_summon(world, owner, WYVERN_SUMMON, WYVERN_SUMMON, owner, depth)
        }
        BOAR_READY => cast_boar_action(world, owner, target, depth),
        BEAR_READY => {
            cast_and_confirm_summon(world, owner, BEAR_LINKED_AURA, BEAR_SUMMON, target, depth)
        }
        _ => unreachable!("registered Howl ready {ready}"),
    }
}

fn query_owned_summons(world: &mut World, owner: Entity, spell: u32) -> HashSet<Entity> {
    world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, summon)| summon.owner == owner && summon.spell_id == spell)
        .map(|(actor, _)| actor)
        .collect()
}

fn cast_and_confirm_summon(
    world: &mut World,
    owner: Entity,
    action: u32,
    summon: u32,
    target: Entity,
    depth: u8,
) -> bool {
    let before = query_owned_summons(world, owner, summon);
    if !cast_native_child(world, owner, action, target, depth) {
        return false;
    }
    let after = query_owned_summons(world, owner, summon);
    let created = after.difference(&before).next().is_some();
    if !created {
        warn!(
            ?owner,
            action, summon, "Howl action created no native actor; retaining readiness"
        );
    }
    created
}

fn cast_boar_action(world: &mut World, owner: Entity, target: Entity, depth: u8) -> bool {
    // SimC4797–4848 executes native impact and cleave, not a summoned Boar.
    // Native root effect1 supplies the number of passes. No guessed travel or cap math.
    let passes = super::scripts::script_effect_value(world, owner, HOWL, 1) as u32;
    let mut launched = false;
    for _ in 0..passes {
        if cast_native_child(world, owner, BOAR_IMPACT, target, depth) {
            cast_native_child(world, owner, BOAR_CLEAVE, target, depth);
            launched = true;
        }
    }
    if !launched {
        warn!(
            ?owner,
            passes, "Howl Boar executed no native impact; retaining readiness"
        );
    }
    launched
}

fn cast_native_child(
    world: &mut World,
    owner: Entity,
    spell: u32,
    target: Entity,
    depth: u8,
) -> bool {
    let cast = super::try_cast_triggered(world, owner, spell, Some(target), depth);
    if !cast {
        warn!(?owner, spell, ?target, "Howl native child failed");
    }
    cast
}

pub(super) fn on_summoned(world: &mut World, ctx: &EffectCtx, actor: Entity) {
    if ctx.spell.id != WYVERN_SUMMON || !crate::auras::has_aura(world, ctx.caster, HOWL) {
        return;
    }
    let actual_summon = world
        .get::<SummonedCreature>(actor)
        .is_some_and(|summon| summon.owner == ctx.caster && summon.spell_id == WYVERN_SUMMON);
    if !actual_summon {
        return;
    }
    let stacks = super::scripts::script_effect_value(world, ctx.caster, HOWL, 2) as u8;
    if stacks == 0 {
        return;
    }
    let cast = super::cast_triggered_with_overrides(
        world,
        ctx.caster,
        WYVERN_CRY,
        Some(ctx.caster),
        ctx.depth.saturating_add(1),
        super::TriggeredSpellOverrides {
            aura_stacks: Some(stacks),
            ..Default::default()
        },
    );
    if !cast {
        warn!(owner = ?ctx.caster, ?actor, "actual Howl Wyvern could not apply native Cry");
    }
}
