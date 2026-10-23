//! SimC76f8118 sc_monk.cpp358–373,870–871,6359–6365; native70448 values.
use bevy::prelude::*;

use crate::auras::{self, ActiveAuras, RemoveReason};
use crate::random as rand;
use crate::spell_cast::{CostPool, SpellHistory, applicable_costs};
use crate::spell_info::SpellInfo;
use shared::components::PowerType;

// Source may_combo_strike actions, shared with the published starter policy.
const COMBO_ACTIONS: &[u32] = &[100780, 100784, 107428, 113656, 101546, 392983, 322109];
const COMBO_STRIKES: u32 = 115636;
const MASTERY_DAMAGE: &[u32] = &[
    100780, 100784, 185099, 117418, 107270, 395519, 395521, 322109,
];

#[derive(Component)]
struct ComboStrikes {
    active: bool,
}

const HIT_COMBO: u32 = 196740;
const HIT_COMBO_BUFF: u32 = 196741;
const COMBO_BREAKER: u32 = 137384;
const COMBO_BREAKER_BUFF: u32 = 116768;
pub(crate) const DANCE_OF_CHIJI: u32 = 325201;
const DANCE_BUFF: u32 = 325202;
const SPINNING_CRANE_KICK: u32 = 101546;

fn ordinary_cast_event(event: &auras::procs::AuraEvent) -> bool {
    event.can_proc && event.actor_mask[1] & auras::procs::PROC2_CAST_SUCCESSFUL != 0
}

/// SimC383–392: current Chi resource qualifies, even when native cost is discounted.
/// Check the root action rather than losing its resource identity on a damage child.
pub(crate) fn check_dance_proc(
    world: &World,
    aura: &auras::AuraInstance,
    event: &auras::procs::AuraEvent,
) -> bool {
    if !ordinary_cast_event(event) {
        return false;
    }
    let Some(spell) = event
        .spell_id
        .and_then(|id| world.resource::<crate::spell_info::SpellStore>().get(id))
    else {
        return false;
    };
    applicable_costs(world, aura.caster, spell).is_ok_and(|costs| {
        costs
            .iter()
            .any(|(pool, _, _)| *pool == CostPool::Power(PowerType::Chi))
    })
}

/// SimC1664–1666: the generic Chi-action roll occurs before this SCK use is spent.
pub(crate) fn after_dance_cast_event(world: &mut World, event: &auras::procs::AuraEvent) {
    if !ordinary_cast_event(event) || event.spell_id != Some(SPINNING_CRANE_KICK) {
        return;
    }
    let caster = event.actor;
    let instance = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == DANCE_BUFF && aura.caster == caster)
        })
        .map(|aura| aura.instance_id);
    if let Some(instance) = instance {
        auras::spend_stack(world, caster, instance);
    }
}

/// Ordinary successful casts only; triggered children do not change combo order.
pub(crate) fn ordinary_combo_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !COMBO_ACTIONS.contains(&spell.id) {
        return;
    }
    let previous = world
        .get::<SpellHistory>(caster)
        .and_then(|history| history.last_cast_in(COMBO_ACTIONS));
    let active = previous != Some(spell.id);
    if auras::has_aura(world, caster, COMBO_STRIKES) {
        world.entity_mut(caster).insert(ComboStrikes { active });
    }
    if !auras::has_aura(world, caster, HIT_COMBO) {
        return;
    }
    if !active {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == HIT_COMBO_BUFF && aura.caster == caster
        });
    } else {
        crate::spell_cast::cast_triggered(world, caster, HIT_COMBO_BUFF, Some(caster), 1);
    }
}

/// WW_COMBO_STRIKES_SIMC_POLICY: SimC76f8118 sc_monk.cpp142–156/358–373.
/// TC has no115636 script; DB2 owns the coefficient, not SimC's numeric value.
/// Eligibility covers the published kit's damage actions/children only. Utility,
/// white hits and unrelated procs neither consume the state nor gain mastery.
pub(crate) fn combo_strikes_multiplier(world: &World, caster: Entity, spell: u32) -> f32 {
    let active = world
        .get::<ComboStrikes>(caster)
        .is_some_and(|state| state.active);
    if !active || !MASTERY_DAMAGE.contains(&spell) || !auras::has_aura(world, caster, COMBO_STRIKES)
    {
        return 1.0;
    }
    let stats = world
        .get::<crate::stats::CharacterStats>(caster)
        .expect("native Combo Strikes caster retains class stats");
    let mastery = world
        .resource::<crate::spell_info::SpellStore>()
        .get(COMBO_STRIKES)
        .expect("native Combo Strikes aura retains its definition");
    let coefficient = mastery
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Combo Strikes effect0")
        .bonus_coefficient;
    1.0 + stats.mastery_points * coefficient / 100.0
}

/// Tiger Palm rolls its learned passive's native effect0 chance on a landed hit.
pub(super) fn tiger_palm_hit(world: &mut World, caster: Entity, _target: Entity, depth: u8) {
    let chance = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == COMBO_BREAKER))
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount);
    if rand::random::<f32>() * 100.0 < chance {
        crate::spell_cast::cast_triggered(
            world,
            caster,
            COMBO_BREAKER_BUFF,
            Some(caster),
            depth + 1,
        );
    }
}
