//! Brewmaster MID2 ignited Keg and Scorched set effects.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

const TWO: u32 = 1296617;
const FOUR: u32 = 1296618;
const HOT_POTATO: u32 = 1301477;
const SHRAPNEL: u32 = 1301619;
const SCORCHED: u32 = 1301410;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(|spell| spell == TWO),
    SpellScriptHook::AfterForegroundSpecial(ignite),
    SpellScriptHook::BeforeNativeOnCast(launch_shrapnel),
    SpellScriptHook::ResolvedDamage(scorch),
];

fn equipped(world: &World, caster: Entity, spell: u32) -> bool {
    world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.spell_id == spell
                && aura.caster == caster
                && matches!(aura.source, AuraSource::ItemSet(_))
        })
    })
}

fn ignite(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id == 115181 && equipped(world, caster, TWO) {
        auras::apply_aura(world, &AuraApplication {
            caster, target: caster, spell_id: HOT_POTATO,
            effect_mask: u32::MAX, source: AuraSource::Cast,
        });
    }
}

fn launch_shrapnel(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if depth != 0 || spell.id != 121253 || !equipped(world, caster, TWO)
        || !auras::has_aura(world, caster, HOT_POTATO) {
        return;
    }
    let Some(target) = spell.cast_primary_target else { return; };
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == HOT_POTATO && aura.caster == caster
    });
    if !spell_cast::try_cast_triggered(world, caster, SHRAPNEL, Some(target), 1) {
        warn!(?caster, ?target, "Fiery Shrapnel refused");
    }
}

fn scorch(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool, _: f32) {
    if ctx.spell.id == SHRAPNEL && equipped(world, ctx.caster, FOUR) {
        auras::apply_aura(world, &AuraApplication {
            caster: ctx.caster, target, spell_id: SCORCHED,
            effect_mask: u32::MAX, source: AuraSource::Cast,
        });
    }
}
