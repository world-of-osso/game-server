//! Shared Evoker Living/Chrono Flame native hit relationships.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{
    self, CastFailure,
    script_registry::{HitHook, SpellScriptHook},
    targets::{self, Relation, relation_matches},
};

#[path = "evoker_flames/chrono_flame.rs"]
mod chrono_flame;

/// Living Flame (Red): DUMMY 0 only.
const LIVING_FLAME: u32 = 361_469;
/// Living Flame damage: SCHOOL_DAMAGE 0.
const LIVING_FLAME_DAMAGE: u32 = 361_500;
/// Living Flame heal.
const LIVING_FLAME_HEAL: u32 = 361_509;
/// Energizing Flame's percentage of the Living Flame mana cost.
const ENERGIZING_FLAME: u32 = 400_006;
/// SimC76f8118 sc_evoker.cpp1321 `evoker.heal_eb_chance` default.
const HEAL_ESSENCE_BURST_CHANCE: f32 = 0.9;

pub const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EffectHit(effect_hit),
    SpellScriptHook::CheckTarget(check_target),
    SpellScriptHook::HealthChangeObserved(chrono_flame::record_health_change),
    SpellScriptHook::NamedAuraFields(&[(431442, 0), (431442, 1), (431442, 2), (1260647, 0)]),
];

fn effect_hit(spell: u32, index: u8) -> Option<HitHook> {
    match (spell, index) {
        (LIVING_FLAME, 0) => Some(living_flame),
        (chrono_flame::CHRONO_FLAMES, 0) => Some(ordinary_flame),
        (LIVING_FLAME_DAMAGE | LIVING_FLAME_HEAL, 0) => Some(chrono_flame::repeat_landed_flame),
        _ => None,
    }
}

fn check_target(world: &World, _: Entity, spell: u32, _: Entity) -> Result<(), CastFailure> {
    if spell == chrono_flame::CHRONO_FLAMES {
        chrono_flame::check_children(world)?;
    }
    Ok(())
}

/// `spell_evo_living_flame::HandleHitTarget` (spell_evoker.cpp:505-530): heal a
/// unit the caster can assist, damage any other; `HandleLaunchTarget` refunds
/// Energizing Flame's percent of the mana cost on an enemy.
fn living_flame(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let stacks = world
        .get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 370901 && aura.caster == caster)
        })
        .map_or(0, |aura| aura.stacks);
    combat::auras::remove_auras(
        world,
        caster,
        combat::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 370901 && aura.caster == caster,
    );
    if relation_matches(world, caster, target, Relation::Ally) {
        spell_cast::cast_triggered(world, caster, LIVING_FLAME_HEAL, Some(target), depth + 1);
        super::essence_burst::roll_ruby_essence_burst(world, caster);
        return;
    }
    refund_energizing_flame(world, caster);
    spell_cast::cast_triggered(world, caster, LIVING_FLAME_DAMAGE, Some(target), depth + 1);
    let extras = leaping_extras(world, caster, target, stacks);
    for &extra in &extras {
        spell_cast::cast_triggered(world, caster, LIVING_FLAME_DAMAGE, Some(extra), depth + 1);
    }
    // SimC76f8118 sc_evoker.cpp6936–6951: unused Leaping Flames heal the caster;
    // each heal hit counts for Ruby Essence Burst at heal_eb_chance.
    let unused = usize::from(stacks) - extras.len();
    let mut heal_hits = 0;
    for _ in 0..unused {
        spell_cast::cast_triggered(world, caster, LIVING_FLAME_HEAL, Some(caster), depth + 1);
        if combat::random::random::<f32>() < HEAL_ESSENCE_BURST_CHANCE {
            heal_hits += 1;
        }
    }
    // sc_evoker.cpp7004–7023: one Ruby roll per damage hit and counted heal hit.
    for _ in 0..1 + extras.len() + heal_hits {
        super::essence_burst::roll_ruby_essence_burst(world, caster);
    }
}

fn leaping_extras(world: &mut World, caster: Entity, target: Entity, stacks: u8) -> Vec<Entity> {
    if stacks == 0 {
        return Vec::new();
    }
    let center = targets::position(world, caster).expect("Living Flame caster position");
    let spell = world
        .resource::<combat::spell_info::SpellStore>()
        .get(LIVING_FLAME)
        .expect("native Living Flame");
    let (_, range) = targets::spell_range(spell, true);
    let mut extras = targets::area_units(world, caster, center, range, Relation::Enemy, 0);
    extras.retain(|&unit| unit != target);
    extras.truncate(usize::from(stacks));
    extras
}

fn ordinary_flame(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if relation_matches(world, caster, target, Relation::Ally) {
        spell_cast::cast_triggered(world, caster, LIVING_FLAME_HEAL, Some(target), depth + 1);
    } else {
        refund_energizing_flame(world, caster);
        spell_cast::cast_triggered(world, caster, LIVING_FLAME_DAMAGE, Some(target), depth + 1);
    }
}

fn refund_energizing_flame(world: &mut World, caster: Entity) {
    if !combat::auras::has_aura(world, caster, ENERGIZING_FLAME) {
        return;
    }
    let store = world.resource::<combat::spell_info::SpellStore>();
    let (Some(living_flame), Some(energizing)) =
        (store.get(LIVING_FLAME), store.get(ENERGIZING_FLAME))
    else {
        return;
    };
    let percent = energizing
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .map_or(0.0, |effect| effect.base_points);
    let living_flame = living_flame.clone();
    let mana_cost: f32 = spell_cast::applicable_costs(world, caster, &living_flame)
        .unwrap_or_default()
        .into_iter()
        .filter(|(pool, ..)| {
            *pool == spell_cast::CostPool::Power(shared::components::PowerType::Mana)
        })
        .map(|(_, amount, _)| amount)
        .sum();
    if mana_cost > 0.0 {
        spell_cast::powers::change_power(
            world,
            caster,
            shared::components::PowerType::Mana,
            mana_cost * percent / 100.0,
        );
    }
}
