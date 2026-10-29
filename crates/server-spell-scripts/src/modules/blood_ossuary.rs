//! Blood's Bone Shield threshold and Death Strike cost.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{self, CostPool, script_registry::SpellScriptHook};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{ActiveSpec, PowerType};

const BONE_SHIELD: u32 = 195181;
const OSSUARY: u32 = 219786;
const OSSUARY_BUFF: u32 = 219788;
const IMPROVED_DEATH_STRIKE: u32 = 374277;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::AfterAuraApplication(bone_shield_applied),
    SpellScriptHook::PowerCost(death_strike_cost),
];

fn register(scripts: &mut AuraScripts) {
    let bone = scripts.0.entry(BONE_SHIELD).or_default();
    bone.on_proc = Some(before_bone_spent);
    bone.on_remove = Some(bone_shield_removed);
}

fn threshold(world: &World) -> u8 {
    world
        .resource::<SpellStore>()
        .get(OSSUARY)
        .expect("Ossuary talent")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Ossuary threshold")
        .base_points as u8
}

// SimC76f8118 sc_death_knight.cpp:15972–15979: gaining the threshold
// enables Ossuary; losing the threshold removes it.
fn bone_shield_applied(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.spell_id == BONE_SHIELD
        && aura.caster == unit
        && aura.stacks >= threshold(world)
        && world
            .get::<KnownSpells>(unit)
            .is_some_and(|known| known.contains(OSSUARY))
        && !auras::has_aura(world, unit, OSSUARY_BUFF)
    {
        spell_cast::cast_triggered(world, unit, OSSUARY_BUFF, Some(unit), 1);
    }
}

fn before_bone_spent(world: &mut World, unit: Entity, aura: &AuraInstance, _: Option<Entity>) {
    if aura.stacks <= threshold(world) {
        remove_ossuary(world, unit);
    }
}

fn bone_shield_removed(world: &mut World, unit: Entity, aura: &AuraInstance, _: RemoveReason) {
    if aura.caster == unit {
        remove_ossuary(world, unit);
    }
}

fn remove_ossuary(world: &mut World, unit: Entity) {
    auras::remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
        aura.spell_id == OSSUARY_BUFF && aura.caster == unit
    });
}

// SimC76f8118 sc_death_knight.cpp:10064–10105,15187–15189 uses the
// Blood-specific Improved Death Strike effect instead of the generic effect.
fn death_strike_cost(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    pool: CostPool,
    amount: f32,
) -> f32 {
    if spell.id != 49998
        || pool != CostPool::Power(PowerType::RunicPower)
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 250)
        || !auras::has_aura(world, caster, IMPROVED_DEATH_STRIKE)
    {
        return amount;
    }
    let talent = world
        .resource::<SpellStore>()
        .get(IMPROVED_DEATH_STRIKE)
        .expect("Improved Death Strike");
    let effect = |index| {
        talent
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("Improved Death Strike cost")
            .base_points
    };
    amount + effect(4) - effect(2)
}
