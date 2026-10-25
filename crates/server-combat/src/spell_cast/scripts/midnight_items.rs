//! SimC76f8118 unique_gear_midnight.cpp: item relationships absent from native trigger edges.
//! Equipment eligibility, scaling, proc rates and payloads always come from native catalogs.
use crate::auras::{
    ActiveAuras, AuraInstance, RemoveReason,
    procs::AuraEvent,
    scripts::{AuraScript, AuraScripts},
};
use crate::spell_cast::{self, TriggeredSpellOverrides};
use bevy::prelude::*;

const VENOM_DRIVER: u32 = 1291718;
const VENOM: u32 = 1306635;
const BURST: u32 = 1306639;
const HEART_VALUES: u32 = 1297760;
const HEART: u32 = 1297761;
const MORSEL: u32 = 1305374;
const DEVOURED: u32 = 1305376;
const FIRE: u32 = 1279599;
const FIRE_DAMAGE: u32 = 1286970;
const LINGERING: u32 = 1287555;
const LINGERING_DAMAGE: u32 = 1287663;

pub(crate) fn register(scripts: &mut AuraScripts) {
    for driver in [VENOM_DRIVER, HEART, FIRE] {
        scripts.0.insert(
            driver,
            AuraScript {
                check_proc: Some(check_proc),
                on_damage_proc: Some(proc_item),
                on_apply: (driver == HEART).then_some(apply_heart),
                on_remove: (driver == HEART).then_some(remove_heart),
                ..Default::default()
            },
        );
    }
    scripts.0.insert(
        VENOM,
        AuraScript {
            on_remove: Some(remove_venom),
            on_stack_loss: Some(lose_venom),
            ..Default::default()
        },
    );
}

fn native_amount(
    world: &World,
    caster: Entity,
    id: u32,
    index: u8,
    item_level: Option<u16>,
) -> f32 {
    let mut spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
        .expect("native item value spell")
        .clone();
    spell.cast_item_level = item_level;
    let effect = spell
        .effects
        .iter()
        .find(|e| e.index == index)
        .expect("native item value effect");
    spell_cast::effects::modded_base_points(world, caster, &spell, effect)
}

fn role(world: &World, caster: Entity, driver: u32) -> f32 {
    let amount = if driver == HEART {
        crate::item_proc_data::default_role_multiplier(world, caster)
    } else {
        crate::item_proc_data::role_multiplier(world, caster, driver)
    };
    amount.expect("native item role expression")
}

fn check_proc(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    // dbc_proc_callback.cpp186–190: exclude callback's own action, not every item proc.
    if event.actor != aura.caster || event.damage <= 0.0 {
        return false;
    }
    let own = match aura.spell_id {
        VENOM_DRIVER => VENOM,
        HEART => MORSEL,
        FIRE => FIRE_DAMAGE,
        _ => unreachable!(),
    };
    // Attackable neutral training dummies need not be faction-hostile. Reuse the
    // ordinary native proc recipient predicate, including its no-self/null rules.
    event.spell_id != Some(own)
        && event.target.is_some()
        && crate::auras::procs::select_proc_trigger_target(world, aura.caster, own, event.target)
            .is_ok()
}

fn cast_amount(
    world: &mut World,
    caster: Entity,
    target: Entity,
    id: u32,
    amount: f32,
    level: Option<u16>,
    mask: Option<u32>,
) {
    spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        id,
        Some(target),
        1,
        TriggeredSpellOverrides {
            effect0_amount: Some(amount),
            item_level: level,
            effect_mask: mask,
            ..Default::default()
        },
    );
}

fn proc_item(world: &mut World, caster: Entity, aura: &AuraInstance, event: &AuraEvent) {
    let target = event.target.expect("qualified item proc recipient");
    match aura.spell_id {
        // unique_gear_midnight.cpp6280–6330: asynchronous poison, per-stack tick and expiry burst.
        VENOM_DRIVER => apply_venom(world, caster, target, aura.cast_item_level),
        // unique_gear_midnight.cpp3750–3803: on-use buff activates the native RPPM callback.
        HEART => {
            let amount = native_amount(world, caster, HEART_VALUES, 1, aura.cast_item_level)
                * role(world, caster, HEART);
            cast_amount(
                world,
                caster,
                target,
                MORSEL,
                amount,
                aura.cast_item_level,
                None,
            );
            let stack = native_amount(world, caster, HEART_VALUES, 2, aura.cast_item_level);
            apply_primary_aura(world, caster, DEVOURED, stack, aura.cast_item_level, true);
        }
        FIRE => proc_fire(world, caster, target),
        _ => unreachable!(),
    }
}

pub(super) fn apply_perfected_venom(world: &mut World, caster: Entity, target: Entity) {
    let driver = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|a| a.spell_id == VENOM_DRIVER))
        .cloned();
    if let Some(driver) = driver {
        apply_venom(world, caster, target, driver.cast_item_level);
    }
}

fn apply_venom(world: &mut World, caster: Entity, target: Entity, level: Option<u16>) {
    let amount =
        native_amount(world, caster, VENOM_DRIVER, 0, level) * role(world, caster, VENOM_DRIVER);
    // The native secondary trigger encodes the burst payload, not its application timing.
    cast_amount(world, caster, target, VENOM, amount, level, Some(1));
}

fn lose_venom(world: &mut World, target: Entity, aura: &AuraInstance, lost: u8) {
    let amount = native_amount(world, aura.caster, VENOM_DRIVER, 1, aura.cast_item_level)
        * role(world, aura.caster, VENOM_DRIVER);
    for _ in 0..lost {
        cast_amount(
            world,
            aura.caster,
            target,
            BURST,
            amount,
            aura.cast_item_level,
            None,
        );
    }
}
fn remove_venom(world: &mut World, target: Entity, aura: &AuraInstance, _reason: RemoveReason) {
    if aura.stacks > 0 {
        lose_venom(world, target, aura, aura.stacks);
    }
}

fn primary_stat(world: &World, caster: Entity) -> Option<crate::stats::PrimaryStat> {
    let spec = world.get::<crate::class_progression::ActiveSpec>(caster)?.0;
    let spec = world
        .resource::<crate::spell_info::ClassSpells>()
        .classes
        .iter()
        .flat_map(|class| &class.specs)
        .find(|info| info.spec_id == spec)?;
    Some(crate::stats::PrimaryStat::from_priority(
        spec.primary_stat_priority,
    ))
}

fn apply_heart(world: &mut World, caster: Entity, aura: &AuraInstance) {
    let amount = native_amount(world, caster, HEART_VALUES, 0, aura.cast_item_level);
    let strength = primary_stat(world, caster) == Some(crate::stats::PrimaryStat::Strength);
    if let Some(mut auras) = world.get_mut::<ActiveAuras>(caster) {
        let live = auras.get_mut(aura.instance_id).expect("applied Heart aura");
        for effect in &mut live.effects {
            effect.amount = if effect.index == u8::from(!strength) {
                amount
            } else {
                0.0
            };
        }
    }
}
fn remove_heart(world: &mut World, caster: Entity, _aura: &AuraInstance, _reason: RemoveReason) {
    crate::auras::remove_auras(world, caster, RemoveReason::Consumed, |a| {
        a.spell_id == DEVOURED && a.caster == caster
    });
}
fn apply_primary_aura(
    world: &mut World,
    caster: Entity,
    id: u32,
    amount: f32,
    level: Option<u16>,
    inverse: bool,
) {
    let mut spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(id)
        .expect("native primary aura")
        .clone();
    spell.cast_item_level = level;
    let strength = primary_stat(world, caster) == Some(crate::stats::PrimaryStat::Strength);
    let selected = u8::from(strength == inverse);
    for effect in &mut spell.effects {
        effect.base_points = if effect.index == selected {
            amount
        } else {
            0.0
        };
    }
    crate::auras::apply_aura_with_spell(
        world,
        &crate::auras::AuraApplication {
            caster,
            target: caster,
            spell_id: id,
            effect_mask: 3,
            source: crate::auras::AuraSource::Cast,
        },
        &spell,
    );
}

fn proc_fire(world: &mut World, caster: Entity, target: Entity) {
    // unique_gear_midnight.cpp6063–6123: Omnium scales its native coefficient at fixed ilvl289.
    let store = world.resource::<crate::spell_info::SpellStore>();
    let coefficient_id = store
        .get(FIRE)
        .unwrap()
        .effects
        .iter()
        .find(|e| e.index == 1)
        .unwrap()
        .trigger_spell;
    let coefficient = store
        .get(coefficient_id)
        .unwrap()
        .effects
        .iter()
        .find(|e| e.index == 0)
        .unwrap();
    let scale = (world
        .resource::<crate::item_stats::ItemScalingTables>()
        .spell_budgets
        .get(&289)
        .expect("native Omnium budget")[1]
        * coefficient.coefficient)
        .floor()
        / 100.0;
    let amount = native_amount(world, caster, FIRE_DAMAGE, 1, None) * scale;
    let overload = if crate::auras::has_aura(world, caster, 1279614) {
        1.0 + native_amount(world, caster, 1279614, 0, None) / 100.0
    } else {
        1.0
    };
    cast_amount(
        world,
        caster,
        target,
        FIRE_DAMAGE,
        amount * overload,
        None,
        None,
    );
    proc_omnium_stat(world, caster, coefficient_id);
    if crate::auras::has_aura(world, caster, LINGERING) {
        let duration = world
            .resource::<crate::spell_info::SpellStore>()
            .get(LINGERING_DAMAGE)
            .unwrap()
            .misc
            .as_ref()
            .unwrap()
            .duration
            .unwrap()
            .duration_ms;
        let residual = if crate::auras::has_aura(world, caster, 1279615) {
            1.0 + native_amount(world, caster, 1279615, 0, None) / 100.0
        } else {
            1.0
        };
        let tick = native_amount(world, caster, LINGERING_DAMAGE, 1, None) * scale
            / (duration as f32 / 1000.0)
            * residual;
        cast_amount(world, caster, target, LINGERING_DAMAGE, tick, None, None);
    }
}

// unique_gear_midnight.cpp6148–6170: select the first actual secondary rune.
fn proc_omnium_stat(world: &mut World, caster: Entity, coefficient_id: u32) {
    let runes = [
        (1279613, 1287770),
        (1279612, 1287771),
        (1279610, 1287774),
        (1279609, 1287772),
    ];
    let Some((_, buff)) = runes
        .into_iter()
        .find(|(driver, _)| crate::auras::has_aura(world, caster, *driver))
    else {
        return;
    };
    let effect = world
        .resource::<crate::spell_info::SpellStore>()
        .get(coefficient_id)
        .unwrap()
        .effects
        .iter()
        .find(|e| e.index == 2)
        .unwrap();
    let tables = world.resource::<crate::item_stats::ItemScalingTables>();
    let coefficient = (tables.spell_budgets.get(&289).unwrap()[2]
        * tables.rating_multiplier[288][2]
        * effect.coefficient)
        .floor()
        / 100.0;
    let amount = native_amount(world, caster, buff, 1, None) * coefficient;
    cast_amount(world, caster, caster, buff, amount, None, None);
}
