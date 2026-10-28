//! Outnumber counts actual lesser ghoul lifetimes on the owner.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraSource};
use combat::spell_cast::{script_registry::SpellScriptHook, summon::SummonedCreature};
use shared::components::{ActiveSpec, Npc};

const OUTNUMBER: u32 = 1241705;
const COUNTER: u32 = 1242998;

#[derive(Component)]
struct CountedGhoul;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SummonInitialized(add_ghoul),
    SpellScriptHook::SummonRemoved(remove_ghoul),
];

fn add_ghoul(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<SummonedCreature>(actor) else {
        return;
    };
    let owner = summon.owner;
    let Some(npc) = world.get::<Npc>(actor) else {
        return;
    };
    if !matches!(
        (summon.spell_id, npc.template_id),
        (275430 | 1294026, 237409) | (1277098, 258578)
    ) || world
        .get::<ActiveSpec>(owner)
        .is_none_or(|spec| spec.0 != 252)
        || !auras::has_aura(world, owner, OUTNUMBER)
    {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp3673–3689: each real lesser ghoul
    // owns one counter stack from its appearance through its dismissal.
    auras::apply_aura(
        world,
        &AuraApplication {
            caster: owner,
            target: owner,
            spell_id: COUNTER,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    if auras::has_aura(world, owner, COUNTER) {
        world.entity_mut(actor).insert(CountedGhoul);
    } else {
        warn!(?owner, ?actor, "native Outnumber counter refused");
    }
}

fn remove_ghoul(world: &mut World, actor: Entity) {
    if world.get::<CountedGhoul>(actor).is_none() {
        return;
    }
    let Some(owner) = world
        .get::<SummonedCreature>(actor)
        .map(|summon| summon.owner)
    else {
        return;
    };
    let instance = world.get::<ActiveAuras>(owner).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == COUNTER && aura.caster == owner)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        auras::spend_stack(world, owner, instance);
    }
}
