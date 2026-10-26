//! Native Darkglare Presence reads the real guardian lifetime and authored damage mask.
use super::combat;
use bevy::prelude::*;
use combat::auras::spell_mods;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{
    effects::EffectCtx, script_registry::SpellScriptHook, summon::SummonedCreature,
};
use combat::spell_info::SpellStore;
use shared::components::{Health, Npc};

const PRESENCE: u32 = 1280663;
const DARKGLARE: u32 = 103673;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetContextDamageMultiplier(
    damage_multiplier,
)];

fn has_living_darkglare(world: &World, owner: Entity) -> bool {
    let now = combat::auras::aura_now(world);
    let Some(mut guardians) = world.try_query::<(&SummonedCreature, &Npc, &Health)>() else {
        return false;
    };
    guardians.iter(world).any(|(summon, npc, health)| {
        summon.owner == owner
            && npc.template_id == DARKGLARE
            && health.current > 0.0
            && summon.expires_at.is_none_or(|expiry| expiry > now)
    })
}

// SimC76f8118 sc_warlock_pets.cpp2487–2500: actual arise/demise owns
// Presence; sc_warlock_actions.cpp200 parses its native direct/periodic masks.
// Read the existing guardian instead of creating a second duration clock.
fn damage_multiplier(world: &World, ctx: &EffectCtx<'_>, _: Entity, periodic: bool) -> f32 {
    let store = world.resource::<SpellStore>();
    let Some(presence) = store.get(PRESENCE) else {
        return 1.0;
    };
    let index = u8::from(periodic);
    let Some(effect) = presence.effects.iter().find(|effect| effect.index == index) else {
        return 1.0;
    };
    let family = presence
        .class_options
        .as_ref()
        .map_or(0, |options| options.spell_class_set);
    if !spell_mods::class_mask_matches(family, &effect.spell_class_mask, ctx.spell)
        || !has_living_darkglare(world, ctx.caster)
    {
        return 1.0;
    }
    1.0 + script_effect_value(world, ctx.caster, PRESENCE, index) / 100.0
}
