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
    FillStacks(u32),
    GrantForCastDuration(u32),
    Decrement {
        aura: u32,
        unless_aura: Option<u32>,
    },
    GrantIfTalent {
        talent: u32,
        aura: u32,
    },
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
    // SimC76f8118 sc_mage.cpp4760–4761: selected Sunfury + Pyroclasm
    // executes one native buff application, without the HS-spend chance roll.
    CastRule {
        name: "SIMC_FIRE_SUNFURY_METEOR_PYROCLASM",
        spec: 63,
        talent: 449349,
        cast: 153561,
        operation: Operation::GrantIfTalent {
            talent: 269650,
            aura: 269651,
        },
    },
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
    // SimC76f8118 sc_hunter.cpp4482–4485: selected Stalk and Strike
    // grants Lock and Load on the actual Chakram cast, not on each bounce.
    CastRule {
        name: "SIMC_MM_STALK_LOCK_AND_LOAD",
        spec: 254,
        talent: 1266069,
        cast: 1264949,
        operation: Operation::Grant(194594),
    },
    // sc_hunter.cpp5803–5806/5596: fill the native buff's actual maximum,
    // then spend exactly one use per Aimed Shot. Native70448 has one use.
    CastRule {
        name: "SIMC_MM_BULLETSTORM_PRODUCER",
        spec: 254,
        talent: 389019,
        cast: 257044,
        operation: Operation::FillStacks(389020),
    },
    CastRule {
        name: "SIMC_MM_BULLETSTORM_CONSUMER",
        spec: 254,
        talent: 389019,
        cast: 19434,
        operation: Operation::Decrement {
            aura: 389020,
            unless_aura: None,
        },
    },
    // sc_hunter.cpp6990–6994/3632–3637: Volley grants Trick Shots for
    // its native field duration; ordinary Aimed/Rapid consume only outside Volley.
    CastRule {
        name: "SIMC_MM_VOLLEY_TRICK_SHOTS",
        spec: 254,
        talent: 260243,
        cast: 260243,
        operation: Operation::GrantForCastDuration(257622),
    },
    CastRule {
        name: "SIMC_MM_AIMED_TRICK_CONSUMER",
        spec: 254,
        talent: 257621,
        cast: 19434,
        operation: Operation::Decrement {
            aura: 257622,
            unless_aura: Some(260243),
        },
    },
    // sc_hunter.cpp5836 consumes on last_tick, not at channel launch.
    // The native Rapid Fire aura's natural-end hook owns that boundary.
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

struct CritRule {
    set_aura: u32,
    modifier: u32,
    actions: &'static [u32],
}

const CRIT_RULES: &[CritRule] = &[
    // SimC76f8118 sc_mage.cpp2369–2370: MID2 adds100% crit for Pyroclasm
    // hardcasts. Native2060 supplies the earned2pc identity; never grant it.
    CritRule {
        set_aura: 1296583,
        modifier: 269651,
        actions: &[11366, 2120, 1254851],
    },
];

pub(in crate::spell_cast) fn guarantees_crit(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> bool {
    spell.cast_was_hardcast
        && CRIT_RULES.iter().any(|rule| {
            rule.actions.contains(&spell.id)
                && owns_aura(world, caster, rule.set_aura)
                && used_modifier(spell, rule.modifier)
        })
}

fn selected(world: &World, caster: Entity, rule: &CastRule) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == rule.spec)
        && has_talent(world, caster, rule.talent)
}

pub(super) fn has_talent(world: &World, caster: Entity, talent: u32) -> bool {
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
    super::marksmanship_sentinel::after_cast(world, caster, spell);
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
        Operation::FillStacks(aura) => grant_maximum(world, caster, aura, rule.name),
        Operation::GrantForCastDuration(aura) => {
            grant_for_cast_duration(world, caster, spell, aura, rule.name)
        }
        Operation::Decrement { aura, unless_aura } => {
            if unless_aura.is_none_or(|excluded| !owns_aura(world, caster, excluded)) {
                decrement(world, caster, aura);
            }
        }
        Operation::GrantIfTalent { talent, aura } => {
            if has_talent(world, caster, talent) {
                grant(world, caster, aura, rule.name);
            }
        }
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

pub(super) fn decrement(world: &mut World, caster: Entity, spell: u32) {
    let stacks = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.caster == caster && aura.spell_id == spell)
            .map(|aura| aura.stacks)
    });
    match stacks {
        Some(0 | 1) => {
            consume(world, caster, spell);
        }
        Some(count) => {
            crate::auras::set_self_cast_stacks(world, caster, spell, count - 1);
        }
        None => {}
    }
}

fn grant_maximum(world: &mut World, caster: Entity, aura: u32, policy: &'static str) {
    let buff = world
        .resource::<crate::spell_info::SpellStore>()
        .get(aura)
        .expect("relationship requires native buff");
    let maximum = crate::auras::fresh_max_stacks(world, caster, buff).max(1);
    grant_with_overrides(
        world,
        caster,
        aura,
        policy,
        super::super::TriggeredSpellOverrides {
            aura_stacks: Some(maximum),
            ..Default::default()
        },
    );
}

fn grant_for_cast_duration(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    aura: u32,
    policy: &'static str,
) {
    let duration = super::super::duration_ms(world, caster, spell) as i32;
    grant_with_overrides(
        world,
        caster,
        aura,
        policy,
        super::super::TriggeredSpellOverrides {
            duration_ms: Some(duration),
            ..Default::default()
        },
    );
}

fn grant_with_overrides(
    world: &mut World,
    caster: Entity,
    spell: u32,
    policy: &'static str,
    values: super::super::TriggeredSpellOverrides,
) {
    if !super::super::cast_triggered_with_overrides(world, caster, spell, Some(caster), 1, values) {
        warn!(?caster, spell, policy, "Native relationship aura refused");
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
