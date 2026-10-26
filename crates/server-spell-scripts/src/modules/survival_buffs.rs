//! Earned Survival buffs retain native effects, caps and aura lifetimes.
use super::cast_relationships::has_talent;
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SuccessfulCastRelationships(grant_mongoose_fury),
    SpellScriptHook::AsynchronousAuras(&[STARGAZER_BUFF]),
];

const RAPTOR_STRIKE: u32 = 186270;
const RAPTOR_SWIPE: u32 = 1262293;
const MONGOOSE_FURY: u32 = 259388;
const STARGAZER: u32 = 1253751;
const STARGAZER_BUFF: u32 = 1253750;

fn grant_mongoose_fury(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // SimC76f8118 sc_hunter.cpp6149–6155/7749/8160–8165: every
    // actual base execution triggers the buff, even without the named talent.
    if matches!(spell.id, RAPTOR_STRIKE | RAPTOR_SWIPE) {
        grant_self_buff(world, caster, MONGOOSE_FURY);
    }
}

pub(super) fn after_tip_impact(world: &mut World, caster: Entity) {
    // SimC76f8118 sc_hunter.cpp1358–1361/7880/8303–8305: selected
    // Stargazer triggers after real Tip consumption, with independent stack expiry.
    if has_talent(world, caster, STARGAZER) {
        grant_self_buff(world, caster, STARGAZER_BUFF);
    }
}

fn grant_self_buff(world: &mut World, caster: Entity, spell: u32) {
    // Full native applications retain all effects, including Mongoose's label
    // modifiers; effect0-only priming would omit selected native payloads.
    if !spell_cast::try_cast_triggered(world, caster, spell, Some(caster), 1) {
        warn!(?caster, spell, "Earned native Survival buff refused");
    }
}
