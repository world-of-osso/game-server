//! Wildspeaker commands existing native DireCritter summons, before Howl consumption.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook, summon::SummonedCreature};
use combat::spell_info::SpellInfo;
use shared::components::Health;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterElementalLaunch(command_existing)];

const WILDSPEAKER: u32 = 1232739;
const KILL_COMMAND: u32 = 34026;
const COMMAND_DAMAGE: u32 = 1232922;
// SimC76f8118 sc_hunter.cpp1900/1928/1961/2003/2027/3398–3402:
// Dark Hound, Dire Beast, Fenryr, Hati and Bear are DireCritters. Wyvern
// and the ordinary stable bodies are not members of this source class.
const DIRE_CRITTER_SUMMONS: &[u32] = &[219199, 442419, 459733, 459739, 471990];

fn command_existing(world: &mut World, owner: Entity, spell: &SpellInfo, foreground: bool) {
    if !foreground || spell.id != KILL_COMMAND || !has_talent(world, owner, WILDSPEAKER) {
        return;
    }
    let Some(primary) = spell.cast_primary_target else { return };
    let pets: Vec<_> = world.query::<(Entity, &SummonedCreature, &Health)>()
        .iter(world)
        .filter(|(_, summon, health)| summon.owner == owner
            && DIRE_CRITTER_SUMMONS.contains(&summon.spell_id) && health.current > 0.0)
        .map(|(pet, _, _)| pet).collect();
    // SimC76f8118 sc_hunter.cpp6590–6604: snapshot and command existing
    // DireCritters before consuming Howl. The2530 tie retains declaration
    // order: hunter_command_wildspeaker runs before hunter_howl.
    for pet in pets {
        if !spell_cast::try_cast_triggered(world, pet, COMMAND_DAMAGE, Some(primary), 1) {
            warn!(?owner, ?pet, ?primary, "Native Wildspeaker Kill Command refused");
        }
    }
}
