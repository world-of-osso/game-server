//! SimC76f8118 sc_monk.cpp4709–4885 and 4890–4993: Aspect of Harmony pools.
use bevy::prelude::*;
use shared::components::{ActiveSpec, Health};
use shared::protocol::{CombatLogEvent, CombatLogKind};

use super::combat;
use combat::auras::{
    self, AuraApplication, AuraInstance, AuraSource, RemoveReason,
    harmony::HarmonyPools,
    scripts::{AuraScript, AuraScripts},
};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{
    self,
    script_registry::SpellScriptHook,
    scripts::{AfterCastHook, script_effect_value},
};
use combat::spell_info::{SpellInfo, SpellStore};
use combat::stats::{CharacterStats, StatModifiers, WeaponAttackType};

const TALENT: u32 = 450508;
const ACCUMULATOR: u32 = 450521;
const SPENDER: u32 = 450711;
const DAMAGE: u32 = 450763;
const COALESCENCE: u32 = 450529;
const THOUSAND_STRIKES: u32 = 450965;
const PURIFIED_SPIRIT: u32 = 450867;
const PURIFIED_DAMAGE: u32 = 450820;
const CLARITY_OF_PURPOSE: u32 = 451017;
const PURIFYING_BREW: u32 = 119582;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::MidAuraRegistration(register_spender),
    SpellScriptHook::HealthChangeObserved(observe),
    SpellScriptHook::AfterResourceValues(spender_hook),
];

fn register_spender(scripts: &mut AuraScripts) {
    scripts.0.insert(
        SPENDER,
        AuraScript {
            on_remove: Some(expire_spender),
            ..Default::default()
        },
    );
}

fn expire_spender(world: &mut World, caster: Entity, _: &AuraInstance, _: RemoveReason) {
    release_purified_spirit(world, caster);
}

fn release_purified_spirit(world: &mut World, caster: Entity) {
    if !has_talent(world, caster, PURIFIED_SPIRIT) {
        return;
    }
    let Some(mut pools) = world.get_mut::<HarmonyPools>(caster) else {
        return;
    };
    let amount = std::mem::take(&mut pools.spender);
    if amount <= 0.0 {
        return;
    }
    let Some(target) = world
        .get::<combat::networking::CombatTarget>(caster)
        .and_then(|target| target.0)
    else {
        return;
    };
    let spell = world
        .resource::<SpellStore>()
        .get(PURIFIED_DAMAGE)
        .expect("native Purified Spirit damage")
        .clone();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Purified Spirit tick");
    let duration = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .expect("native Purified Spirit duration")
        .duration_ms;
    assert!(
        duration > 0 && effect.aura_period_ms > 0,
        "native Purified Spirit timing"
    );
    let targets =
        spell_cast::targets::select_targets(world, caster, Some(target), &spell, effect, 0, false)
            .expect("native Purified Spirit target selection");
    if targets.is_empty() {
        return;
    }
    let ticks = duration as f32 / effect.aura_period_ms as f32;
    spell_cast::cast_triggered_with_effect0_amount(
        world,
        caster,
        PURIFIED_DAMAGE,
        Some(target),
        1,
        amount / ticks / targets.len() as f32,
    );
}

fn has_talent(world: &World, caster: Entity, spell: u32) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == spell)
    })
}

fn selected(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 268)
        && has_talent(world, caster, TALENT)
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
    match spell {
        322507 | 1241059 => Some(start_spender),
        PURIFYING_BREW => Some(add_clarity),
        _ => None,
    }
}

fn add_clarity(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !selected(world, caster)
        || !has_talent(world, caster, CLARITY_OF_PURPOSE)
        || auras::has_aura(world, caster, SPENDER)
    {
        return;
    }
    let stats = world.get::<CharacterStats>(caster).expect("Harmony caster stats");
    let attack_power = stats.total_attack_power(
        WeaponAttackType::MainHand,
        world.get::<StatModifiers>(caster),
    );
    let versatility = 1.0 + stats.versatility_pct / 100.0;
    let coefficient = script_effect_value(world, caster, CLARITY_OF_PURPOSE, 0) / 100.0;
    add_accumulator(world, caster, attack_power * versatility * coefficient);
}

fn add_accumulator(world: &mut World, caster: Entity, amount: f32) {
    let max = world.get::<Health>(caster).expect("Harmony caster health").max;
    let current = world.get::<HarmonyPools>(caster).map_or(0.0, |pools| pools.accumulator);
    let next = (current + amount).min(max);
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

fn start_spender(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !selected(world, caster) {
        return;
    }
    if auras::has_aura(world, caster, SPENDER) {
        release_purified_spirit(world, caster);
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
        let coalescence = has_talent(world, caster, COALESCENCE);
        // sc_monk.cpp4832–4833/4919–4924: Keg spends, rather than generates.
        if coalescence && event.spell_id == Some(121253) {
            let percent = script_effect_value(world, caster, COALESCENCE, 1) / 100.0;
            let Some(mut pools) = world.get_mut::<HarmonyPools>(caster) else {
                return;
            };
            let used = (amount * percent).min(pools.accumulator);
            pools.accumulator -= used;
            let depleted = pools.accumulator <= 0.0;
            if used > 0.0 {
                apply_damage(world, caster, target, used);
            }
            if depleted {
                auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
                    aura.spell_id == ACCUMULATOR && aura.caster == caster
                });
            }
            return;
        }
        let mut percent = script_effect_value(world, caster, TALENT, effect) / 100.0;
        if coalescence {
            percent *= 1.0 + script_effect_value(world, caster, COALESCENCE, 2) / 100.0;
        }
        // sc_monk.cpp4859–4867: selected Way amplifies its listed direct actions.
        if matches!(event.spell_id, Some(205523 | 100780 | 185099))
            && has_talent(world, caster, THOUSAND_STRIKES)
        {
            let effect = world
                .resource::<SpellStore>()
                .get(THOUSAND_STRIKES)
                .expect("native Way of a Thousand Strikes")
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .expect("native Way of a Thousand Strikes effect")
                .base_points;
            percent *= 1.0 + effect / 100.0;
        }
        add_accumulator(world, caster, amount * percent);
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
