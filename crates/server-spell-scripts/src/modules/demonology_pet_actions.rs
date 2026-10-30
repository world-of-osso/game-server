//! Source-qualified Demonology pet actions.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::combat::AutoAttack;
use combat::hunter_pet::HunterPet;
use combat::spell_cast::script_registry::{SpellScriptHook, TargetPlanHook};
use combat::spell_cast::summon::SummonedCreature;
use combat::spell_cast::targets::{self, Relation};
use combat::spell_cast::{self, PlannedEffect, TargetPlan};
use combat::spell_info::SpellInfo;
use shared::components::{ActiveSpec, Npc};

const DREADBITE: u32 = 271971;
const DREADLASH: u32 = 264078;
const BLIGHTED_MAW: u32 = 1276956;
const BLIGHTED_DAMAGE: u32 = 1276960;
const FELSTORM: u32 = 89751;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AdditionalReachActions(autonomous_actions),
    SpellScriptHook::TargetPlan(target_plan),
    SpellScriptHook::ResolvedDamage(blighted_maw),
    SpellScriptHook::AfterQueuedTriggers(felstorm),
];

fn autonomous_actions(world: &World, unit: Entity) -> Vec<u32> {
    let Some(pet) = world.get::<HunterPet>(unit) else {
        return Vec::new();
    };
    let felguard = world
        .get::<Npc>(unit)
        .is_some_and(|npc| npc.template_id == 17252);
    let demonology = world
        .get::<ActiveSpec>(pet.owner)
        .is_some_and(|spec| spec.0 == 266);
    if felguard && demonology {
        vec![FELSTORM]
    } else {
        Vec::new()
    }
}

fn target_plan(spell: u32) -> Option<(Relation, TargetPlanHook)> {
    (spell == DREADBITE).then_some((Relation::Enemy, plan_dreadbite))
}

fn plan_dreadbite(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let primary = spell
        .cast_primary_target
        .expect("Dreadbite needs its primary target");
    let dreadlash = world
        .get::<SummonedCreature>(caster)
        .is_some_and(|summon| auras::has_aura(world, summon.owner, DREADLASH));
    spell
        .effects
        .iter()
        .enumerate()
        .filter(|(_, effect)| effect.effect != 0)
        .map(|(index, effect)| {
            let native = targets::select_targets(world, caster, explicit, spell, effect, 0, false)
                .expect("native Dreadbite selectors");
            PlannedEffect {
                index,
                units: if dreadlash { native } else { vec![primary] },
                selected_units: Vec::new(),
                dest: targets::effect_dest(world, caster, explicit, effect, spell.cast_destination),
                chain_multipliers: Vec::new(),
                blocked_units: Vec::new(),
                missed_units: Vec::new(),
                prevent_hit_script: false,
            }
        })
        .collect()
}

// SimC76f8118 pets.cpp1098–1110: the owner deals a native percentage of the actual bite.
fn blighted_maw(
    world: &mut World,
    ctx: &spell_cast::effects::EffectCtx<'_>,
    target: Entity,
    _: bool,
    amount: f32,
) {
    if ctx.spell.id != DREADBITE || amount <= 0.0 {
        return;
    }
    let Some(owner) = world.get::<SummonedCreature>(ctx.caster).map(|s| s.owner) else {
        return;
    };
    if !auras::has_aura(world, owner, BLIGHTED_MAW) {
        return;
    }
    let percent = spell_cast::scripts::script_effect_value(world, owner, BLIGHTED_MAW, 0);
    spell_cast::cast_triggered_with_effect0_amount(
        world,
        owner,
        BLIGHTED_DAMAGE,
        Some(target),
        ctx.depth + 1,
        amount * percent / 100.0,
    );
}

// pets.cpp536–554: Felguard prioritizes its real Felstorm when ready, before Legion Strike.
fn felstorm(world: &mut World) {
    let pets: Vec<_> = world
        .query::<(Entity, &HunterPet, &Npc, &AutoAttack)>()
        .iter(world)
        .filter(|(_, pet, npc, _)| {
            npc.template_id == 17252
                && world
                    .get::<ActiveSpec>(pet.owner)
                    .is_some_and(|spec| spec.0 == 266)
        })
        .map(|(entity, _, _, _)| entity)
        .collect();
    for pet in pets {
        if world.get::<shared::casting::CastState>(pet).is_none() {
            if let Err(error) = spell_cast::creature_cast(world, pet, FELSTORM, Some(pet), false) {
                trace!(?pet, ?error, "Felstorm not ready");
            }
        }
    }
}
