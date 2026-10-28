//! Selected talents amplify an earned Rime Howling Blast primary hit.
use bevy::prelude::*;

use super::combat;
use combat::auras;
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellStore;

const HOWLING_BLAST: u32 = 49184;
const RIME: u32 = 59052;
const ICEBREAKER: u32 = 392950;
const BIND_IN_DARKNESS: u32 = 440031;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetContextDamageMultiplier(
    rime_primary_multiplier,
)];

fn rime_primary_multiplier(world: &World, ctx: &EffectCtx, target: Entity, _: bool) -> f32 {
    if ctx.spell.id != HOWLING_BLAST {
        return 1.0;
    }
    let targets = combat::spell_cast::resolving_targets(world, ctx).expect("Howling Blast targets");
    let index = targets
        .iter()
        .position(|candidate| *candidate == target)
        .expect("Howling Blast target ordinal");
    let northwinds = auras::has_aura(world, ctx.caster, 1230284) && index == 1;
    let full_targets = if auras::has_aura(world, ctx.caster, 1230284) {
        2
    } else {
        1
    };
    let mut multiplier = if index >= full_targets && targets.len() > 1 {
        (1.0 / targets.len().min(20) as f32).sqrt()
    } else {
        1.0
    };
    let earned_rime = ctx
        .spell
        .cast_charge_mods
        .iter()
        .flatten()
        .any(|modifier| modifier.aura_spell_id == RIME);
    if index > 0 && !northwinds {
        if earned_rime && auras::has_aura(world, ctx.caster, 376938) {
            multiplier *= native_multiplier(world, 376938, 1);
        }
        multiplier /= native_multiplier(world, HOWLING_BLAST, 1);
    }
    if !earned_rime || (index > 0 && !northwinds) {
        return multiplier;
    }
    // SimC76f8118 sc_death_knight.cpp11545–11555 applies each selected
    // talent's native effect separately to the earned Rime primary.
    let icebreaker = if auras::has_aura(world, ctx.caster, ICEBREAKER) {
        native_multiplier(world, ICEBREAKER, 0)
    } else {
        1.0
    };
    let darkness = if auras::has_aura(world, ctx.caster, BIND_IN_DARKNESS) {
        native_multiplier(world, BIND_IN_DARKNESS, 3)
    } else {
        1.0
    };
    multiplier * icebreaker * darkness
}

fn native_multiplier(world: &World, spell: u32, index: u8) -> f32 {
    let percent = world
        .resource::<SpellStore>()
        .get(spell)
        .and_then(|spell| spell.effects.iter().find(|effect| effect.index == index))
        .expect("native Rime talent effect")
        .base_points;
    1.0 + percent / 100.0
}
