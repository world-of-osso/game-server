//! 70448 Celestial Conduit (Mistweaver): each 443028 pulse heals injured allies (443039)
//! and damages nearby enemies (443038). 443028's periodic effect names no trigger spell.
//! Windwalker's pulse is not implemented here.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::{AreaTargetFilter, ContextHitHook, SpellScriptHook},
    scripts::script_effect_value,
    summon::SummonedCreature,
};
use combat::spell_info::SpellInfo;
use shared::components::{ActiveSpec, Health, UnitIsPet};

const CONDUIT: u32 = 443028;
const HEAL: u32 = 443039;
const DAMAGE: u32 = 443038;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ChannelPeriodic(periodic),
    SpellScriptHook::AreaFilter(area_filter),
];

fn periodic(spell: u32, effect: u8) -> Option<ContextHitHook> {
    (spell == CONDUIT && effect == 0).then_some(pulse)
}

fn pulse(world: &mut World, ctx: &EffectCtx<'_>, _: Entity) {
    let caster = ctx.caster;
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 270)
    {
        return;
    }
    for spell in [HEAL, DAMAGE] {
        if !spell_cast::try_cast_triggered(world, caster, spell, Some(caster), ctx.depth + 1) {
            warn!(?caster, spell, "Celestial Conduit pulse failed");
        }
    }
}

fn area_filter(spell: u32, effect: u8) -> Option<AreaTargetFilter> {
    (spell == HEAL && effect == 0).then_some(injured_allies)
}

/// "up to 443028 s3 injured allies": party members below full health, no summons or pets.
fn injured_allies(world: &World, _: &SpellInfo, caster: Option<Entity>, units: &mut Vec<Entity>) {
    let Some(caster) = caster else {
        units.clear();
        return;
    };
    units.retain(|&unit| {
        world.get::<SummonedCreature>(unit).is_none()
            && !world.get::<UnitIsPet>(unit).is_some_and(|pet| pet.0)
            && world
                .get::<Health>(unit)
                .is_some_and(|health| health.current < health.max)
    });
    units.truncate(script_effect_value(world, caster, CONDUIT, 2).max(0.0) as usize);
}
