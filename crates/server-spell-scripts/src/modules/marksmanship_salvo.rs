//! SimC76f8118 sc_hunter.cpp6940–6965: Volley earns capped background explosives.
use super::combat;
use super::marksmanship::selected;
use bevy::prelude::*;
use combat::spell_cast::script_registry::{ContextHitHook, SpellScriptHook};
use combat::spell_cast::{self, effects::EffectCtx, scripts::script_effect_value};
use combat::spell_info::SpellStore;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ContextHit(hit_hook)];

const SALVO: u32 = 400456;
const VOLLEY: u32 = 260243;
const VOLLEY_DAMAGE: u32 = 260247;
const EXPLOSIVE_SHOT: u32 = 212431;

#[derive(Component)]
struct SalvoCooldown(f64);

fn hit_hook(spell: u32) -> Option<ContextHitHook> {
    (spell == VOLLEY_DAMAGE).then_some(on_volley_hit)
}

fn on_volley_hit(world: &mut World, ctx: &EffectCtx<'_>, _target: Entity) {
    if !selected(world, ctx.caster, SALVO) {
        return;
    }
    let now = spell_cast::now(world);
    if world
        .get::<SalvoCooldown>(ctx.caster)
        .is_some_and(|cooldown| cooldown.0 > now)
    {
        return;
    }
    let cap = script_effect_value(world, ctx.caster, SALVO, 0) as usize;
    let targets: Vec<_> = spell_cast::resolving_targets(world, ctx)
        .expect("Salvo retains the actual Volley TargetInfos")
        .iter()
        .copied()
        .take(cap)
        .collect();
    let volley = world
        .resource::<SpellStore>()
        .get(VOLLEY)
        .expect("native Volley owns Salvo's cooldown duration");
    // sc_hunter.cpp6951/7940: one pulse starts the Volley-duration ICD,
    // after admitting its first native-capped recipients, never once per hit.
    let duration = f64::from(spell_cast::duration_ms(world, ctx.caster, volley)) / 1000.0;
    world
        .entity_mut(ctx.caster)
        .insert(SalvoCooldown(now + duration));
    for target in targets {
        // Background execution keeps native periodic amounts and rolling carry,
        // without ordinary Focus payment, cooldown or Unstable Trigger producers.
        spell_cast::cast_triggered(
            world,
            ctx.caster,
            EXPLOSIVE_SHOT,
            Some(target),
            ctx.depth + 1,
        );
    }
}
