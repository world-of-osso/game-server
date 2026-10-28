//! SimC76f8118 sc_druid.cpp2303–2328,5489,11444–11445: Gore procs on Bear impacts.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;
use shared::components::PowerType;

#[derive(Component)]
struct GoreReadyAt(f64);

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterDirectDamageBeforeItems(roll_gore)];

fn roll_gore(world: &mut World, ctx: &EffectCtx<'_>) {
    if !matches!(ctx.spell.id, 77758 | 6807 | 400254 | 213771 | 164812) {
        return;
    }
    let caster = ctx.caster;
    let Some(chance) = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|a| a.spell_id == 210706 && a.caster == caster)
        })
        .and_then(|a| a.effects.iter().find(|e| e.index == 0))
        .map(|e| e.amount)
    else {
        return;
    };
    let harnessed = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|a| a.spell_id == 1253035 && a.caster == caster)
    });
    let bonus = harnessed
        .and_then(|aura| {
            let threshold = aura.effects.iter().find(|effect| effect.index == 0)?.amount;
            let percent = aura.effects.iter().find(|effect| effect.index == 1)?.amount;
            let rage = spell_cast::powers::power_value(world, caster, PowerType::Rage)?.0 / 10.0;
            (matches!(ctx.spell.id, 6807 | 400254) && rage >= threshold).then_some(percent)
        })
        .unwrap_or(0.0);
    let now = spell_cast::now(world);
    if world
        .get::<GoreReadyAt>(caster)
        .is_some_and(|ready| ready.0 > now)
        || combat::random::random::<f32>() >= chance / 100.0 * (1.0 + bonus / 100.0)
    {
        return;
    }
    let cooldown = world
        .resource::<SpellStore>()
        .get(210706)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("Gore native proc recovery")
        .proc_category_recovery_ms
        .max(0) as f64
        / 1000.0;
    spell_cast::cast_triggered(world, caster, 93622, Some(caster), ctx.depth + 1);
    world.entity_mut(caster).insert(GoreReadyAt(now + cooldown));
}
