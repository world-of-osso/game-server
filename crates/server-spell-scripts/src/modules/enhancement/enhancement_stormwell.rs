//! SimC76f8118 sc_shaman.cpp6141–6145: selected Stormwell owns one primary-impact callback.
use super::combat;
use bevy::prelude::*;

use combat::auras::{self, ActiveAuras, procs};
use combat::class_progression::KnownSpells;
use combat::spell_cast::effects::EffectCtx;

const CRASH: u32 = 187874;
const STORMWELL: u32 = 1264762;
const WINDFURY: u32 = 319773;
const GHOST_WOLF: u32 = 2645;

pub(super) fn after_primary_crash(world: &mut World, ctx: &EffectCtx, target: Entity) {
    if ctx.spell.id != CRASH || auras::has_aura(world, ctx.caster, GHOST_WOLF) {
        return;
    }
    let selected = world
        .get::<KnownSpells>(ctx.caster)
        .is_some_and(|known| known.contains(STORMWELL));
    if !selected {
        return;
    }
    let instance = world.get::<ActiveAuras>(ctx.caster).and_then(|active| {
        active
            .iter()
            .find(|aura| aura.spell_id == WINDFURY && aura.caster == ctx.caster)
            .map(|aura| aura.instance_id)
    });
    let Some(instance) = instance else { return };
    // The source's explicit1.0 callback does not roll the ordinary proc chance.
    // Reuse the equipped native aura's ICD and its ordinary payload/Unruly Winds.
    if let Some(aura) = procs::start_policy_proc(world, ctx.caster, instance) {
        super::enhancement_imbues::cast_windfury(world, ctx.caster, &aura, Some(target));
    }
}
