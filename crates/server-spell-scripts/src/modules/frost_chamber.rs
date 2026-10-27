//! Cryogenic Chamber stores earned-Rime impacts until a real Winter starts.
use bevy::prelude::*;

use super::combat;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{self, TriggeredSpellOverrides};

const TALENT: u32 = 456237;
const CHAMBER: u32 = 456370;
const EXPLOSION: u32 = 456371;
const RIME: u32 = 59052;
const HOWLING_BLAST: u32 = 49184;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResolvedDamage(store_rime_hit),
    SpellScriptHook::OnCast(release_on_winter),
];

#[derive(Component, Default)]
struct ChamberDamage(f32);

fn store_rime_hit(world: &mut World, ctx: &EffectCtx, _: Entity, _: bool, damage: f32) {
    let earned_rime = ctx
        .spell
        .cast_charge_mods
        .iter()
        .flatten()
        .any(|modifier| modifier.aura_spell_id == RIME);
    if ctx.spell.id != HOWLING_BLAST || !earned_rime || !auras::has_aura(world, ctx.caster, TALENT)
    {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp11590–11606: each Rime impact
    // banks its landed damage until the native talent stack cap is reached.
    let stacks = world.get::<ActiveAuras>(ctx.caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == CHAMBER && aura.caster == ctx.caster)
            .map(|aura| aura.stacks)
    });
    if stacks.is_none() {
        world
            .entity_mut(ctx.caster)
            .insert(ChamberDamage::default());
    }
    let cap = script_effect_value(world, ctx.caster, TALENT, 1) as u8;
    if stacks.unwrap_or(0) < cap {
        let fraction = script_effect_value(world, ctx.caster, TALENT, 0) / 100.0;
        world
            .entity_mut(ctx.caster)
            .entry::<ChamberDamage>()
            .or_default()
            .into_mut()
            .0 += damage * fraction;
    }
    assert!(
        spell_cast::try_cast_triggered(world, ctx.caster, CHAMBER, Some(ctx.caster), 1),
        "native Cryogenic Chamber buff refused"
    );
}

fn release_on_winter(world: &mut World, caster: Entity, spell: u32, _: u8) {
    if !matches!(spell, 196770 | 1233152) || !auras::has_aura(world, caster, CHAMBER) {
        return;
    }
    // sc_death_knight.cpp7215–7245/12445–12455: Winter expires an active
    // chamber early; natural expiration discards its stored damage instead.
    let amount = world
        .entity_mut(caster)
        .take::<ChamberDamage>()
        .expect("active chamber has stored damage")
        .0;
    auras::remove_auras(world, caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == CHAMBER && aura.caster == caster
    });
    assert!(
        spell_cast::cast_triggered_with_overrides(
            world,
            caster,
            EXPLOSION,
            Some(caster),
            1,
            TriggeredSpellOverrides {
                effect0_amount: Some(amount),
                ..Default::default()
            }
        ),
        "native Cryogenic Chamber explosion refused"
    );
}
