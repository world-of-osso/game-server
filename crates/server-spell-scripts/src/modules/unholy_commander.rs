//! Dark Transformation grants Commander of the Dead to eligible real guardians.
use bevy::prelude::*;
use shared::components::{ActiveSpec, Npc};

use super::combat;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraSource};
use combat::spell_cast::{script_registry::SpellScriptHook, summon::SummonedCreature};
use combat::spell_info::SpellInfo;

const TALENT: u32 = 390259;
const OWNER_BUFF: u32 = 390260;
const PET_BUFF: u32 = 390264;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SuccessfulCastRelationships(grant_commander),
    SpellScriptHook::SummonInitialized(apply_to_new_summon),
];

fn grant_commander(world: &mut World, owner: Entity, spell: &SpellInfo) {
    if spell.id != 1233448
        || world
            .get::<ActiveSpec>(owner)
            .is_none_or(|spec| spec.0 != 252)
        || !auras::has_aura(world, owner, TALENT)
    {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp8761–8762: real Dark Transformation
    // grants the native owner buff; its spell modifiers remain ordinary native effects.
    auras::apply_aura(
        world,
        &AuraApplication {
            caster: owner,
            target: owner,
            spell_id: OWNER_BUFF,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    let pets: Vec<_> = world
        .query::<(Entity, &SummonedCreature, &Npc)>()
        .iter(world)
        .filter_map(|(pet, summon, npc)| {
            (summon.owner == owner && eligible(summon.spell_id, npc.template_id)).then_some(pet)
        })
        .collect();
    for pet in pets {
        apply_pet_buff(world, owner, pet);
    }
}

fn eligible(root: u32, entry: u32) -> bool {
    // sc_death_knight.cpp3627/3853/3980/4473/5483 marks these pet families,
    // not the permanent ghoul or Horsemen, for Commander of the Dead.
    matches!(
        (root, entry),
        (275430 | 1294026, 237409)
            | (317776 | 1242294, 163366)
            | (1292072, 264321)
            | (288853, 149555)
            | (49206, 27829)
    )
}

fn apply_to_new_summon(world: &mut World, pet: Entity) {
    let Some((owner, root, entry)) = world.get::<SummonedCreature>(pet).and_then(|summon| {
        world
            .get::<Npc>(pet)
            .map(|npc| (summon.owner, summon.spell_id, npc.template_id))
    }) else {
        return;
    };
    if eligible(root, entry) && auras::has_aura(world, owner, OWNER_BUFF) {
        apply_pet_buff(world, owner, pet);
    }
}

fn apply_pet_buff(world: &mut World, owner: Entity, pet: Entity) {
    let now = auras::aura_now(world);
    let Some(expires_at) = world.get::<ActiveAuras>(owner).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == OWNER_BUFF && aura.caster == owner)
            .and_then(|aura| aura.expires_at)
    }) else {
        return;
    };
    let remaining_ms = ((expires_at - now) * 1000.0).max(0.0) as u32;
    if remaining_ms == 0 {
        return;
    }
    // Native390264 effect1 is modified by talent390259 effect2; retain its
    // native school mask and synchronize the lifetime with the owner's buff.
    auras::apply_aura(
        world,
        &AuraApplication {
            caster: owner,
            target: pet,
            spell_id: PET_BUFF,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    auras::set_owned_duration(world, owner, pet, PET_BUFF, remaining_ms);
}
