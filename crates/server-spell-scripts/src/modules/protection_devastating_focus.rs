//! SimC76f8118 sc_warrior.cpp1913–1947: Devastator marks its latest target.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};

const TALENT: u32 = 1264330;
const FOCUS: u32 = 1277983;
const DEVASTATOR: u32 = 236282;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(on_devastator)];

fn on_devastator(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool, _: f32) {
    if ctx.spell.id != DEVASTATOR
        || !world.get::<TraitLoadout>(ctx.caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == TALENT)
        })
    {
        return;
    }
    let previous: Vec<_> = world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .filter(|(unit, auras)| {
            *unit != target
                && auras
                    .iter()
                    .any(|aura| aura.caster == ctx.caster && aura.spell_id == FOCUS)
        })
        .map(|(unit, _)| unit)
        .collect();
    for unit in previous {
        auras::remove_auras(world, unit, RemoveReason::Replaced, |aura| {
            aura.caster == ctx.caster && aura.spell_id == FOCUS
        });
    }
    auras::apply_aura(
        world,
        &AuraApplication {
            caster: ctx.caster,
            target,
            spell_id: FOCUS,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}
