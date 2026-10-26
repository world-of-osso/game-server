//! SimC76f8118 unique_gear_midnight.cpp5233–5280: Zatha'tek's native damage and owned Tear.
use super::combat;
use bevy::prelude::*;
use combat::auras::procs::AuraEvent;
use combat::auras::scripts::AuraScripts;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraInstance, AuraSource};
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::{DamageAmountHook, SpellScriptHook};
use combat::spell_cast::{self, TriggeredSpellOverrides};
use combat::spell_info::SpellStore;

const DRIVER: u32 = 1298023;
const DAMAGE: u32 = 1305395;
const TEAR: u32 = 1305391;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::DamageAmount(payload),
    SpellScriptHook::ResolvedDamage(apply_tear),
];

fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(DRIVER).or_default();
    script.check_proc = Some(admits_proc);
    script.on_damage_proc = Some(proc_weapon);
}

fn admits_proc(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    // dbc_proc_callback.cpp186–190 excludes only this callback's own action.
    if event.actor != aura.caster || event.damage <= 0.0 || event.spell_id == Some(DAMAGE) {
        return false;
    }
    event.target.is_some()
        && auras::procs::select_proc_trigger_target(world, aura.caster, DAMAGE, event.target)
            .is_ok()
}

fn proc_weapon(world: &mut World, unit: Entity, driver: &AuraInstance, event: &AuraEvent) {
    let amount = driver
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Zatha'tek budget effect")
        .amount;
    spell_cast::cast_triggered_with_overrides(
        world,
        unit,
        DAMAGE,
        event.target,
        1,
        TriggeredSpellOverrides {
            effect0_amount: Some(amount),
            item_level: driver.cast_item_level,
            ..Default::default()
        },
    );
}

fn payload(spell: u32) -> Option<DamageAmountHook> {
    (spell == DAMAGE).then_some(amplify_payload)
}

fn amplify_payload(world: &World, caster: Entity, target: Entity, amount: f32) -> f32 {
    let role = combat::item_proc_data::role_multiplier(world, caster, DRIVER)
        .expect("native Zatha'tek role expression");
    let tear = world.get::<ActiveAuras>(target).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == TEAR && aura.caster == caster)
    });
    let Some(tear) = tear else {
        return amount * role;
    };
    let percent = tear
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Necrotic Tear stack value")
        .amount;
    amount * role * (1.0 + f32::from(tear.stacks) * percent / 100.0)
}

fn tear_percent(world: &World) -> f32 {
    world
        .resource::<SpellStore>()
        .get(DRIVER)
        .expect("native Zatha'tek driver")
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("native Necrotic Tear percentage")
        .base_points
}

fn apply_tear(world: &mut World, ctx: &EffectCtx, target: Entity, _crit: bool, _amount: f32) {
    if ctx.spell.id != DAMAGE {
        return;
    }
    let Some(level) = ctx.spell.cast_item_level else {
        return;
    };
    // generic_proc_t::impact deals damage before triggering its recipient buff.
    let mut spell = world
        .resource::<SpellStore>()
        .get(TEAR)
        .expect("native Necrotic Tear")
        .clone();
    spell.cast_item_level = Some(level);
    spell
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("native Necrotic Tear amount effect")
        .base_points = tear_percent(world);
    auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: ctx.caster,
            target,
            spell_id: TEAR,
            effect_mask: auras::aura_effect_mask(&spell),
            source: AuraSource::Cast,
        },
        &spell,
    );
}
