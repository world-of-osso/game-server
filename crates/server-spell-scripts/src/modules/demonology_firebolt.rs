//! Source-qualified Wild Imp Fel Firebolt recipients.
use super::combat;
use bevy::prelude::*;
use combat::random::seq::SliceRandom;
use combat::spell_cast::{
    PlannedEffect, TargetPlan,
    script_registry::{SpellScriptHook, TargetPlanHook},
    summon::SummonedCreature,
    targets::{self, Relation},
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::Npc;

const FEL_FIREBOLT: u32 = 104318;
const UNSTABLE_SOUL: u32 = 1281512;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetPlan(target_plan)];

fn target_plan(spell: u32) -> Option<(Relation, TargetPlanHook)> {
    (spell == FEL_FIREBOLT).then_some((Relation::Enemy, plan_firebolt))
}

fn plan_firebolt(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let primary = explicit.expect("native Fel Firebolt requires its primary target");
    let imp = world
        .get::<Npc>(caster)
        .is_some_and(|npc| matches!(npc.template_id, 55659 | 143622))
        && world.get::<SummonedCreature>(caster).is_some();
    let unstable = imp && combat::auras::has_aura(world, caster, UNSTABLE_SOUL);
    let extra = if unstable {
        world
            .resource::<SpellStore>()
            .get(UNSTABLE_SOUL)
            .expect("native Unstable Soul")
            .effects
            .iter()
            .find(|effect| effect.index == 1)
            .expect("native Unstable Soul effect2")
            .base_points as usize
    } else {
        0
    };
    spell
        .effects
        .iter()
        .enumerate()
        .filter(|(_, effect)| effect.effect != 0)
        .map(|(index, effect)| {
            let native = targets::select_targets(world, caster, explicit, spell, effect, 0, false)
                .expect("native Fel Firebolt target selectors");
            let units = if imp && effect.index == 0 {
                firebolt_recipients(primary, native, unstable, extra)
            } else {
                native
            };
            PlannedEffect {
                index,
                units,
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

fn firebolt_recipients(
    primary: Entity,
    candidates: Vec<Entity>,
    unstable: bool,
    extra: usize,
) -> Vec<Entity> {
    // SimC76f8118 pets.cpp721–793: ordinary bolts are single-target;
    // Unstable Soul uses its native extra count and repeats available targets.
    if !unstable {
        return vec![primary];
    }
    let mut unique = vec![primary];
    unique.extend(candidates.into_iter().filter(|&target| target != primary));
    if unique.len() == 1 {
        return unique;
    }
    let total = extra + 1;
    let mut recipients = unique.iter().copied().take(total).collect::<Vec<_>>();
    while recipients.len() + unique.len() <= total {
        recipients.extend_from_slice(&unique);
    }
    let remaining = total - recipients.len();
    if remaining > 0 {
        unique.shuffle(&mut combat::random::thread_rng());
        recipients.extend(unique.into_iter().take(remaining));
    }
    recipients
}
