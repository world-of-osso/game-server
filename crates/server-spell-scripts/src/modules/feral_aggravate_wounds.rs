//! Feral combo actions extend the caster's existing Dreadful Wounds.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

const TALENT: u32 = 441829;
const WOUND: u32 = 441812;
// SimC76f8118 sc_druid.cpp3705,3765,4058,4114,4457,4507,4518,
// 4638,4737,4780,4833: Feral CP generators/spenders and the two Frenzies.
const ACTIONS: &[u32] = &[
    5221, 1822, 155625, 106785, 202028, 1079, 22568, 441591, 22570, 285381, 1243807, 274837,
];

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::BeforeNativeOnCast(extend_wounds)];

fn collect_owned_wounds(world: &mut World, caster: Entity) -> Vec<Entity> {
    world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .filter_map(|(unit, auras)| {
            auras
                .iter()
                .any(|aura| aura.spell_id == WOUND && aura.caster == caster)
                .then_some(unit)
        })
        .collect()
}

// SimC76f8118 sc_druid.cpp2119–2146 extends all existing owned wounds before
// the admitted action executes, even if its impact later misses.
fn extend_wounds(world: &mut World, caster: Entity, spell: &SpellInfo, _: u8) {
    if !ACTIONS.contains(&spell.id)
        || world
            .get::<ActiveSpec>(caster)
            .is_none_or(|spec| spec.0 != 103)
    {
        return;
    }
    let Some((extension, cap)) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|a| a.spell_id == TALENT && a.caster == caster)
            .map(|a| {
                let value = |index| {
                    a.effects
                        .iter()
                        .find(|effect| effect.index == index)
                        .expect("native Aggravate Wounds effect")
                        .amount
                };
                (
                    value(1).max(0.0) as u32,
                    (value(2).max(0.0) * 1000.0) as u32,
                )
            })
    }) else {
        return;
    };
    for target in collect_owned_wounds(world, caster) {
        combat::auras::extend_owned_duration_capped(
            world,
            caster,
            target,
            &[WOUND],
            extension,
            cap,
        );
    }
}
