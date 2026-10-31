//! SimC76f8118 sc_evoker.cpp3795–3839/6256–6261/6368–6377.
//! Native summon identities, flight period, payloads and actor lifetimes remain authoritative.
use bevy::prelude::*;
use shared::components::Npc;

use super::combat;
use combat::auras::scripts::AuraScripts;
use combat::auras::{AuraInstance, RemoveReason, has_aura};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::AfterCastHook;
use combat::spell_cast::summon::SummonedCreature;
use combat::spell_info::{SpellInfo, SpellStore};

const TALENT: u32 = 1260745;
const FLIGHT: u32 = 1236943;
const PYRE: u32 = 1236970;
const ENTRY: u32 = 219827;
const SUMMONS: &[u32] = &[1235843, 1237036];

#[derive(Component)]
struct CommandTarget(Entity);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AdditionalReachActions(autonomous_actions),
    SpellScriptHook::AfterResourceValues(breath_hook),
    SpellScriptHook::MidAuraRegistration(register),
    SpellScriptHook::CriticalChance(pyre_critical_chance),
];

fn autonomous_actions(world: &World, actor: Entity) -> Vec<u32> {
    if world.get::<CommandTarget>(actor).is_some() {
        vec![PYRE]
    } else {
        Vec::new()
    }
}

fn pyre_critical_chance(world: &World, caster: Entity, _target: Entity, spell: u32) -> Option<f32> {
    if spell != PYRE || world.get::<Npc>(caster)?.template_id != ENTRY {
        return None;
    }
    let summon = world.get::<SummonedCreature>(caster)?;
    if !SUMMONS.contains(&summon.spell_id) {
        return None;
    }
    // SimC76f8118 pet.cpp217–220: actual passive Commandos read live owner spell crit.
    Some(
        world
            .get::<combat::stats::CharacterStats>(summon.owner)
            .expect("native Commando owner retains character stats")
            .crit_pct,
    )
}

fn breath_hook(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 357210 | 433874).then_some(start_squadron)
}

fn owned_commandos(world: &mut World, owner: Entity) -> Vec<Entity> {
    world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| {
            npc.template_id == ENTRY && summon.owner == owner && SUMMONS.contains(&summon.spell_id)
        })
        .map(|(actor, _, _)| actor)
        .collect()
}

fn start_squadron(world: &mut World, owner: Entity, spell: &SpellInfo) {
    if !has_aura(world, owner, TALENT) {
        return;
    }
    let Some(primary) = spell.cast_primary_target else {
        return;
    };
    let prior = owned_commandos(world, owner);
    let duration = world
        .resource::<SpellStore>()
        .get(FLIGHT)
        .and_then(|flight| flight.misc.as_ref())
        .and_then(|misc| misc.duration.as_ref())
        .expect("native Commando flight duration")
        .duration_ms;
    for &summon in SUMMONS {
        let overrides = combat::spell_cast::TriggeredSpellOverrides {
            duration_ms: Some(duration),
            ..Default::default()
        };
        if !combat::spell_cast::cast_triggered_with_overrides(
            world,
            owner,
            summon,
            Some(owner),
            1,
            overrides,
        ) {
            warn!(?owner, summon, "Native Command Squadron summon refused");
        }
    }
    for actor in owned_commandos(world, owner) {
        if prior.contains(&actor) {
            continue;
        }
        world.entity_mut(actor).insert(CommandTarget(primary));
        if !combat::spell_cast::try_cast_triggered(world, actor, FLIGHT, Some(actor), 1) {
            warn!(?actor, "Native Commando flight aura refused");
        }
    }
}

fn register(scripts: &mut AuraScripts) {
    let flight = scripts.0.entry(FLIGHT).or_default();
    flight.on_apply = Some(first_pyre);
    flight.on_periodic = Some(next_pyre);
}

fn first_pyre(world: &mut World, actor: Entity, _aura: &AuraInstance) {
    cast_pyre(world, actor);
}

fn next_pyre(world: &mut World, actor: Entity, aura: &AuraInstance, index: u8) {
    if index != 2 {
        return;
    }
    // SimC buff.cpp182/2525–2527: application is tick0; scheduled ticks1–3 also cast.
    if combat::auras::paid_periodic_ticks(world, aura.caster, actor, FLIGHT) <= 3 {
        cast_pyre(world, actor);
    }
}

fn cast_pyre(world: &mut World, actor: Entity) {
    let Some(target) = world.get::<CommandTarget>(actor).map(|target| target.0) else {
        return;
    };
    if !combat::spell_cast::try_cast_triggered(world, actor, PYRE, Some(target), 1) {
        warn!(?actor, ?target, "Native Commando Pyre refused");
    }
}

pub(super) fn finish_maneuverability(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    reason: RemoveReason,
) {
    if unit != aura.caster || !matches!(reason, RemoveReason::ChannelEnded | RemoveReason::Expired)
    {
        return;
    }
    for actor in owned_commandos(world, unit) {
        combat::spell_cast::summon::despawn_summon(world, actor);
    }
}
