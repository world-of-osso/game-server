//! SimC76f8118 sc_druid.cpp10791–10795; native1262034: stronger Mark only on its owner.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::script_effect_value};

const MARK: u32 = 1126;
const TALENT: u32 = 1262034;
pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EarlyAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(MARK).or_default().on_apply = Some(on_change);
    let talent = scripts.0.entry(TALENT).or_default();
    talent.on_apply = Some(on_change);
    talent.on_remove = Some(on_remove);
}

fn on_change(world: &mut World, unit: Entity, _: &AuraInstance) {
    update_mark(world, unit);
}
fn on_remove(world: &mut World, unit: Entity, _: &AuraInstance, _: RemoveReason) {
    update_mark(world, unit);
}

fn update_mark(world: &mut World, unit: Entity) {
    let Some((instance, caster, bonus)) = world.get::<ActiveAuras>(unit).and_then(|auras| {
        let mark = auras.iter().find(|aura| aura.spell_id == MARK)?;
        let bonus = auras
            .iter()
            .find(|aura| aura.spell_id == TALENT && aura.caster == unit)
            .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
            .map_or(0.0, |effect| effect.amount);
        Some((mark.instance_id, mark.caster, bonus))
    }) else {
        return;
    };
    let base = script_effect_value(world, caster, MARK, 0);
    auras::set_effect_amount(world, unit, instance, 0, base * (1.0 + bonus / 100.0));
}
