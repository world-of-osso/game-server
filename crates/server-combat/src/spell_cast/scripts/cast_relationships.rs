//! Source-qualified successful-cast relationships; native records own aura payloads.
use crate::auras::procs::{AuraEvent, PROC2_CAST_SUCCESSFUL};
use crate::auras::{ActiveAuras, AuraInstance, RemoveReason};
use crate::class_progression::TraitLoadout;
use crate::spell_info::SpellInfo;
use bevy::prelude::*;
use shared::components::ActiveSpec;

#[derive(Clone, Copy)]
enum Operation {
    Grant(u32),
    Consume {
        aura: u32,
        conditional_grant: Option<(u32, u32)>,
    },
    NativeProc,
    FreeRecast(u32),
}

struct CastRule {
    name: &'static str,
    spec: u32,
    talent: u32,
    cast: u32,
    operation: Operation,
}

const PRECISE_CONSUMER: Operation = Operation::Consume {
    aura: 260242,
    conditional_grant: Some((1253751, 1253750)),
};

const RULES: &[CastRule] = &[
    // SimC76f8118 sc_hunter.cpp6913–6914: only Trueshot grants Chakram.
    // Native1264902 aura42 supplies1264946 and its actual duration/chance.
    CastRule {
        name: "SIMC_MM_TRUESHOT_CHAKRAM",
        spec: 254,
        talent: 1264902,
        cast: 288613,
        operation: Operation::NativeProc,
    },
    // sc_hunter.cpp4480–4481 consumes the actual enabler on a Chakram cast.
    CastRule {
        name: "SIMC_MM_CHAKRAM_CONSUMER",
        spec: 254,
        talent: 1264902,
        cast: 1264949,
        operation: Operation::Consume {
            aura: 1264946,
            conditional_grant: None,
        },
    },
    // sc_hunter.cpp5565–5567: one native Precise Shots application per Aimed Shot.
    CastRule {
        name: "SIMC_MM_AIMED_PRECISE_SHOTS",
        spec: 254,
        talent: 260240,
        cast: 19434,
        operation: Operation::Grant(260242),
    },
    // sc_hunter.cpp5796–5799: selected No Scope grants the same native buff.
    CastRule {
        name: "SIMC_MM_NO_SCOPE_PRECISE_SHOTS",
        spec: 254,
        talent: 473385,
        cast: 257044,
        operation: Operation::Grant(260242),
    },
    // sc_hunter.cpp3640–3647/4127–4130/4249–4252/5215–5217:
    // a real spender expires Precise Shots and triggers selected Stargazer.
    CastRule {
        name: "SIMC_MM_ARCANE_PRECISE_CONSUMER",
        spec: 254,
        talent: 260240,
        cast: 185358,
        operation: PRECISE_CONSUMER,
    },
    CastRule {
        name: "SIMC_MM_MULTI_PRECISE_CONSUMER",
        spec: 254,
        talent: 260240,
        cast: 257620,
        operation: PRECISE_CONSUMER,
    },
    CastRule {
        name: "SIMC_MM_KILL_PRECISE_CONSUMER",
        spec: 254,
        talent: 260240,
        cast: 53351,
        operation: PRECISE_CONSUMER,
    },
    // sc_hunter.cpp5927–5940/5959–5980: one free, cooldown-independent recast.
    // Native1301778 owns its three-second duration and -100% cost modifier.
    CastRule {
        name: "SIMC_MM_UNSTABLE_TRIGGER_RECAST",
        spec: 254,
        talent: 473520,
        cast: 212431,
        operation: Operation::FreeRecast(1301778),
    },
];

fn selected(world: &World, caster: Entity, rule: &CastRule) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == rule.spec)
        && has_talent(world, caster, rule.talent)
}

fn has_talent(world: &World, caster: Entity, talent: u32) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == talent)
    })
}

fn owns_aura(world: &World, caster: Entity, spell: u32) -> bool {
    let now = super::super::now(world);
    world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.caster == caster
                && aura.spell_id == spell
                && aura.expires_at.is_none_or(|at| at > now)
        })
    })
}

pub(crate) fn register(scripts: &mut crate::auras::scripts::AuraScripts) {
    for rule in RULES
        .iter()
        .filter(|rule| matches!(rule.operation, Operation::NativeProc))
    {
        scripts.0.entry(rule.talent).or_default().check_proc = Some(check_native_proc);
    }
}

fn check_native_proc(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    // Only explicitly assigned actor/spec policies add a veto. Other users of
    // the same native hero talent are unchanged, not qualified by the MM proof.
    let assigned = RULES.iter().any(|rule| {
        matches!(rule.operation, Operation::NativeProc)
            && rule.talent == aura.spell_id
            && world
                .get::<ActiveSpec>(aura.caster)
                .is_some_and(|spec| spec.0 == rule.spec)
    });
    if !assigned {
        return true;
    }
    if !is_owned_successful_cast(event, aura.caster) {
        return false;
    }
    RULES.iter().any(|rule| {
        let matching_proc =
            matches!(rule.operation, Operation::NativeProc) && rule.talent == aura.spell_id;
        matching_proc && selected(world, aura.caster, rule) && event.spell_id == Some(rule.cast)
    })
}

fn is_owned_successful_cast(event: &AuraEvent, caster: Entity) -> bool {
    let owned = event.actor == caster && event.can_proc;
    let successful = event.actor_mask[1] & PROC2_CAST_SUCCESSFUL != 0;
    owned && successful
}

/// sc_hunter.cpp3640–3647: damage is established before Precise Shots expires.
/// Reuse the native launch snapshot rather than letting flight lose its modifier.
pub(super) fn snapshots_modifier(id: u32) -> bool {
    RULES
        .iter()
        .any(|rule| matches!(rule.operation, Operation::Consume { aura, .. } if aura == id))
}

fn used_modifier(spell: &SpellInfo, aura: u32) -> bool {
    spell.cast_charge_mods.as_ref().is_some_and(|modifiers| {
        modifiers
            .iter()
            .any(|modifier| modifier.aura_spell_id == aura)
    })
}

/// Ordinary native readiness and cooldown storage share this same sourced window.
pub(in crate::spell_cast) fn ignores_cooldown(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> bool {
    RULES
        .iter()
        .filter(|rule| rule.cast == spell.id && selected(world, caster, rule))
        .any(|rule| match rule.operation {
            Operation::FreeRecast(aura) => {
                owns_aura(world, caster, aura) || used_modifier(spell, aura)
            }
            _ => false,
        })
}

pub(in crate::spell_cast) fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for rule in RULES.iter().filter(|rule| rule.cast == spell.id) {
        if !selected(world, caster, rule) {
            continue;
        }
        apply_rule(world, caster, spell, rule);
    }
}

fn apply_rule(world: &mut World, caster: Entity, spell: &SpellInfo, rule: &CastRule) {
    match rule.operation {
        Operation::Grant(aura) => grant(world, caster, aura, rule.name),
        Operation::Consume {
            aura,
            conditional_grant,
        } => {
            if consume(world, caster, aura)
                && let Some((talent, buff)) = conditional_grant
                && has_talent(world, caster, talent)
            {
                grant(world, caster, buff, rule.name);
            }
        }
        Operation::FreeRecast(aura) => {
            let used = used_modifier(spell, aura) || owns_aura(world, caster, aura);
            if used {
                consume(world, caster, aura);
            } else {
                grant(world, caster, aura, rule.name);
            }
        }
        Operation::NativeProc => {}
    }
}

fn consume(world: &mut World, caster: Entity, spell: u32) -> bool {
    !crate::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.caster == caster && aura.spell_id == spell
    })
    .is_empty()
}

fn grant(world: &mut World, caster: Entity, spell: u32, policy: &'static str) {
    if !super::super::try_cast_triggered(world, caster, spell, Some(caster), 1) {
        warn!(?caster, spell, policy, "Native relationship aura refused");
    }
}
