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
const BIND_BUFF: u32 = 443532;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetContextDamageMultiplier(
    rime_primary_multiplier,
)];

fn rime_primary_multiplier(world: &World, ctx: &EffectCtx, target: Entity, _: bool) -> f32 {
    let earned_rime = ctx
        .spell
        .cast_charge_mods
        .iter()
        .flatten()
        .any(|modifier| modifier.aura_spell_id == RIME);
    if ctx.spell.id != HOWLING_BLAST || ctx.primary_target() != Some(target) || !earned_rime {
        return 1.0;
    }
    // SimC76f8118 sc_death_knight.cpp11545–11555 applies each selected
    // talent's native effect separately to the earned Rime primary.
    let icebreaker = if auras::has_aura(world, ctx.caster, ICEBREAKER) {
        native_multiplier(world, ICEBREAKER, 0)
    } else {
        1.0
    };
    let darkness = if auras::has_aura(world, ctx.caster, BIND_BUFF) {
        native_multiplier(world, BIND_IN_DARKNESS, 3)
    } else {
        1.0
    };
    icebreaker * darkness
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
