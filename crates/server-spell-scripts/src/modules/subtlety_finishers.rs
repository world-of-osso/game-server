//! SimC76f8118 sc_rogue.cpp4400–4449/5730–5806: Shadowed Finishers
//! dispatch native Shadow payloads while the caster's Find Weakness is active.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::spell_cast::{
    self, effects::EffectCtx, script_registry::SpellScriptHook, scripts::script_effect_value,
    targets,
};
use combat::spell_info::SpellStore;
use shared::components::{ActiveSpec, PowerType};

const FIND_WEAKNESS: u32 = 91023;
const WEAKNESS_BUFF: u32 = 1264521;
const SHADOWED_FINISHERS: u32 = 382511;
const FINISHERS: &[(u32, u32)] = &[(196819, 328082), (319175, 319190)];

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResourceContext(&[196819, 319175]),
    SpellScriptHook::AfterPolicyDirectDamage(after_damage),
];

fn is_subtlety(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 261)
}

fn after_damage(world: &mut World, ctx: &EffectCtx, target: Entity, critical: bool) {
    if ctx.depth != 0 || !is_subtlety(world, ctx.caster) {
        return;
    }
    grant_find_weakness(world, ctx, target, critical);
    if ctx.spell.cast_primary_target == Some(target) {
        dispatch_shadow_finisher(world, ctx, target);
    }
}

fn grant_find_weakness(world: &mut World, ctx: &EffectCtx, target: Entity, critical: bool) {
    // sc_rogue.cpp5652/3735–3738/5899–5902/8512–8528: Shadowstrike
    // grants the full native buff; qualifying builder crits grant timed windows.
    let duration_ms = match ctx.spell.id {
        185438 if has_talent(world, ctx.caster, FIND_WEAKNESS) => None,
        53 if critical && behind_target(world, ctx.caster, target) => {
            timed_weakness(world, ctx.caster, 319949)
        }
        197835 if critical => timed_weakness(world, ctx.caster, 319951),
        _ => return,
    };
    if ctx.spell.id != 185438 && duration_ms.is_none() {
        return;
    }
    spell_cast::cast_triggered_with_overrides(
        world,
        ctx.caster,
        WEAKNESS_BUFF,
        Some(ctx.caster),
        ctx.depth + 1,
        spell_cast::TriggeredSpellOverrides {
            duration_ms,
            ..Default::default()
        },
    );
}

fn timed_weakness(world: &World, caster: Entity, talent: u32) -> Option<i32> {
    if !has_talent(world, caster, talent) {
        return None;
    }
    let native_ms = script_effect_value(world, caster, talent, 0) * 1000.0;
    let now = spell_cast::now(world);
    let remaining_ms = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == WEAKNESS_BUFF && aura.caster == caster)
        })
        .and_then(|aura| aura.expires_at)
        .map_or(0.0, |expiry| ((expiry - now) * 1000.0) as f32);
    Some(native_ms.max(remaining_ms) as i32)
}

fn behind_target(world: &World, caster: Entity, target: Entity) -> bool {
    let from = targets::position(world, target).expect("native Find Weakness target position");
    let to = targets::position(world, caster).expect("native Find Weakness caster position");
    let angle = targets::orientation(world, target);
    Vec3::new(angle.cos(), 0.0, -angle.sin()).dot(to - from) < 0.0
}

fn dispatch_shadow_finisher(world: &mut World, ctx: &EffectCtx, target: Entity) {
    let Some(&(_, payload)) = FINISHERS
        .iter()
        .find(|&&(parent, _)| parent == ctx.spell.id)
    else {
        return;
    };
    if !has_talent(world, ctx.caster, SHADOWED_FINISHERS)
        || !auras::has_aura(world, ctx.caster, WEAKNESS_BUFF)
    {
        return;
    }
    let points = ctx
        .spell
        .cast_policy_context
        .as_ref()
        .expect("native finisher retains resource context")
        .resources
        .iter()
        .find(|&&(power, _)| power == PowerType::ComboPoints)
        .expect("native finisher retains effective points")
        .1;
    let fraction = script_effect_value(world, ctx.caster, SHADOWED_FINISHERS, 0) / 100.0;
    let mut child = world
        .resource::<SpellStore>()
        .get(payload)
        .expect("native Shadowed Finishers payload")
        .clone();
    child.cast_damage_multiplier = Some(points * fraction);
    spell_cast::launch_triggered_spell(world, ctx.caster, &child, Some(target), ctx.depth + 1);
}
