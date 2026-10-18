//! Bounded Mainline12.1.0.69933 tooltip bridges. No guessed proc/soft-cap formula.
use bevy::prelude::*;
use shared::components::PowerType;

use super::{batch8, script_effect_value};
use crate::auras::{AuraApplication, AuraSource};
use crate::spell_cast;
use crate::spell_info::{SpellInfo, SpellStore};

pub(super) const BLADE_FLURRY: u32 = 13877;
pub(super) const FLURRY_DAMAGE: u32 = 331850;
pub(super) const FERAL_LUNGE: u32 = 196884;
pub(super) const VOIDFORM: u32 = 228260;
pub(super) const ARCANE_SURGE: u32 = 365350;
pub(super) const PROTECTION_JUDGMENT: u32 = 275779;
pub(super) const HAND_OF_GULDAN: u32 = 105174;
pub(super) const LIGHTNING_BOLT: u32 = 188196;
const IMPROVED_FLURRY: u32 = 272026;
const DEFT_MANEUVERS: u32 = 381878;
const LUNGE_BITE: u32 = 215802;
const VOIDFORM_BUFF: u32 = 194249;
const SURGE_BUFF: u32 = 365362;
const JUDGMENT_RANK: u32 = 315867;
const JUDGMENT_ENERGIZE: u32 = 220637;
const GULDAN_METEOR: u32 = 86040;
const ELEMENTAL_RESOURCE: u32 = 343725;

///13877 explicitly names331850s1/i: five base targets, eight with272026.
///Only the initial strike is bridged; subsequent single-target transfer is open.
pub(super) fn cast_flurry(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    let mut child = world
        .resource::<SpellStore>()
        .get(FLURRY_DAMAGE)
        .expect("Blade Flurry requires331850")
        .clone();
    let rank = crate::auras::has_aura(world, caster, IMPROVED_FLURRY);
    let restrictions = child
        .target_restrictions
        .as_mut()
        .expect("Flurry target cap");
    if !rank {
        restrictions.max_targets -= script_effect_value(world, caster, IMPROVED_FLURRY, 2) as u32;
    }
    let plan = spell_cast::plan_targets(world, caster, &child, None, false)
        .expect("Flurry native area selectors");
    spell_cast::missiles::launch(world, caster, &child, plan, 1);
}

///381878m2 awards one combo point per actual initial hit, not per cast.
pub(super) fn energize_flurry(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    let hit = world
        .resource::<spell_cast::ResolvingHits>()
        .0
        .last()
        .is_some_and(|frame| {
            frame.caster == caster
                && frame.spell_id == FLURRY_DAMAGE
                && frame.damage.iter().any(|(unit, _)| *unit == target)
        });
    if hit && crate::auras::has_aura(world, caster, DEFT_MANEUVERS) {
        let amount = script_effect_value(world, caster, DEFT_MANEUVERS, 1);
        spell_cast::powers::change_power(world, caster, PowerType::ComboPoints, amount);
    }
}

///196884 names215802. Keep this bite independently of effect254 displacement;
///its source parabolic-curve/arrival options are explicitly unsupported.
pub(super) fn bite_lunge(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    spell_cast::cast_triggered(world, caster, LUNGE_BITE, Some(target), depth + 1);
}

fn apply_named_buff(world: &mut World, caster: Entity, spell_id: u32, effect_mask: u32) {
    crate::auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id,
            effect_mask,
            source: AuraSource::Cast,
        },
    );
}

///228260 explicitly names194249s1/d. Volley and replacement charges remain open.
pub(super) fn apply_voidform(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    apply_named_buff(world, caster, VOIDFORM_BUFF, (1 << 0) | (1 << 1));
}

///365350 explicitly names365362s1/s3/d. No guessed Clearcasting child or
///mana-spent damage snapshot; this is only the named post-cast buff.
pub(super) fn apply_surge(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    apply_named_buff(world, caster, SURGE_BUFF, (1 << 0) | (1 << 1) | (1 << 2));
}

///275779's315867 gate names220637s1 Holy Power, once on actual damage.
pub(super) fn energize_judgment(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if crate::auras::has_aura(world, caster, JUDGMENT_RANK)
        && batch8::first_damage_target(world, caster, PROTECTION_JUDGMENT, target)
    {
        spell_cast::cast_triggered(world, caster, JUDGMENT_ENERGIZE, Some(caster), depth + 1);
    }
}

///105174m1*86040m1. The native root costs three whole shards (30 raw),
///matching s1=3; imp actors and optional Doom are not inferred from damage.
pub(super) fn launch_guldan_meteor(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if depth >= spell_cast::effects::MAX_TRIGGER_DEPTH {
        warn!("Hand of Guldan trigger depth exhausted");
        return;
    }
    let child = world
        .resource::<SpellStore>()
        .get(GULDAN_METEOR)
        .expect("Hand of Guldan requires86040")
        .clone();
    let multiplier = script_effect_value(world, caster, HAND_OF_GULDAN, 0);
    let mut plan = spell_cast::plan_targets(world, caster, &child, Some(target), false)
        .expect("Guldan meteor native target area");
    for effect in &mut plan {
        effect.chain_multipliers = effect
            .units
            .iter()
            .map(|&unit| (unit, multiplier))
            .collect();
    }
    spell_cast::missiles::launch(world, caster, &child, plan, depth + 1);
}

///188196's343725s1 awards six Maelstrom once, after the actual bolt hits.
pub(super) fn energize_lightning_bolt(
    world: &mut World,
    caster: Entity,
    target: Entity,
    _depth: u8,
) {
    if crate::auras::has_aura(world, caster, ELEMENTAL_RESOURCE)
        && batch8::first_damage_target(world, caster, LIGHTNING_BOLT, target)
    {
        let amount = script_effect_value(world, caster, ELEMENTAL_RESOURCE, 0);
        spell_cast::powers::change_power(world, caster, PowerType::Maelstrom, amount);
    }
}
