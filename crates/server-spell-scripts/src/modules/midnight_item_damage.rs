//! SimC76f8118 unique_gear_midnight.cpp5679–5748: native driver-owned item damage.
use super::combat;
use bevy::prelude::*;
use combat::auras::procs::AuraEvent;
use combat::auras::scripts::AuraScripts;
use combat::auras::{AuraInstance, procs};
use combat::spell_cast::effects::{self, EffectCtx, EffectHandler};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::{self, TriggeredSpellOverrides};
use combat::spell_info::{SpellEffect, SpellStore};

const LIGHTWOOD_DRIVER: u32 = 1296732;
const LIGHTWOOD_DAMAGE: u32 = 1296735;
const SWARM_DRIVER: u32 = 1310082;
const ACIDBROOD: u32 = 1310088;
const VOID_DRIVER: u32 = 1310208;
const VOID_ERUPTION: u32 = 1310209;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::EffectHandler(effect_handler),
];

fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(VOID_DRIVER).or_default();
    script.check_proc = Some(admits_void_proc);
    script.on_proc = Some(proc_void);
    let script = scripts.0.entry(LIGHTWOOD_DRIVER).or_default();
    script.check_proc = Some(admits_lightwood_proc);
    script.on_proc = Some(proc_lightwood);
}

fn admits_lightwood_proc(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.spell_id != Some(LIGHTWOOD_DAMAGE)
        && procs::select_proc_trigger_target(world, aura.caster, LIGHTWOOD_DAMAGE, event.target)
            .is_ok()
}

fn proc_lightwood(world: &mut World, unit: Entity, driver: &AuraInstance, target: Option<Entity>) {
    // SimC76f8118 unique_gear_midnight.cpp5219–5230 binds the native dummy
    // driver to its damage action; native proc timing and item level remain intact.
    spell_cast::cast_triggered_with_overrides(
        world,
        unit,
        LIGHTWOOD_DAMAGE,
        target,
        1,
        TriggeredSpellOverrides {
            item_level: driver.cast_item_level,
            ..Default::default()
        },
    );
}

fn admits_void_proc(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    // dbc_proc_callback.cpp186–190 excludes the callback's own action.
    event.spell_id != Some(VOID_ERUPTION)
        && combat::combat_refs::active_opponents(world, aura.caster)
            .next()
            .is_some()
        && procs::select_proc_trigger_target(world, aura.caster, VOID_ERUPTION, event.target)
            .is_ok()
}

fn proc_void(world: &mut World, unit: Entity, driver: &AuraInstance, target: Option<Entity>) {
    // The native proc manager owns chance/RPPM/ICD and the nullable event target.
    spell_cast::cast_triggered_with_overrides(
        world,
        unit,
        VOID_ERUPTION,
        target,
        1,
        TriggeredSpellOverrides {
            item_level: driver.cast_item_level,
            ..Default::default()
        },
    );
}

fn driver_for_payload(spell: u32) -> Option<(u32, bool)> {
    match spell {
        LIGHTWOOD_DAMAGE => Some((LIGHTWOOD_DRIVER, false)),
        ACIDBROOD => Some((SWARM_DRIVER, true)),
        VOID_ERUPTION => Some((VOID_DRIVER, true)),
        _ => None,
    }
}

fn effect_handler(spell: u32, index: u8) -> Option<EffectHandler> {
    (index == 0 && driver_for_payload(spell).is_some()).then_some(deal_native_item_damage)
}

fn native_driver_budget(
    world: &World,
    caster: Entity,
    driver: u32,
    level: u16,
    role_scaled: bool,
) -> f32 {
    let mut native = world
        .resource::<SpellStore>()
        .get(driver)
        .expect("native item damage driver")
        .clone();
    native.cast_item_level = Some(level);
    let effect = native
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native driver damage budget");
    let amount = effects::modded_base_points(world, caster, &native, effect);
    if role_scaled {
        let role = combat::item_proc_data::role_multiplier(world, caster, driver)
            .expect("native item damage role expression");
        amount * role
    } else {
        amount
    }
}

fn deal_native_item_damage(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    effect: &SpellEffect,
    target: Entity,
) {
    let Some(level) = ctx.spell.cast_item_level else {
        error!(
            spell = ctx.spell.id,
            "Native item payload lost its casting item level"
        );
        return;
    };
    let (driver, role_scaled) =
        driver_for_payload(ctx.spell.id).expect("registered native item payload");
    let mut payload = effect.clone();
    // These native damage leaves have zero BP/coefficients. SimC assigns driver
    // effect1 average; native class−9 item scaling and role data supply the value.
    payload.base_points = native_driver_budget(world, ctx.caster, driver, level, role_scaled);
    // Retain native selectors/AoE, crit and ordinary downstream damage stages.
    // Acidbrood still arrives through native missile1310083→1310088.
    // Pinned bugs=true intentionally omits Void's unimplemented Venomcursed bonus.
    effects::handler(effects::EFFECT_SCHOOL_DAMAGE).expect("school damage handler")(
        world, ctx, &payload, target,
    );
}
