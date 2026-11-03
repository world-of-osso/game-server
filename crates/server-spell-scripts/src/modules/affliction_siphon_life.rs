//! VERIFIED_WCL mHcDYRVyprndwjFC fight 2 pull 2: Wither ticks heal their caster.
//! DB2 70448 Siphon Life 452999 effect 1 provides the 5% healing value;
//! effect 0's class mask identifies Corruption and Wither.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, spell_mods};
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};

const SIPHON_LIFE: u32 = 452999;
const SIPHON_HEAL: u32 = 453000;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::PeriodicPrimaryDamage(heal_from_tick),
    SpellScriptHook::ConsumedEffectValues(&[(SIPHON_LIFE, 1)]),
];

fn heal_from_tick(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    _target: Entity,
    damage: f32,
    _crit: bool,
) {
    let percent = world.get::<ActiveAuras>(ctx.caster).and_then(|auras| {
        auras.iter().find_map(|aura| {
            if aura.caster != ctx.caster || aura.spell_id != SIPHON_LIFE {
                return None;
            }
            let affected = aura.effects.iter().any(|effect| {
                effect.index == 0 && spell_mods::class_mask_affects(aura, effect, ctx.spell)
            });
            affected
                .then(|| {
                    aura.effects
                        .iter()
                        .find(|effect| effect.index == 1)
                        .map(|effect| effect.amount)
                })
                .flatten()
        })
    });
    if let Some(percent) = percent.filter(|_| damage > 0.0) {
        spell_cast::cast_triggered_with_effect0_amount(
            world,
            ctx.caster,
            SIPHON_HEAL,
            Some(ctx.caster),
            ctx.depth + 1,
            damage * percent / 100.0,
        );
    }
}
