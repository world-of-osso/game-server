//! SimC76f8118 sc_druid.cpp2153–2181,5356–5390,14298–14307.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, script_registry::SpellScriptHook};

const TALENT: u32 = 429532;
const DEBUFF: u32 = 430589;
const SOURCES: &[u32] = &[1270292, 414613, 211545, 274283];

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LandedDamagePolicy(after_damage)];

fn after_damage(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: Option<u32>,
    damage: f32,
    _periodic: bool,
) {
    if damage <= 0.0 || !spell.is_some_and(|id| SOURCES.contains(&id)) {
        return;
    }
    if !world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == TALENT && aura.caster == caster)
    }) {
        return;
    }
    spell_cast::cast_triggered(world, caster, DEBUFF, Some(target), 1);
}
