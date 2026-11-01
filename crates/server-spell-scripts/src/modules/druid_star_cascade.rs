//! SimC76f8118 sc_druid.cpp:8566–8578, 8601–8630.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::class_progression::KnownSpells;
use combat::spell_cast::{
    self, effects::EffectCtx, script_registry::SpellScriptHook, scripts::script_effect_value,
};
use combat::spell_info::SpellStore;

const TALENT: u32 = 1271206;
const CASCADE: u32 = 1271222;
const HAIL: u32 = 469004;
const SOLSTICE_TALENT: u32 = 343647;
const SOLSTICE: u32 = 343648;
const STARFIRE: u32 = 194153;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(on_starfire_hit)];

#[derive(Component)]
struct CascadeReadyAt(f64);

fn selected(world: &World, caster: Entity, spell: u32) -> bool {
    world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(spell))
}

fn on_starfire_hit(world: &mut World, ctx: &EffectCtx, target: Entity, _: bool, damage: f32) {
    if ctx.spell.id != STARFIRE
        || damage <= 0.0
        || !selected(world, ctx.caster, TALENT)
        || ctx.spell.cast_primary_target != Some(target)
    {
        return;
    }
    let now = spell_cast::now(world);
    if world
        .get::<CascadeReadyAt>(ctx.caster)
        .is_some_and(|ready| now < ready.0)
    {
        return;
    }
    let chance = script_effect_value(world, ctx.caster, TALENT, 0);
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    let cooldown = world
        .resource::<SpellStore>()
        .get(TALENT)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("Star Cascade native cooldown")
        .proc_category_recovery_ms;
    world
        .entity_mut(ctx.caster)
        .insert(CascadeReadyAt(now + f64::from(cooldown) / 1000.0));
    let multiplier = script_effect_value(world, ctx.caster, TALENT, 2) / 100.0;
    spell_cast::cast_triggered_with_overrides(
        world,
        ctx.caster,
        CASCADE,
        Some(target),
        ctx.depth + 1,
        spell_cast::TriggeredSpellOverrides {
            damage_multiplier: Some(multiplier),
            ..Default::default()
        },
    );
    if selected(world, ctx.caster, HAIL) {
        extend_solstice(world, ctx.caster);
    }
}

fn extend_solstice(world: &mut World, caster: Entity) {
    if auras::has_aura(world, caster, SOLSTICE) {
        auras::extend_owned_duration(world, caster, caster, &[SOLSTICE], 1000);
    } else {
        let amount = script_effect_value(world, caster, SOLSTICE_TALENT, 0);
        spell_cast::cast_triggered_with_overrides(
            world,
            caster,
            SOLSTICE,
            Some(caster),
            1,
            spell_cast::TriggeredSpellOverrides {
                effect0_amount: Some(amount),
                fixed_aura_duration_ms: Some(1000),
                ..Default::default()
            },
        );
    }
}
