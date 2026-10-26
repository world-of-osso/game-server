//! SimC76f8118 Judgment uses: cpp1436–1441/1804–1812/3057–3065/3936.
use super::combat;
use bevy::prelude::*;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::{self, ActiveAuras, AuraApplication, AuraInstance, AuraSource, spell_mods};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::{CastDamageRole, SpellStore};
use shared::components::{ActiveSpec, CombatStatus};

const MARK: u32 = 197277;
const GREATER_JUDGMENT: u32 = 231663;
const LINGERING_RADIANCE: u32 = 431407;
const DAWNLIGHT: u32 = 431380;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterPolicyDirectDamage(after_hit),
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::AsynchronousAuras(&[MARK]),
    SpellScriptHook::NonStackingIncomingAuras(&[MARK]),
    SpellScriptHook::OwnsPolicyProc(owns_mark),
];

fn owns_mark(spell: u32) -> bool {
    spell == MARK
}

fn selected(world: &World, caster: Entity, talent: u32) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 70)
        && world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == talent)
        })
}

fn after_hit(world: &mut World, ctx: &EffectCtx, target: Entity, _: bool) {
    if matches!(ctx.spell.id, 20271 | 406957 | 24275 | 1279408)
        && selected(world, ctx.caster, GREATER_JUDGMENT)
        && world
            .get::<CombatStatus>(target)
            .is_some_and(|status| status.0)
    {
        let uses = world
            .resource::<SpellStore>()
            .get(MARK)
            .expect("native Judgment debuff")
            .aura_options
            .as_ref()
            .expect("native Judgment initial uses")
            .proc_charges;
        // spell_data.hpp564: SimC initial_stacks is native ProcCharges.
        for _ in 0..uses.max(0) {
            apply_mark(world, ctx.caster, target);
        }
    }
    consume_mark(world, ctx, target);
}

fn consume_mark(world: &mut World, ctx: &EffectCtx, target: Entity) {
    // Ret450/505: Tempest and Sunrise Storm benefit but do not decrement uses.
    let secondary_storm =
        ctx.spell.id == 53385 && ctx.spell.cast_damage_role != CastDamageRole::Primary;
    if ctx.spell.id == 224239 || secondary_storm {
        return;
    }
    let instance = world.get::<ActiveAuras>(target).and_then(|auras| {
        auras
            .iter()
            .find(|aura| {
                aura.spell_id == MARK
                    && aura.caster == ctx.caster
                    && aura.effects.iter().any(|effect| {
                        effect.aura_type == 271
                            && spell_mods::class_mask_affects(aura, effect, ctx.spell)
                    })
            })
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        // hpp1237–1247: a successful affected impact consumes one owned use.
        auras::spend_stack(world, target, instance);
    }
}

fn apply_mark(world: &mut World, caster: Entity, target: Entity) {
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target,
            spell_id: MARK,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
}

fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(DAWNLIGHT).or_insert(AuraScript::default());
    script.on_natural_end = Some(after_dawnlight);
}

fn after_dawnlight(world: &mut World, target: Entity, aura: &AuraInstance) {
    if selected(world, aura.caster, LINGERING_RADIANCE) {
        apply_mark(world, aura.caster, target);
    }
}
