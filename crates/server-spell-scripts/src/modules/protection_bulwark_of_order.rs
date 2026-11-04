//! DB2 70448 Bulwark of Order converts Avenger's Shield damage into an owned absorb.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};

const TALENT: u32 = 209389;
const SHIELD: u32 = 209388;
const AVENGERS_SHIELD: u32 = 31935;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(shield_impact)];

fn shield_impact(world: &mut World, ctx: &EffectCtx<'_>, _: Entity, _: bool, damage: f32) {
    if ctx.spell.id != AVENGERS_SHIELD
        || damage <= 0.0
        || !combat::auras::has_aura(world, ctx.caster, TALENT)
    {
        return;
    }
    let percent = spell_cast::scripts::script_effect_value(world, ctx.caster, TALENT, 0);
    let mut shield = spell_cast::load_triggered_spell(world, SHIELD, ctx.depth + 1)
        .expect("selected Bulwark of Order requires its native shield");
    shield
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("native Bulwark of Order absorb")
        .base_points = damage * percent / 100.0;
    // The source damage already contains outgoing modifiers, as for Ursoc's Fury.
    shield
        .misc
        .as_mut()
        .expect("native Bulwark of Order metadata")
        .attributes[11] |= 0x0008_0000;
    assert!(
        spell_cast::launch_triggered_spell(
            world,
            ctx.caster,
            &shield,
            Some(ctx.caster),
            ctx.depth + 1
        ),
        "Bulwark of Order shield refused for {:?}",
        ctx.caster
    );
}
