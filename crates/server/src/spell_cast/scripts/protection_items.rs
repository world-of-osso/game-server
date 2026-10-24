//! Pinned SimC76f8118 decisions absent from TC scripts/native trigger edges.
//! Native equipment, role expressions, proc masks/RPPM and child effects remain authoritative.
use crate::auras::{AuraInstance, procs::AuraEvent};
use crate::spell_cast::{self, TargetPlan};
use crate::spell_info::SpellInfo;
use bevy::prelude::*;

pub(crate) const GUILLOTINE: u32 = 1291728;
const GUILLOTINE_DAMAGE: u32 = 1306604;
pub(super) const PERFECTED_GUILLOTINE: u32 = 1306624;
const PERFECTED_SET: u32 = 1291726;
const PROTECTION_FOUR_PIECE: u32 = 1296648;
const BLOODY_REBUKE: u32 = 1300690;
const FREE_REVENGE: u32 = 5302;
pub(crate) const VENOMFANG_DRIVER: u32 = 1291718;
pub(crate) const VENOMFANG: u32 = 1306635;
const VENOMFANG_BURST: u32 = 1306639;

/// SimC dbc_proc_callback.cpp186–190: the callback cannot trigger itself.
pub(crate) fn check_venomfang_proc(_world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.actor == aura.caster && !matches!(event.spell_id, Some(VENOMFANG | VENOMFANG_BURST))
}

fn venomfang_amounts(world: &World, driver: &AuraInstance) -> Result<(f32, f32), String> {
    let role = crate::item_proc_data::role_multiplier(world, driver.caster, VENOMFANG_DRIVER)?;
    let amount = |index| {
        driver
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .map(|effect| effect.amount * role)
            .ok_or_else(|| format!("native Venomfang driver lacks scaled effect{index}"))
    };
    Ok((amount(0)?, amount(1)?))
}

fn snapshot_equipped_venomfang(world: &World, caster: Entity) -> Result<(f32, f32), String> {
    let driver = world
        .get::<crate::auras::ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == VENOMFANG_DRIVER))
        .ok_or_else(|| "perfected Guillotine has no native equipped Venomfang driver".to_owned())?;
    venomfang_amounts(world, driver)
}

fn cast_poison_snapshot(
    world: &mut World,
    caster: Entity,
    target: Option<Entity>,
    depth: u8,
    amounts: (f32, f32),
) {
    spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        VENOMFANG,
        target,
        depth,
        spell_cast::TriggeredSpellOverrides {
            effect0_amount: Some(amounts.0),
            venomfang_amounts: Some(amounts),
            ..Default::default()
        },
    );
}

/// VENOMFANG_EQUIPPED_DRIVER: unique_gear_midnight.cpp6283–6292,6379–6383.
pub(crate) fn cast_venomfang(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    event: &AuraEvent,
) {
    match venomfang_amounts(world, aura) {
        Ok(amounts) => cast_poison_snapshot(world, unit, event.target, 1, amounts),
        Err(error) => warn!("Venomfang refused: {error}"),
    }
}

/// PERFECTED_GUILLOTINE_POISON_IMPACT: unique_gear_midnight.cpp6347–6350;
/// action.cpp4486–4503 executes the precreated action on every landed recipient.
pub(super) fn perfected_guillotine_poison(
    world: &mut World,
    ctx: &spell_cast::effects::EffectCtx,
    target: Entity,
) {
    let Some(amounts) = ctx.spell.cast_venomfang_amounts else {
        warn!("Perfected Guillotine poison refused: missing native launch snapshot");
        return;
    };
    cast_poison_snapshot(world, ctx.caster, Some(target), ctx.depth + 1, amounts);
}

/// VENOMFANG_STACK_EXPIRY: unique_gear_midnight.cpp6307–6315. Native effect64
/// supplies the child edge; each asynchronous expiry, not launch, dispatches it.
pub(super) fn defer_venomfang_burst(
    _world: &mut World,
    _ctx: &spell_cast::effects::EffectCtx,
    effect: &crate::spell_info::SpellEffect,
    _target: Entity,
) {
    assert_eq!(
        effect.trigger_spell, VENOMFANG_BURST,
        "native Venomfang expiry child changed"
    );
}

pub(crate) fn burst_venomfang(world: &mut World, target: Entity, aura: &AuraInstance) {
    let Some(amount) = aura.venomfang_burst_amount else {
        warn!("Venomfang burst refused: missing native item snapshot");
        return;
    };
    let child = world
        .resource::<crate::spell_info::SpellStore>()
        .get(VENOMFANG)
        .and_then(|spell| spell.effects.iter().find(|effect| effect.index == 1))
        .expect("native Venomfang requires expiry effect1")
        .trigger_spell;
    spell_cast::cast_triggered_with_effect0_amount(
        world,
        aura.caster,
        child,
        Some(target),
        1,
        amount,
    );
}

/// SimC dbc_proc_callback.cpp186–190: exclude the callback's own action, not unrelated procs.
pub(crate) fn check_guillotine_proc(
    _world: &World,
    aura: &AuraInstance,
    event: &AuraEvent,
) -> bool {
    event.actor == aura.caster
        && !matches!(
            event.spell_id,
            Some(GUILLOTINE_DAMAGE | PERFECTED_GUILLOTINE)
        )
}

/// GUILLOTINE_EQUIPPED_DRIVER: unique_gear_midnight.cpp6334–6378.
pub(crate) fn cast_guillotine(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    event: &AuraEvent,
) {
    match snapshot_guillotine_cast(world, unit, aura) {
        Ok((child, values)) => {
            spell_cast::cast_triggered_with_overrides(world, unit, child, event.target, 1, values);
        }
        Err(error) => warn!("Guillotine refused: {error}"),
    }
}

fn snapshot_guillotine_cast(
    world: &World,
    unit: Entity,
    aura: &AuraInstance,
) -> Result<(u32, spell_cast::TriggeredSpellOverrides), String> {
    let role = crate::item_proc_data::role_multiplier(world, unit, GUILLOTINE)?;
    let perfected = crate::item_proc_data::has_equipped_bonus(world, unit, PERFECTED_SET)?;
    let base_amount = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native equipped Guillotine requires item-scaled effect0")
        .amount
        * role;
    let (child, amount, venomfang_amounts) = if perfected {
        let multiplier = super::script_effect_value(world, unit, PERFECTED_SET, 1) / 100.0;
        (
            PERFECTED_GUILLOTINE,
            base_amount * multiplier,
            Some(snapshot_equipped_venomfang(world, unit)?),
        )
    } else {
        (GUILLOTINE_DAMAGE, base_amount, None)
    };
    let values = spell_cast::TriggeredSpellOverrides {
        effect0_amount: Some(amount),
        venomfang_amounts,
        ..Default::default()
    };
    Ok((child, values))
}

/// GUILLOTINE_TARGET_HEALTH_SNAPSHOT: unique_gear_midnight.cpp6357–6365;
/// action.cpp1860–1915 calculates each recipient's amount before travel.
/// Reuse the existing per-target scalar snapshot, preserving native chain multipliers.
pub(in crate::spell_cast) fn snapshot_guillotine_health(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    plan: &mut TargetPlan,
) {
    if !matches!(spell.id, GUILLOTINE_DAMAGE | PERFECTED_GUILLOTINE) {
        return;
    }
    let percent = super::script_effect_value(world, caster, GUILLOTINE, 1);
    for planned in plan
        .iter_mut()
        .filter(|planned| spell.effects[planned.index].index == 0)
    {
        planned.chain_multipliers = planned
            .units
            .iter()
            .map(|&target| {
                let health = world
                    .get::<shared::components::Health>(target)
                    .expect("admitted Guillotine recipient requires Health");
                assert!(
                    health.max > 0.0,
                    "admitted Guillotine recipient requires positive max health"
                );
                let missing = (100.0 * (1.0 - health.current / health.max)).clamp(0.0, 100.0);
                (
                    target,
                    planned.chain_multiplier(target) * (1.0 + percent / 100.0 * missing),
                )
            })
            .collect();
    }
}

/// BLOODY_REBUKE_FREE_REVENGE: sc_warrior.cpp6065–6066, before native5302 consumption.
pub(super) fn revenge_bloody_rebuke(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if crate::auras::has_aura(world, caster, FREE_REVENGE) {
        cast_bloody_rebuke(world, caster, target, depth);
    }
}

/// BLOODY_REBUKE_RAVAGER_TICK: sc_warrior.cpp5843–5844.
pub(super) fn ravager_bloody_rebuke(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    cast_bloody_rebuke(world, caster, target, depth);
}

fn cast_bloody_rebuke(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    match crate::item_proc_data::has_equipped_bonus(world, caster, PROTECTION_FOUR_PIECE) {
        Ok(true) => {
            spell_cast::cast_triggered(world, caster, BLOODY_REBUKE, Some(target), depth + 1)
        }
        Ok(false) => {}
        Err(error) => warn!("Bloody Rebuke refused: {error}"),
    }
}
