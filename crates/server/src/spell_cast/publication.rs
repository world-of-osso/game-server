//! Approved bounded TC IsNeedSendToClient (Spell.cpp8346–8353), before flight.
//! Conditional-only visuals are an explicit unresolved gap, not an assumed match.
use std::collections::HashSet;

use bevy::prelude::*;
use shared::protocol::{CombatLogKind, SpellGo};

use super::{
    TargetPlan, attributes, broadcast_spell_go, cast_event, hit_targets, is_channeled, log_event,
};
use crate::spell_info::SpellInfo;

/// SharedDefines.h744; distinct from visual row presence or aura effect type.
const AURA_POINTS_ON_CLIENT: u32 = 0x1000;
/// SharedDefines.h726; Spell.cpp8353 is the final independent send term.
const ALWAYS_CAST_LOG: u32 = 0x8000_0000;

#[derive(Resource, Default)]
struct UnresolvedCasterVisuals(HashSet<u32>);

fn needs_client_send(spell: &SpellInfo, triggered: bool) -> bool {
    // SpellInfo.cpp1893–1896/1930–1933: channels and authored hit delay are
    // independent send reasons, even if no caster visual can be resolved.
    let hit_delay = spell
        .misc
        .as_ref()
        .is_some_and(|misc| misc.speed > 0.0 || misc.launch_delay > 0.0);
    let points_on_client = attributes(spell, 8) & AURA_POINTS_ON_CLIENT != 0;
    let always_log = attributes(spell, 7) & ALWAYS_CAST_LOG != 0;
    let runtime_send = !triggered || is_channeled(spell);
    let authored_send = points_on_client || hit_delay || always_log;
    runtime_send || authored_send || spell.has_unconditional_spell_x_visual
}

fn log_unresolved_visual_once(world: &mut World, spell_id: u32) {
    let first = world
        .get_resource_or_init::<UnresolvedCasterVisuals>()
        .0
        .insert(spell_id);
    if first {
        warn!(
            spell_id,
            "Triggered cast publication unresolved: only conditional caster visuals; no send event"
        );
    }
}

/// One publication point for ordinary, scripted, costed and bool FULL_MASK paths.
/// No cast-success proc event is generated here: FULL_MASK still disallows procs.
pub(super) fn publish(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
    plan: &TargetPlan,
    triggered: bool,
) {
    if !needs_client_send(spell, triggered) {
        if spell.has_conditional_spell_x_visual {
            log_unresolved_visual_once(world, spell.id);
        }
        return;
    }
    log_event(
        world,
        cast_event(caster, explicit, spell, CombatLogKind::CastSuccess),
    );
    broadcast_spell_go(
        world,
        SpellGo {
            caster: caster.to_bits(),
            target: explicit.map(Entity::to_bits),
            hit_targets: hit_targets(plan),
            spell_id: spell.id,
        },
    );
}
