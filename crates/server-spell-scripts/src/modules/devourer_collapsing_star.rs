//! SimC76f8118 sc_demon_hunter.cpp6529–6553: Collapsing Star spends its souls.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterNativeLaunch(after_launch)];

fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != 1221150 || world.get::<ActiveSpec>(caster).map(|spec| spec.0) != Some(1480) {
        return;
    }
    let Some(stacks) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras.iter().find(|aura| aura.caster == caster && aura.spell_id == 1227702)
            .map(|aura| aura.stacks)
    }) else {
        return;
    };
    let cost = script_effect_value(world, caster, 1221167, 0) as u8;
    let remaining = stacks.saturating_sub(cost);
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.caster == caster && (aura.spell_id == 1221171 || (aura.spell_id == 1227702 && remaining == 0))
    });
    if remaining > 0 {
        assert!(auras::set_owned_stacks(world, caster, caster, 1227702, remaining), "native Collapsing Star soul stack");
    }
    // SimC76f8118 cpp6464–6474,6532–6533 executes the native damage child.
    let target = spell.cast_primary_target.expect("native Collapsing Star target");
    if !spell_cast::try_cast_triggered(world, caster, 1221162, Some(target), 1) {
        warn!(?caster, ?target, "native Collapsing Star damage 1221162 was refused");
    }
}
