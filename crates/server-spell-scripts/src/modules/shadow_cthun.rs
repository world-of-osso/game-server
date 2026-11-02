//! SimC76f8118 sc_priest.cpp4220–4230/4263–4282 and sc_priest_pets.cpp903–1056.
use super::{combat, shadow_apparitions, shadow_priest};
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, procs};
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::{ScriptReach, SpellScriptHook},
    summon::SummonedCreature,
};
use combat::spell_info::SpellInfo;
use shared::components::Npc;

const TALENT: u32 = 377349;
const APEX: u32 = 1264096;
const TENDRIL_SUMMON: u32 = 377355;
const LASHER_SUMMON: u32 = 377357;
const TENDRIL: u32 = 192337;
const LASHER: u32 = 198757;
const MIND_FLAY: u32 = 193473;
const MIND_SEAR: u32 = 394979;
const INSANITY: u32 = 377358;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(owns_proc),
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::ResolvedDamage(on_damage),
    SpellScriptHook::DotMasteryMultiplier(pet_mastery),
    SpellScriptHook::TargetContextDamageMultiplier(lasher_scale),
    SpellScriptHook::ConsumedEffectValues(&[(MIND_SEAR, 2)]),
];

fn owns_proc(spell: u32) -> bool {
    spell == TALENT
}

fn declared_reach(_: &World, _: Entity, spell: &SpellInfo) -> ScriptReach {
    if matches!(spell.id, 15407 | 391403 | 263165) {
        ScriptReach::EngagedOnly
    } else {
        ScriptReach::None
    }
}

fn on_damage(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool, _: f32) {
    if matches!(ctx.spell.id, 15407 | 391403 | 263165) && ctx.spell.cast_periodic_ordinal.is_some()
    {
        try_summon(world, ctx, target);
    }
    if let Some(owner) = pet_owner(world, ctx.caster, ctx.spell.id) {
        let primary = ctx.spell.id == MIND_FLAY || ctx.primary_target() == Some(target);
        if primary {
            spell_cast::cast_triggered(world, owner, INSANITY, Some(owner), ctx.depth + 1);
        }
    }
}

fn try_summon(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let instance = world.get::<ActiveAuras>(ctx.caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.caster == ctx.caster && aura.spell_id == TALENT)
            .map(|aura| aura.instance_id)
    });
    let Some(instance) = instance else { return };
    if !procs::try_native_policy_proc(world, ctx.caster, instance) {
        return;
    }
    // SimC action.cpp1738–1773 and sc_priest.cpp4268–4276 use its live enemy list.
    let count = combat::combat_refs::active_opponents(world, ctx.caster)
        .filter(|&unit| spell_cast::targets::is_alive(world, unit))
        .count();
    let id = if count > 2 {
        LASHER_SUMMON
    } else {
        TENDRIL_SUMMON
    };
    let mut summon = spell_cast::load_triggered_spell(world, id, ctx.depth + 1)
        .expect("native C'Thun guardian summon");
    let position =
        spell_cast::targets::position(world, ctx.caster).expect("C'Thun caster position");
    summon.cast_destination = Some(position.to_array());
    if !spell_cast::launch_triggered_spell(
        world,
        ctx.caster,
        &summon,
        Some(ctx.caster),
        ctx.depth + 1,
    ) {
        warn!(caster = ?ctx.caster, spell = id, "C'Thun guardian summon rejected");
        return;
    }
    if auras::has_aura(world, ctx.caster, APEX) {
        shadow_apparitions::spawn_apparitions(world, ctx, target);
    }
}

fn pet_owner(world: &World, actor: Entity, spell: u32) -> Option<Entity> {
    let summon = world.get::<SummonedCreature>(actor)?;
    let npc = world.get::<Npc>(actor)?;
    matches!(
        (summon.spell_id, npc.template_id, spell),
        (TENDRIL_SUMMON, TENDRIL, MIND_FLAY) | (LASHER_SUMMON, LASHER, MIND_SEAR)
    )
    .then_some(summon.owner)
}

fn pet_mastery(world: &World, caster: Entity, target: Entity, spell: u32) -> f32 {
    let Some(owner) = pet_owner(world, caster, spell) else {
        return 1.0;
    };
    shadow_priest::weaving_multiplier(world, owner, target, spell)
}

fn lasher_scale(world: &World, ctx: &EffectCtx<'_>, _: Entity, _: bool) -> f32 {
    if pet_owner(world, ctx.caster, ctx.spell.id).is_none() || ctx.spell.id != MIND_SEAR {
        return 1.0;
    }
    let count = spell_cast::resolving_targets(world, ctx)
        .expect("native Mind Sear targets")
        .len() as f32;
    let cap = spell_cast::scripts::script_effect_value(world, ctx.caster, MIND_SEAR, 2);
    (cap / count).min(1.0).sqrt()
}
