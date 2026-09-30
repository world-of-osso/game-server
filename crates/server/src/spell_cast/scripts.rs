//! Spell scripts (TrinityCore `SpellScript`) keyed by spell ID: behaviour that the
//! spell's DB2 effects do not carry.

use bevy::prelude::*;

use super::targets::{Relation, relation_matches};
use crate::spell_info::SpellInfo;

/// Frostbolt: its effects are TRIGGER_MISSILE 32 to 228597 (SCHOOL_DAMAGE) only.
const FROSTBOLT: u32 = 116;
/// Chilled: APPLY_AURA MOD_DECREASE_SPEED 33 (-50%, 8 s).
const CHILLED: u32 = 205_708;

/// Chaos Strike (Demon Hunter, no spec): DUMMY 0 only.
const CHAOS_STRIKE_INITIAL: u32 = 344_862;
/// Chaos Strike: DUMMY 0, TRIGGER_SPELL 1 (222031) and 2 (199547), the weapon hits.
const CHAOS_STRIKE: u32 = 162_794;
/// Living Flame (Red): DUMMY 0 only.
const LIVING_FLAME: u32 = 361_469;
/// Living Flame damage: SCHOOL_DAMAGE 0.
const LIVING_FLAME_DAMAGE: u32 = 361_500;
/// Living Flame heal.
const LIVING_FLAME_HEAL: u32 = 361_509;
/// Azure Strike (Blue): DUMMY 0 (the target count), SCHOOL_DAMAGE 1 in an area.
const AZURE_STRIKE: u32 = 362_969;

/// `SpellEffectFn` on `OnEffectHitTarget`: runs per unit the effect selects,
/// after the effect's handler. A DUMMY effect with a hook selects targets.
pub type EffectHitHook = fn(&mut World, Entity, Entity, u8);

pub fn on_effect_hit(spell_id: u32, effect_index: u8) -> Option<EffectHitHook> {
    match (spell_id, effect_index) {
        (CHAOS_STRIKE_INITIAL, 0) => Some(chaos_strike_initial),
        (LIVING_FLAME, 0) => Some(living_flame),
        _ => None,
    }
}

/// `SpellObjectAreaTargetSelectFn`: narrows the units an effect selected.
/// Receives the spell and its explicit target.
pub type AreaTargetFilter = fn(&SpellInfo, Option<Entity>, &mut Vec<Entity>);

pub fn area_target_filter(spell_id: u32, effect_index: u8) -> Option<AreaTargetFilter> {
    match (spell_id, effect_index) {
        (AZURE_STRIKE, 1) => Some(azure_strike_targets),
        _ => None,
    }
}

/// `spell_dh_chaos_strike_initial::HandleHit` (spell_dh.cpp:368-388): the caster
/// casts Chaos Strike 162794 at the hit unit, triggered.
fn chaos_strike_initial(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    super::cast_triggered(world, caster, CHAOS_STRIKE, Some(target), depth + 1);
}

/// `spell_evo_living_flame::HandleHitTarget` (spell_evoker.cpp:505-530): heal a
/// unit the caster can assist, damage any other. The Energizing Flame mana
/// refund (`HandleLaunchTarget`) is not implemented.
fn living_flame(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let spell = if relation_matches(world, caster, target, Relation::Ally) {
        LIVING_FLAME_HEAL
    } else {
        LIVING_FLAME_DAMAGE
    };
    super::cast_triggered(world, caster, spell, Some(target), depth + 1);
}

/// `spell_evo_azure_strike::FilterTargets` (spell_evoker.cpp:94-107): the explicit
/// target plus up to effect 0's value minus one others. TrinityCore picks the
/// others at random; this keeps the first ones selected.
fn azure_strike_targets(spell: &SpellInfo, explicit: Option<Entity>, units: &mut Vec<Entity>) {
    let count = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .map_or(1, |effect| effect.base_points.max(1.0) as usize);
    units.retain(|&unit| Some(unit) != explicit);
    units.truncate(count - 1);
    units.extend(explicit);
}

/// `SpellHitFn`: runs once per unit the spell lands on, after its effects resolved there.
pub type HitHook = fn(&mut World, Entity, Entity, u8);

pub fn on_hit(spell_id: u32) -> Option<HitHook> {
    match spell_id {
        FROSTBOLT => Some(frostbolt_chilled),
        _ => None,
    }
}

/// `spell_mage_frostbolt::HandleChilled` (spell_mage.cpp:978-995): the caster casts
/// Chilled at the hit unit, triggered.
fn frostbolt_chilled(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    super::cast_triggered(world, caster, CHILLED, Some(target), depth + 1);
}
