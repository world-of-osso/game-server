//! SimC76f8118 sc_monk.cpp1378–1383 and sc_monk_pets.cpp376–448.
use bevy::prelude::*;
use shared::components::Npc;

use super::combat;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{
    self, TriggeredSpellOverrides,
    script_registry::SpellScriptHook,
    scripts::{AfterCastHook, script_effect_value},
    summon::SummonedCreature,
};
use combat::spell_info::SpellInfo;

const INVOKE: u32 = 132578;
const NIUZAO: u32 = 73967;
const BLACKOUT_KICK: u32 = 205523;
const STOMP: u32 = 227291;
const WALK_WITH_OX: u32 = 387219;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterElementalLaunch(start_summon),
    SpellScriptHook::AfterResourceValues(blackout_hook),
];

fn active_niuzao(world: &mut World, owner: Entity) -> Option<Entity> {
    let now = spell_cast::now(world);
    world
        .query::<(Entity, &SummonedCreature, &Npc)>()
        .iter(world)
        .find(|(_, summon, npc)| {
            summon.owner == owner
                && summon.spell_id == INVOKE
                && npc.template_id == NIUZAO
                && summon.expires_at.is_some_and(|at| at > now)
        })
        .map(|(entity, _, _)| entity)
}

fn start_summon(world: &mut World, owner: Entity, spell: &SpellInfo, _: bool) {
    if spell.id != INVOKE {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    if let Some(pet) = active_niuzao(world, owner) {
        spell_cast::summon::start_melee_attack(world, pet, target);
    }
}

fn blackout_hook(spell: u32) -> Option<AfterCastHook> {
    (spell == BLACKOUT_KICK).then_some(stomp_after_blackout)
}

fn stomp_after_blackout(world: &mut World, owner: Entity, spell: &SpellInfo) {
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    let Some(pet) = active_niuzao(world, owner) else {
        return;
    };
    let selected = world.get::<TraitLoadout>(owner).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == WALK_WITH_OX)
    });
    let multiplier = if selected {
        1.0 + script_effect_value(world, owner, WALK_WITH_OX, 0) / 100.0
    } else {
        1.0
    };
    let values = TriggeredSpellOverrides {
        damage_multiplier: Some(multiplier),
        ..Default::default()
    };
    if !spell_cast::cast_triggered_with_overrides(world, pet, STOMP, Some(target), 1, values) {
        warn!(?owner, ?pet, ?target, "Niuzao Stomp refused");
    }
}
