//! Spell scripts (TrinityCore `SpellScript`) keyed by spell ID: behaviour that the
//! spell's DB2 effects do not carry.

use bevy::prelude::*;

use super::targets::{Relation, relation_matches};
use crate::auras::mount::{FLIGHT_STYLE_STEADY, FlightStyle, set_flight_style};
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
/// Switch Flight Style: 436854 toggles, 460002 to Skyriding, 460003 to Steady.
const SWITCH_FLIGHT_STYLE: u32 = 436_854;
const SWITCH_TO_SKYRIDING: u32 = 460_002;
const SWITCH_TO_STEADY: u32 = 460_003;
/// Azure Strike (Blue): DUMMY 0 (the target count), SCHOOL_DAMAGE 1 in an area.
const AZURE_STRIKE: u32 = 362_969;
const FORTIFYING_BREW: u32 = 115_203;
const FORTIFYING_BREW_BUFF: u32 = 120_954;

/// `SpellEffectFn` on `OnEffectHitTarget`: runs per unit the effect selects,
/// after the effect's handler. A DUMMY effect with a hook selects targets.
pub type EffectHitHook = fn(&mut World, Entity, Entity, u8);

pub fn on_effect_hit(spell_id: u32, effect_index: u8) -> Option<EffectHitHook> {
    match (spell_id, effect_index) {
        (FORTIFYING_BREW, 0) => Some(fortifying_brew),
        (CHAOS_STRIKE_INITIAL, 0) => Some(chaos_strike_initial),
        (LIVING_FLAME, 0) => Some(living_flame),
        (SWITCH_FLIGHT_STYLE, 0) => Some(switch_flight_style),
        (SWITCH_TO_SKYRIDING, 0) => Some(switch_to_skyriding),
        (SWITCH_TO_STEADY, 0) => Some(switch_to_steady),
        _ => None,
    }
}

/// `SpellCastFn` on `AfterCast`: runs once on the caster after the spell resolved.
pub type AfterCastHook = fn(&mut World, Entity, &SpellInfo);

pub fn after_cast(spell_id: u32) -> Option<AfterCastHook> {
    shared::skyriding::ability_impulse(spell_id).map(|_| crate::skyriding::flap as AfterCastHook)
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

/// Fortifying Brew's DUMMY cast applies 120954 with custom health and damage
/// amounts from 115203 (local catalog: +20%, -20%). Retail snapshots a fixed
/// health gain at cast time and removes that same gain when the buff expires:
/// simulationcraft/simc, midnight, buffs::fortifying_brew_t::trigger/expire_override
/// https://github.com/simulationcraft/simc/blob/midnight/engine/class_modules/monk/sc_monk.cpp
/// The 115203 -> 120954 DUMMY bridge also exists in the TrinityCore-derived
/// RoleplayCore spell_monk_fortifying_brew::HandleDummy:
/// https://github.com/KamiliaBlow/RoleplayCore/blob/master/src/server/scripts/Spells/spell_monk.cpp
fn fortifying_brew(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    let store = world.resource::<crate::spell_info::SpellStore>();
    let (Some(cast), Some(buff)) = (store.get(FORTIFYING_BREW), store.get(FORTIFYING_BREW_BUFF))
    else {
        warn!("Fortifying Brew requires spells 115203 and 120954");
        return;
    };
    let (Some(health_effect), Some(damage_effect)) = (
        cast.effects.iter().find(|effect| effect.index == 0),
        cast.effects.iter().find(|effect| effect.index == 1),
    ) else {
        warn!("Fortifying Brew requires health and damage DUMMY effects");
        return;
    };
    let health_percent = health_effect.base_points;
    let damage_percent = damage_effect.base_points;
    let mut buff = buff.clone();
    let Some(health) = world.get::<shared::components::Health>(caster) else {
        return;
    };
    let existing = world
        .get::<crate::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == FORTIFYING_BREW_BUFF)
        });
    let previous_bonus = existing
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount);
    let health_bonus = ((health.max - previous_bonus) * health_percent / 100.0).floor();
    for effect in &mut buff.effects {
        match effect.index {
            0 => effect.base_points = health_bonus,
            1 => effect.base_points = damage_percent,
            _ => {}
        }
    }
    // Only the two requested defensive effects; stagger, talents and cosmetic
    // size effects are not implemented by this bounded bridge.
    crate::auras::apply_aura_from_spell(
        world,
        &crate::auras::AuraApplication {
            caster,
            target: caster,
            spell_id: FORTIFYING_BREW_BUFF,
            effect_mask: 0b11,
            source: crate::auras::AuraSource::Cast,
        },
        &buff,
    );
}

/// Switch Flight Style 436854 (the Riding skill line's ability, spelldesc
/// `$?A404468[Steady Flight][Skyriding]`): DUMMY 0 on the caster swaps Steady and
/// Skyriding (`auras::mount::set_flight_style`). TrinityCore scripts a DUMMY effect with an
/// `OnEffectHitTarget` `SpellEffectFn`; it has no script for this spell.
fn switch_flight_style(world: &mut World, _caster: Entity, target: Entity, _depth: u8) {
    let style = if crate::auras::has_aura(world, target, FLIGHT_STYLE_STEADY) {
        FlightStyle::Skyriding
    } else {
        FlightStyle::Steady
    };
    set_flight_style(world, target, style);
}

/// Switch Flight Style 460002 (icon ability_dragonriding_dynamicflight01): DUMMY 0 on
/// the caster puts it on Skyriding.
fn switch_to_skyriding(world: &mut World, _caster: Entity, target: Entity, _depth: u8) {
    set_flight_style(world, target, FlightStyle::Skyriding);
}

/// Switch Flight Style 460003 (icon ability_dragonriding_staticflight01): DUMMY 0 on
/// the caster puts it on Steady.
fn switch_to_steady(world: &mut World, _caster: Entity, target: Entity, _depth: u8) {
    set_flight_style(world, target, FlightStyle::Steady);
}

/// Energizing Flame (Evoker passive): DUMMY 0, the percent of Living Flame's mana
/// cost refunded when it hits an enemy.
const ENERGIZING_FLAME: u32 = 400_006;

/// `spell_evo_living_flame::HandleHitTarget` (spell_evoker.cpp:505-530): heal a
/// unit the caster can assist, damage any other; `HandleLaunchTarget` refunds
/// Energizing Flame's percent of the mana cost on an enemy.
fn living_flame(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if relation_matches(world, caster, target, Relation::Ally) {
        super::cast_triggered(world, caster, LIVING_FLAME_HEAL, Some(target), depth + 1);
        return;
    }
    refund_energizing_flame(world, caster);
    super::cast_triggered(world, caster, LIVING_FLAME_DAMAGE, Some(target), depth + 1);
}

fn refund_energizing_flame(world: &mut World, caster: Entity) {
    if !crate::auras::has_aura(world, caster, ENERGIZING_FLAME) {
        return;
    }
    let store = world.resource::<crate::spell_info::SpellStore>();
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
    let mana_cost: f32 = super::applicable_costs(world, caster, &living_flame)
        .unwrap_or_default()
        .into_iter()
        .filter(|(pool, ..)| *pool == super::CostPool::Power(shared::components::PowerType::Mana))
        .map(|(_, amount, _)| amount)
        .sum();
    if mana_cost > 0.0 {
        super::powers::change_power(
            world,
            caster,
            shared::components::PowerType::Mana,
            mana_cost * percent / 100.0,
        );
    }
}

/// `spell_evo_azure_strike::FilterTargets` (spell_evoker.cpp:94-107): the explicit
/// target plus effect 0's value minus one others picked at random
/// (`Trinity::Containers::RandomResize`).
fn azure_strike_targets(spell: &SpellInfo, explicit: Option<Entity>, units: &mut Vec<Entity>) {
    pick_azure_strike_targets(spell, explicit, units, &mut rand::thread_rng());
}

fn pick_azure_strike_targets(
    spell: &SpellInfo,
    explicit: Option<Entity>,
    units: &mut Vec<Entity>,
    rng: &mut impl rand::Rng,
) {
    use rand::seq::SliceRandom;
    let count = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .map_or(1, |effect| effect.base_points.max(1.0) as usize);
    units.retain(|&unit| Some(unit) != explicit);
    let others: Vec<Entity> = units.choose_multiple(rng, count - 1).copied().collect();
    *units = others;
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

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::*;
    use crate::spell_info::SpellEffect;

    /// Azure Strike's second target is random among the other enemies in the area.
    #[test]
    fn azure_strike_keeps_the_target_and_one_random_other() {
        let mut spell = SpellInfo::default();
        spell.effects.push(SpellEffect {
            index: 0,
            base_points: 2.0,
            ..Default::default()
        });
        let mut world = World::new();
        let target = world.spawn_empty().id();
        let others: Vec<Entity> = (0..4).map(|_| world.spawn_empty().id()).collect();
        let mut picked = std::collections::HashSet::new();
        for seed in 0..32 {
            let mut units = others.clone();
            units.insert(1, target);
            pick_azure_strike_targets(
                &spell,
                Some(target),
                &mut units,
                &mut StdRng::seed_from_u64(seed),
            );
            assert_eq!(units.len(), 2);
            assert_eq!(units[1], target);
            picked.insert(units[0]);
        }
        assert!(picked.len() > 1, "always the same other: {picked:?}");
    }
}
