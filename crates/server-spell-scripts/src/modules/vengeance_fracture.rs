//! Fracture creates fragments on its two native weapon impacts.
use super::combat;
use bevy::prelude::*;
use combat::soul_fragments::{self, FragmentKind};
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};
use shared::components::ActiveSpec;

#[derive(Component, Default)]
struct PendingMetaFragments(Vec<f64>);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResolvedDamage(on_impact),
    SpellScriptHook::AfterCastUpdate(spawn_meta_fragments),
];

fn on_impact(world: &mut World, ctx: &EffectCtx, _: Entity, _: bool, damage: f32) {
    if damage <= 0.0
        || !matches!(ctx.spell.id, 225919 | 225921)
        || world
            .get::<ActiveSpec>(ctx.caster)
            .is_none_or(|spec| spec.0 != 581)
    {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp8014–8046 assigns half of the
    // native Fracture effect1 count to each hand; the main hand gets the remainder.
    if ctx.spell.id == 225919 && combat::auras::has_aura(world, ctx.caster, 187827) {
        // sc_demon_hunter.cpp8103–8108: the extra Meta fragment arrives 500ms after the main hit.
        let due = combat::auras::aura_now(world) + 0.5;
        world.entity_mut(ctx.caster).entry::<PendingMetaFragments>().or_default().into_mut().0.push(due);
    }
    let count = world.resource::<combat::spell_info::SpellStore>().get(263642)
        .and_then(|spell| spell.effects.iter().find(|effect| effect.index == 0))
        .expect("native Fracture fragment effect").base_points;
    if !count.is_finite() || count < 0.0 || count.fract() != 0.0 {
        error!(count, "invalid native Fracture fragment count");
        return;
    }
    let count = count as u32;
    let count = count / 2 + u32::from(ctx.spell.id == 225919) * (count % 2);
    for _ in 0..count {
        if let Err(reason) = soul_fragments::spawn(world, ctx.caster, FragmentKind::Lesser, false) {
            error!(%reason, "Fracture fragment spawn failed");
        }
    }
}

fn spawn_meta_fragments(world: &mut World) {
    let now = combat::auras::aura_now(world);
    let mut due = Vec::new();
    for (caster, mut pending) in world.query::<(Entity, &mut PendingMetaFragments)>().iter_mut(world) {
        pending.0.retain(|deadline| {
            if *deadline <= now { due.push(caster); false } else { true }
        });
    }
    for caster in due {
        if let Err(reason) = soul_fragments::spawn(world, caster, FragmentKind::Lesser, false) {
            error!(%reason, "Metamorphosis Fracture fragment spawn failed");
        }
    }
}
