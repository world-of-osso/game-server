//! SimC76f8118 sc_rogue.cpp5131–5135,6278–6287,7993–7999:
//! each landed Mutilate hand contributes its native fraction to the rolling bleed.
use super::{combat, marksmanship::bank_residual};
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(after_hand)];
const TALENT: u32 = 381673;
const BLEED: u32 = 394021;
const HANDS: &[u32] = &[5374, 27576];

fn after_hand(world: &mut World, ctx: &EffectCtx, target: Entity, _crit: bool, damage: f32) {
    if damage <= 0.0 || !HANDS.contains(&ctx.spell.id) {
        return;
    }
    let percent = world.get::<ActiveAuras>(ctx.caster).and_then(|auras| {
        let talent = auras
            .iter()
            .find(|aura| aura.spell_id == TALENT && aura.caster == ctx.caster)?;
        Some(
            talent
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .expect("native Doomblade retains its residual fraction")
                .amount,
        )
    });
    let Some(percent) = percent else { return };
    // Native394021 owns duration, cadence, rolling carry and no-second-crit flags.
    // player.cpp957–964 snapshots DMG_OVER_TIME: native Implacable periodic
    // modifiers apply; Doomblade's unused composite_da override does not.
    bank_residual(world, ctx, target, BLEED, percent, damage);
}
