//! SimC76f8118 sc_priest_shadow.cpp46–58/2745–2765: earned three-DoT window.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance};
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

const TALENT: u32 = 373212;
const DOTS: &[u32] = &[589, 34914, 335467];
const CHANNELS: &[u32] = &[15407, 391403, 263165, 1242189];

#[derive(Component)]
struct IreWindow(f64);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterAuraApplication(refresh_window),
    SpellScriptHook::TargetDamageMultiplier(channel_multiplier),
];

fn refresh_window(world: &mut World, target: Entity, aura: &AuraInstance) {
    let caster = aura.caster;
    if !DOTS.contains(&aura.spell_id) || !auras::has_aura(world, caster, TALENT) {
        return;
    }
    let now = spell_cast::now(world);
    let Some(auras) = world.get::<ActiveAuras>(target) else {
        return;
    };
    let expiry = DOTS
        .iter()
        .map(|id| {
            auras
                .iter()
                .find(|owned| owned.caster == caster && owned.spell_id == *id)
                .and_then(|owned| owned.expires_at)
                .filter(|expiry| *expiry > now)
        })
        .collect::<Option<Vec<_>>>();
    let Some(expiry) = expiry.and_then(|expiries| expiries.into_iter().min_by(f64::total_cmp))
    else {
        return;
    };
    if world
        .get::<IreWindow>(caster)
        .is_none_or(|window| expiry >= window.0)
    {
        world.entity_mut(caster).insert(IreWindow(expiry));
    }
}

fn channel_multiplier(world: &World, caster: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    let live = world
        .get::<IreWindow>(caster)
        .is_some_and(|window| window.0 > spell_cast::now(world));
    if !live || !CHANNELS.contains(&spell.id) {
        return 1.0;
    }
    1.0 + spell_cast::scripts::script_effect_value(world, caster, TALENT, 0) / 100.0
}
