//! SimC76f8118 sc_druid.cpp5954,6002–6007,8601–8614.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::random::Rng;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;

const TALENT: u32 = 1271206;
const CHILD: u32 = 197626;
pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResolvedHits(after_thrash)];

#[derive(Component)]
struct CascadeReadyAt(f64);

fn after_thrash(world: &mut World, ctx: &EffectCtx<'_>) {
    if ctx.spell.id != 77758 || ctx.depth != 0 {
        return;
    }
    let Some(talent) = world.get::<ActiveAuras>(ctx.caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == TALENT && aura.caster == ctx.caster)
    }) else {
        return;
    };
    let effect = |index| {
        talent
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("Star Cascade native effect")
            .amount
    };
    let chance = effect(1);
    let multiplier = effect(3) / 100.0;
    let now = spell_cast::now(world);
    if world
        .get::<CascadeReadyAt>(ctx.caster)
        .is_some_and(|ready| now < ready.0)
    {
        return;
    }
    let Some(targets) = spell_cast::resolving_damage(world, ctx) else {
        return;
    };
    if targets.is_empty() || combat::random::random::<f32>() >= chance / 100.0 {
        return;
    }
    let target = targets[combat::random::thread_rng().gen_range(0..targets.len())].0;
    let recovery = world
        .resource::<SpellStore>()
        .get(TALENT)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("Star Cascade recovery")
        .proc_category_recovery_ms as f64
        / 1000.0;
    world
        .entity_mut(ctx.caster)
        .insert(CascadeReadyAt(now + recovery));
    spell_cast::cast_triggered_with_overrides(
        world,
        ctx.caster,
        CHILD,
        Some(target),
        ctx.depth + 1,
        spell_cast::TriggeredSpellOverrides {
            damage_multiplier: Some(multiplier),
            ..Default::default()
        },
    );
}
