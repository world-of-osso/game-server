//! 70448 Spiritfont (Mistweaver): 1260511's RPPM on Rising Sun Kick and Vivify, and selected
//! 1260680 on Thunder Focus Tea, grant 1260565. The next Enveloping Mist spends a stack, starts
//! 1260670 and puts Soothing Mist 1260617 (20% coefficient) on up to 1260511 s2 allies.
//! SimC 76f8118 has no Spiritfont model. Behaviour DB2 leaves open is INFERRED from WCL
//! `zCXNH4Gf1MPwvcZg` fight 10 actor 10 (Altar of Fangs +17, 50 activations); retire each
//! INFERRED rule when a decomp answer or a SimC implementation supplies it:
//! - INFERRED: only injured allies, caster included, receive a mist: initial counts were
//!   5:17, 4:12, 3:13, 2:5 and 1:3, never padded to five.
//! - INFERRED: healing is pooled. Each mist heals s2/active of its native tick: the per-tick
//!   total stayed about 37.3k with three, four or five mists and moved to survivors when one ended.
//! - INFERRED: a mist whose target is at full health when it is due ends instead of ticking
//!   and moves to a random injured ally without one; with none, the pool splits among the rest.
//! - INFERRED: moved mists end with 1260670, as every logged 1260617 removal does.
//! - 1260680 names no class mask; its tooltip names Thunder Focus Tea, and all 48 WCL Thunder
//!   Focus Tea casts granted 1260565. Each mist application, moved ones included, also casts Chi
//!   Cocoon 1260681 at 1260680 s1 effectiveness (206 of 211 WCL cocoons share its timestamp).
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::random::{self as rand, seq::SliceRandom};
use combat::spell_cast::summon::SummonedCreature;
use combat::spell_cast::{
    self, TriggeredSpellOverrides,
    script_registry::SpellScriptHook,
    scripts::{AfterCastHook, script_effect_value},
    targets::{self, Relation},
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{Health, UnitIsPet};

const TALENT: u32 = 1260511;
const TEA_TALENT: u32 = 1260680;
const PROC: u32 = 1260565;
const DRIVER: u32 = 1260670;
const MIST: u32 = 1260617;
const COCOON: u32 = 1260681;
const THUNDER_FOCUS_TEA: u32 = 116680;
const ENVELOPING_MIST: u32 = 124682;
/// Rising Sun Kick and its Rushing Wind Kick replacement; Vivify and the Sheilun's Gift it
/// resolves to in the replay profile (one of 72 WCL 1260565 gains followed Sheilun's Gift).
const RPPM_SOURCES: [u32; 4] = [107428, 467307, 116670, 399491];

/// Units that received this caster's current Spiritfont mists.
#[derive(Component, Default)]
struct SpiritfontMists(Vec<Entity>);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterResourceValues(after_cast),
    SpellScriptHook::OwnsPolicyProc(|spell| matches!(spell, TALENT | TEA_TALENT)),
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::NamedHealingMultiplier(pooled_mist_multiplier),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(MIST).or_default().before_periodic_tick = Some(move_from_full_target);
}

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    match spell {
        THUNDER_FOCUS_TEA => Some(activate_from_tea),
        ENVELOPING_MIST => Some(start_spiritfont),
        _ if RPPM_SOURCES.contains(&spell) => Some(roll_spiritfont),
        _ => None,
    }
}

fn roll_spiritfont(world: &mut World, caster: Entity, _: &SpellInfo) {
    let Some(talent) = own_instance(world, caster, TALENT) else {
        return;
    };
    if auras::procs::try_native_policy_proc(world, caster, talent) {
        grant_spiritfont(world, caster);
    }
}

fn activate_from_tea(world: &mut World, caster: Entity, _: &SpellInfo) {
    if own_instance(world, caster, TEA_TALENT).is_some() {
        grant_spiritfont(world, caster);
    }
}

fn grant_spiritfont(world: &mut World, caster: Entity) {
    if !spell_cast::try_cast_triggered(world, caster, PROC, Some(caster), 1) {
        warn!(?caster, "Spiritfont proc failed");
    }
}

fn start_spiritfont(world: &mut World, caster: Entity, _: &SpellInfo) {
    let Some(proc) = own_instance(world, caster, PROC) else {
        return;
    };
    auras::spend_stack(world, caster, proc);
    if !spell_cast::try_cast_triggered(world, caster, DRIVER, Some(caster), 1) {
        warn!(?caster, "Spiritfont driver failed");
        return;
    }
    end_mists(world, caster);
    let duration_ms = read_driver_remaining_ms(world, caster).expect("applied Spiritfont driver");
    let limit = script_effect_value(world, caster, TALENT, 1).max(0.0) as usize;
    let allies: Vec<_> = find_injured_allies(world, caster)
        .into_iter()
        .take(limit)
        .collect();
    for ally in allies {
        apply_mist(world, caster, ally, duration_ms);
    }
}

/// A new Enveloping Mist replaces the previous selection.
fn end_mists(world: &mut World, caster: Entity) {
    let Some(mists) = world.entity_mut(caster).take::<SpiritfontMists>() else {
        return;
    };
    for unit in mists.0 {
        auras::remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
            aura.spell_id == MIST && aura.caster == caster
        });
    }
}

fn apply_mist(world: &mut World, caster: Entity, ally: Entity, duration_ms: u32) {
    let overrides = TriggeredSpellOverrides {
        fixed_aura_duration_ms: Some(duration_ms),
        ..Default::default()
    };
    if !spell_cast::cast_triggered_with_overrides(world, caster, MIST, Some(ally), 1, overrides) {
        warn!(?caster, ?ally, "Spiritfont Soothing Mist failed");
        return;
    }
    record_mist_target(world, caster, ally);
    if own_instance(world, caster, TEA_TALENT).is_none() {
        return;
    }
    let overrides = TriggeredSpellOverrides {
        healing_multiplier: Some(script_effect_value(world, caster, TEA_TALENT, 0) / 100.0),
        ..Default::default()
    };
    if !spell_cast::cast_triggered_with_overrides(world, caster, COCOON, Some(ally), 1, overrides) {
        warn!(?caster, ?ally, "Spiritfont Chi Cocoon failed");
    }
}

fn record_mist_target(world: &mut World, caster: Entity, ally: Entity) {
    let mut caster = world.entity_mut(caster);
    let mut mists = caster.entry::<SpiritfontMists>().or_default();
    if !mists.get().0.contains(&ally) {
        mists.get_mut().0.push(ally);
    }
}

fn move_from_full_target(world: &mut World, unit: Entity, mist: &AuraInstance) -> bool {
    let full = world
        .get::<Health>(unit)
        .is_some_and(|health| health.current >= health.max);
    if !full {
        return true;
    }
    let caster = mist.caster;
    auras::remove_instance(world, unit, mist.instance_id, RemoveReason::Consumed);
    let Some(duration_ms) = read_driver_remaining_ms(world, caster) else {
        return false;
    };
    let misted = read_live_mist_targets(world, caster);
    let mut candidates: Vec<_> = find_injured_allies(world, caster)
        .into_iter()
        .filter(|ally| *ally != unit && !misted.contains(ally))
        .collect();
    candidates.shuffle(&mut rand::thread_rng());
    if let Some(&next) = candidates.first() {
        apply_mist(world, caster, next, duration_ms);
    }
    false
}

/// Each live mist heals s2/active of its own tick.
fn pooled_mist_multiplier(world: &World, caster: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    if spell.id != MIST {
        return 1.0;
    }
    let active = read_live_mist_targets(world, caster).len();
    script_effect_value(world, caster, TALENT, 1) / active as f32
}

fn read_live_mist_targets(world: &World, caster: Entity) -> Vec<Entity> {
    let Some(mists) = world.get::<SpiritfontMists>(caster) else {
        return Vec::new();
    };
    mists
        .0
        .iter()
        .copied()
        .filter(|&unit| {
            world.get::<ActiveAuras>(unit).is_some_and(|auras| {
                auras
                    .iter()
                    .any(|aura| aura.spell_id == MIST && aura.caster == caster)
            })
        })
        .collect()
}

/// Living injured party members (no summons or pets) within 1260617's range, nearest first.
fn find_injured_allies(world: &mut World, caster: Entity) -> Vec<Entity> {
    let Some(center) = targets::position(world, caster) else {
        return Vec::new();
    };
    let (_, range) = targets::spell_range(
        world
            .resource::<SpellStore>()
            .get(MIST)
            .expect("Spiritfont Soothing Mist missing"),
        false,
    );
    let mut allies = targets::area_units(world, caster, center, range, Relation::Raid, 0);
    allies.retain(|&unit| {
        world.get::<SummonedCreature>(unit).is_none()
            && !world.get::<UnitIsPet>(unit).is_some_and(|pet| pet.0)
            && world
                .get::<Health>(unit)
                .is_some_and(|health| health.current < health.max)
    });
    allies
}

fn read_driver_remaining_ms(world: &World, caster: Entity) -> Option<u32> {
    let now = auras::aura_now(world);
    let expires_at = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == DRIVER && aura.caster == caster)?
        .expires_at?;
    Some(((expires_at - now).max(0.0) * 1000.0).round() as u32)
}

fn own_instance(world: &World, unit: Entity, spell: u32) -> Option<u32> {
    world
        .get::<ActiveAuras>(unit)?
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == unit)
        .map(|aura| aura.instance_id)
}
