//! SimC76f8118 sc_druid.cpp3745–3749/4879/11313–11316: earned Stalking uses.
use super::{combat, feral_unseen};
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::{CastPolicyContext, SpellInfo};
use shared::components::PowerType;

const TIGERS_FURY: u32 = 5217;
const TALENT: u32 = 1263660;
const STALKING: u32 = 1263962;
const CAT: u32 = 768;
// Pinned Cat attacks used by the literal Feral action set, including resolved Moonfire.
const CAT_ATTACKS: &[u32] = &[
    1079, 1822, 5221, 22568, 106785, 106830, 155625, 202028, 274837, 285381, 1243807,
];

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AuraRollAfterCast(after_cast),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
];

fn owns_proc(spell: u32) -> bool {
    matches!(spell, TALENT | STALKING)
}

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    if spell == TIGERS_FURY {
        Some(grant_stalking)
    } else if CAT_ATTACKS.contains(&spell) {
        Some(consume_stalking)
    } else {
        None
    }
}

fn selected(world: &World, caster: Entity) -> bool {
    world
        .get::<combat::class_progression::TraitLoadout>(caster)
        .is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == TALENT)
        })
}

fn grant_stalking(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !selected(world, caster) {
        return;
    }
    let Some(mut spell) = spell_cast::load_triggered_spell(world, STALKING, 1) else {
        return;
    };
    // sc_druid.cpp11313–11316 initializes to the native maximum, not one stack.
    let maximum = spell
        .aura_options
        .as_ref()
        .expect("native Stalking retains its stack metadata")
        .cumulative_aura;
    assert!(
        maximum > 0,
        "native Stalking requires a positive stack maximum"
    );
    spell.cast_aura_stacks =
        Some(u8::try_from(maximum).expect("native Stalking stack maximum fits u8"));
    spell_cast::launch_triggered_spell(world, caster, &spell, Some(caster), 1);
}

fn consume_stalking(world: &mut World, caster: Entity, source: &SpellInfo) {
    if !selected(world, caster) || !auras::has_aura(world, caster, CAT) {
        return;
    }
    let instance = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == STALKING && aura.caster == caster)
        })
        .map(|aura| aura.instance_id);
    let Some(instance) = instance else {
        return;
    };
    if auras::procs::start_policy_proc(world, caster, instance).is_none() {
        return;
    }
    // Ordinary after-cast runs once per action, not per AoE recipient or proc child.
    // buff_t::consume (buff.cpp2838–2871) spends one stack and starts its native ICD.
    auras::spend_stack(world, caster, instance);
    launch_guaranteed(world, caster, source);
}

fn launch_guaranteed(world: &mut World, caster: Entity, source: &SpellInfo) {
    let (_, maximum) = spell_cast::powers::power_value(world, caster, PowerType::ComboPoints)
        .expect("native Stalking Cat has a combo-point pool");
    // sc_druid.cpp3748–3749: maximum effective points and full extra-Energy fraction;
    // these retained inputs do not grant or spend either actual resource pool.
    let context = Box::new(CastPolicyContext {
        resources: vec![(PowerType::ComboPoints, maximum)],
        free_spender: false,
        targets: source.cast_primary_target.into_iter().collect(),
    });
    feral_unseen::launch_effective_unseen(world, caster, 1, maximum, 1.0, context);
}
