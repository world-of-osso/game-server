//! 70448 Bursting Growth 440120, Restoration side: each expiring Symbiotic Blooms
//! 439530 stack, and each Rejuvenation landing on a target with the caster's
//! blooms, heals that target and the nearest allies with 440121 (s2 total).
//! SimC76f8118 implements only the Feral Vines side. WCL kD8MqXLV6CFYHN2p fight3
//! actor108: 659/711 bloom stack losses and 198/206 Rejuvenation casts on bloomed
//! targets burst there; Rejuvenation removes no stack (4/206); bursts heal ≤3.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraInstance, scripts::AuraScripts};
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};

const TALENT: u32 = 440_120;
const BLOOM: u32 = 439_530;
const BURST: u32 = 440_121;
const REJUVENATION: u32 = 774;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::MidAuraRegistration(register),
    SpellScriptHook::AfterResolvedHits(burst_on_rejuvenation),
];

fn register(scripts: &mut AuraScripts) {
    // The async scheduler reports every expired application, including the last.
    scripts.0.entry(BLOOM).or_default().on_stack_loss = Some(burst_expired_stacks);
}

fn burst_expired_stacks(world: &mut World, target: Entity, aura: &AuraInstance, lost: u8) {
    if !combat::auras::has_aura(world, aura.caster, TALENT) {
        return;
    }
    for _ in 0..lost {
        burst(world, aura.caster, target, 1);
    }
}

fn burst_on_rejuvenation(world: &mut World, ctx: &EffectCtx<'_>) {
    if ctx.spell.id != REJUVENATION || !combat::auras::has_aura(world, ctx.caster, TALENT) {
        return;
    }
    let Some(targets) = spell_cast::resolving_targets(world, ctx) else {
        return;
    };
    let bloomed: Vec<Entity> = targets
        .iter()
        .copied()
        .filter(|&target| owns_bloom(world, ctx.caster, target))
        .collect();
    for target in bloomed {
        burst(world, ctx.caster, target, ctx.depth + 1);
    }
}

fn owns_bloom(world: &World, caster: Entity, target: Entity) -> bool {
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == BLOOM && aura.caster == caster)
    })
}

/// 440121 s2 caps the area; native area selection is nearest-first, so the
/// bloomed target at the center is always healed.
fn burst(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let Some(mut spell) = spell_cast::load_triggered_spell(world, BURST, depth) else {
        return;
    };
    let cap = spell
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("native Bursting Growth retains its target count")
        .base_points as u32;
    spell
        .target_restrictions
        .get_or_insert_with(Default::default)
        .max_targets = cap;
    spell_cast::launch_triggered_spell(world, caster, &spell, Some(target), depth);
}
