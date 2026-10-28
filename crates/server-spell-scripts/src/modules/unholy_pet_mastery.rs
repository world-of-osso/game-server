//! Unholy pets inherit the owner's live Dreadblade critical-damage mastery.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, script_registry::SpellScriptHook, summon::SummonedCreature};
use combat::spell_info::SpellStore;
use combat::stats::CharacterStats;
use shared::components::{ActiveSpec, Npc};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterCastUpdate(update)];

fn update(world: &mut World) {
    let mut pets: Vec<_> = world.query::<(Entity, &SummonedCreature, &Npc)>().iter(world)
        .filter_map(|(pet, summon, npc)| matches!((summon.spell_id, npc.template_id),
            (275430 | 1294026, 237409) | (317776 | 1242294, 163366)
            | (1292072, 264321) | (288853, 149555)
            | (444248, 221632) | (444251, 221633) | (444252, 221634) | (444254, 221635))
            .then_some((pet, summon.owner)))
        .collect();
    pets.extend(world.query::<(Entity, &combat::hunter_pet::HunterPet, &Npc)>().iter(world)
        .filter_map(|(pet, relation, npc)| (npc.template_id == 26125).then_some((pet, relation.owner))));
    for (pet, owner) in pets { refresh(world, owner, pet); }
}

fn refresh(world: &mut World, owner: Entity, pet: Entity) {
    if world.get::<ActiveSpec>(owner).is_none_or(|spec| spec.0 != 252) { return; }
    let effect = world.resource::<SpellStore>().get(77515).expect("native Dreadblade")
        .effects.iter().find(|effect| effect.index == 5).expect("native Dreadblade effect6");
    let mastery = world.get::<CharacterStats>(owner).expect("Unholy pet owner stats").mastery_pct;
    // SimC76f8118 sc_death_knight.cpp2651–2677/2730–2738: live owner mastery
    // increases each actual DK pet's critical damage through Dreadblade effect6.
    let amount = effect.base_points + effect.bonus_coefficient * mastery;
    let instance = world.get::<ActiveAuras>(pet).and_then(|auras| auras.iter()
        .find(|aura| aura.spell_id == 1250728 && aura.caster == pet)
        .map(|aura| (aura.instance_id, aura.effects.iter().find(|effect| effect.index == 0).map(|effect| effect.amount))));
    let instance = match instance {
        Some((_, Some(current))) if current == amount => return,
        Some((id, _)) => id,
        None => {
            if !spell_cast::try_cast_triggered(world, pet, 1250728, Some(pet), 1) {
                warn!(?pet, "native Dreadblade pet aura refused"); return;
            }
            world.get::<ActiveAuras>(pet).expect("Dreadblade pet auras").iter()
                .find(|aura| aura.spell_id == 1250728 && aura.caster == pet)
                .expect("Dreadblade pet aura must be applied").instance_id
        }
    };
    assert!(combat::auras::set_effect_amount(world, pet, instance, 0, amount), "Dreadblade pet effect must exist");
}
