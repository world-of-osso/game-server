//! SimC76f8118 sc_monk.cpp4709–4885 and 4890–4993: Aspect of Harmony pools.
use bevy::prelude::*;
use shared::components::{ActiveSpec, Health};
use shared::protocol::{CombatLogEvent, CombatLogKind};

use super::combat;
use combat::auras::{self, AuraApplication, AuraSource, RemoveReason, harmony::HarmonyPools};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{
    self,
    script_registry::SpellScriptHook,
    scripts::{AfterCastHook, script_effect_value},
};
use combat::spell_info::{SpellInfo, SpellStore};

const TALENT: u32 = 450508;
const ACCUMULATOR: u32 = 450521;
const SPENDER: u32 = 450711;
const DAMAGE: u32 = 450763;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::HealthChangeObserved(observe),
    SpellScriptHook::AfterResourceValues(spender_hook),
];

fn selected(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 268)
        && world
            .get::<TraitLoadout>(caster)
            .is_some_and(|traits| traits.active_talent_spells().any(|t| t.spell_id == TALENT))
}

fn apply(world: &mut World, caster: Entity, spell: u32) {
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn spender_hook(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 322507 | 1241059).then_some(start_spender)
}

fn start_spender(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !selected(world, caster) {
        return;
    }
    let amount = world
        .get::<HarmonyPools>(caster)
        .map_or(0.0, |pools| pools.accumulator);
    world.entity_mut(caster).insert(HarmonyPools {
        accumulator: 0.0,
        spender: amount,
    });
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == ACCUMULATOR && aura.caster == caster
    });
    apply(world, caster, SPENDER);
}

fn observe(world: &mut World, event: &CombatLogEvent) {
    let Some(caster) = event.source.and_then(Entity::try_from_bits) else {
        return;
    };
    let Some(target) = event.target.and_then(Entity::try_from_bits) else {
        return;
    };
    if !selected(world, caster) {
        return;
    }
    let effect = match event.kind {
        CombatLogKind::Damage => 0,
        CombatLogKind::Heal => 2,
        _ => return,
    };
    // sc_monk.cpp2225 admits Monk auto-attacks; class spell callbacks use
    // sc_monk.cpp6790–6808's explicit exclusion list.
    let class_ability = match event.spell_id {
        None => effect == 0,
        Some(216521 | 178173 | 124507 | 387621 | 115129 | 124255 | 450820 | 450763) => false,
        Some(spell) => world
            .resource::<SpellStore>()
            .get(spell)
            .and_then(|info| info.class_options.as_ref())
            .is_some_and(|options| options.spell_class_set == 53),
    };
    if !class_ability {
        return;
    }
    let amount = (event.amount - event.overflow).max(0) as f32;
    if amount <= 0.0 {
        return;
    }
    if auras::has_aura(world, caster, SPENDER) {
        if effect != 0 {
            return;
        }
        let percent = script_effect_value(world, caster, TALENT, 5) / 100.0;
        let Some(mut pools) = world.get_mut::<HarmonyPools>(caster) else {
            return;
        };
        let used = (amount * percent).min(pools.spender);
        pools.spender -= used;
        let depleted = pools.spender <= 0.0;
        if used > 0.0 {
            apply_damage(world, caster, target, used);
        }
        if depleted {
            auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
                aura.spell_id == SPENDER && aura.caster == caster
            });
        }
    } else {
        let percent = script_effect_value(world, caster, TALENT, effect) / 100.0;
        let max = world
            .get::<Health>(caster)
            .expect("Harmony caster health")
            .max;
        let current = world
            .get::<HarmonyPools>(caster)
            .map_or(0.0, |pools| pools.accumulator);
        let next = (current + amount * percent).min(max);
        if let Some(mut pools) = world.get_mut::<HarmonyPools>(caster) {
            pools.accumulator = next;
        } else {
            world.entity_mut(caster).insert(HarmonyPools {
                accumulator: next,
                spender: 0.0,
            });
        }
        if !auras::has_aura(world, caster, ACCUMULATOR) {
            apply(world, caster, ACCUMULATOR);
        }
    }
}

fn apply_damage(world: &mut World, caster: Entity, target: Entity, amount: f32) {
    let info = world
        .resource::<SpellStore>()
        .get(DAMAGE)
        .expect("native Harmony damage");
    let duration = info
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .expect("native Harmony duration")
        .duration_ms;
    let period = info
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Harmony tick")
        .aura_period_ms;
    assert!(duration > 0 && period > 0, "native Harmony tick timing");
    let ticks = duration as f32 / period as f32;
    spell_cast::cast_triggered_with_effect0_amount(
        world,
        caster,
        DAMAGE,
        Some(target),
        1,
        amount / ticks,
    );
}
