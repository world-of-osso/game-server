//! Preservation Echo 364343: the caster's next healing spell also heals every
//! holder of their Echo at its effect1 percentage, consuming it.
//! SimC76f8118 has no Preservation Echo. Native 364343 description: "your next
//! healing spell [casts] an additional time on that ally at $s2% of normal
//! healing. Essence spells cannot be Echoed."
//! Retail WCL hbJmL1xDC6qkrdcR fight 1: Echo removals coincide with Merithra's
//! Blessing (100), Verdant Embrace (38), Living Flame, Dream Breath and Reversion
//! casts, never Temporal Anomaly or Essence spells. Every holder, the primary
//! target included, receives the root's heal at the cast; echoed Verdant Embrace
//! heals but creates no Lifebind bond, so the copy is the heal child alone.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, RemoveReason};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::AfterCastHook;
use combat::spell_cast::targets::{Relation, relation_matches};
use combat::spell_cast::{self, TriggeredSpellOverrides};
use combat::spell_info::SpellInfo;

pub(crate) const ECHO: u32 = 364343;
pub(crate) const REPLICATION_EFFECT: u8 = 1;
const LIVING_FLAME: u32 = 361469;

/// Foreground non-Essence healing roots and the heal each Echo holder receives.
const PAYLOADS: &[(u32, u32)] = &[
    (LIVING_FLAME, 361509),
    (360995, 361195),
    (355936, 355941),
    (366155, 366155),
    (1256581, 1256581),
    (367226, 367230),
];

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ProcAfterCast(lookup)];

fn lookup(spell: u32) -> Option<AfterCastHook> {
    PAYLOADS
        .iter()
        .any(|&(root, _)| root == spell)
        .then_some(replicate)
}

fn replicate(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // Living Flame damages enemies; only its ally heal is a healing spell.
    if spell.id == LIVING_FLAME
        && !spell
            .cast_executed_target
            .is_some_and(|target| relation_matches(world, caster, target, Relation::Ally))
    {
        return;
    }
    let (_, payload) = *PAYLOADS
        .iter()
        .find(|&&(root, _)| root == spell.id)
        .expect("registered Echo root");
    for (holder, instance, percent) in echo_holders(world, caster) {
        combat::auras::remove_instance(world, holder, instance, RemoveReason::Consumed);
        spell_cast::cast_triggered_with_overrides(
            world,
            caster,
            payload,
            Some(holder),
            1,
            TriggeredSpellOverrides {
                healing_multiplier: Some(percent / 100.0),
                ..Default::default()
            },
        );
    }
}

/// Each unit holding `caster`'s Echo, with its instance and replication percent.
fn echo_holders(world: &mut World, caster: Entity) -> Vec<(Entity, u32, f32)> {
    let units = combat::system_states::entities_with::<With<ActiveAuras>>(world);
    units
        .into_iter()
        .filter_map(|unit| {
            owned_echo(world, caster, unit).map(|(instance, percent)| (unit, instance, percent))
        })
        .collect()
}

/// `caster`'s Echo on `unit`: its instance and replication percent.
pub(crate) fn owned_echo(world: &World, caster: Entity, unit: Entity) -> Option<(u32, f32)> {
    let aura = world
        .get::<ActiveAuras>(unit)?
        .iter()
        .find(|aura| aura.spell_id == ECHO && aura.caster == caster)?;
    let effect = aura
        .effects
        .iter()
        .find(|effect| effect.index == REPLICATION_EFFECT)?;
    Some((aura.instance_id, effect.amount))
}
