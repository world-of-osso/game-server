//! Explicit cast→counter→summon relationships; native rows own stacks/payloads.
use super::AfterCastHook;
use crate::auras::ActiveAuras;
use crate::class_progression::TraitLoadout;
use crate::random::Rng;
use crate::spell_cast::{self, summon::SummonedCreature};
use crate::spell_info::SpellInfo;
use bevy::prelude::*;
use shared::components::ActiveSpec;

struct SummonChoice {
    spell: u32,
    duplicate_redirect: Option<usize>,
}
struct Policy {
    spec: u32,
    talent: u32,
    starter: u32,
    advancing_casts: &'static [u32],
    counter: u32,
    refund_talent: u32,
    refund: u32,
    choices: &'static [SummonChoice],
}

// SimC76f8118 actions.cpp2865–2871/3670–3671/5334–5341,
// init.cpp781–788 and warlock.cpp1234–1264. This is a missing script edge,
// not a fabricated proc row. All amounts, aura limits and summons are native.
const POLICIES: &[Policy] = &[Policy {
    spec: 266,
    talent: 1276163,
    starter: 265187,
    advancing_casts: &[105174, 434635],
    counter: 1276166,
    refund_talent: 1276222,
    refund: 1276318,
    choices: &[
        SummonChoice {
            spell: 1276182,
            duplicate_redirect: None,
        },
        SummonChoice {
            spell: 1276283,
            duplicate_redirect: None,
        },
        SummonChoice {
            spell: 1282502,
            duplicate_redirect: Some(0),
        },
        SummonChoice {
            spell: 1282501,
            duplicate_redirect: Some(1),
        },
    ],
}];

struct CastSummons {
    spec: u32,
    parent: u32,
    count_effect: u8,
    first: u32,
    rest: u32,
    companion_talent: u32,
    companion: u32,
    variants: &'static [(u32, u32)],
}
// SimC76f8118 actions.cpp3412–3433/6014–6020: native root104316's
// count supplies first193331/rest193332; selected Vilefiend joins the call.
const CAST_SUMMONS: &[CastSummons] = &[CastSummons {
    spec: 266,
    parent: 104316,
    count_effect: 0,
    first: 193331,
    rest: 193332,
    companion_talent: 1251778,
    companion: 1251781,
    variants: &[(455449, 455465), (455450, 455476)],
}];

pub(super) fn hook(spell: u32) -> Option<AfterCastHook> {
    if CAST_SUMMONS.iter().any(|policy| policy.parent == spell) {
        return Some(apply);
    }
    POLICIES
        .iter()
        .any(|p| p.starter == spell || p.advancing_casts.contains(&spell))
        .then_some(apply)
}

fn selected(world: &World, caster: Entity, talent: u32) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|entry| entry.spell_id == talent)
    })
}

fn apply(world: &mut World, caster: Entity, spell: &SpellInfo) {
    summon_after_cast(world, caster, spell);
    for policy in POLICIES {
        advance_counter(world, caster, spell, policy);
    }
}

fn advance_counter(world: &mut World, caster: Entity, spell: &SpellInfo, policy: &Policy) {
    let correct_spec = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == policy.spec);
    if !correct_spec || !selected(world, caster, policy.talent) {
        return;
    }
    let starts = spell.id == policy.starter;
    let advances = policy.advancing_casts.contains(&spell.id)
        && crate::auras::has_aura(world, caster, policy.counter);
    if !starts && !advances {
        return;
    }
    grant_stack(world, caster, policy);
    if advances
        && selected(world, caster, policy.refund_talent)
        && !spell_cast::try_cast_triggered(world, caster, policy.refund, Some(caster), 1)
    {
        warn!(
            ?caster,
            spell = policy.refund,
            "Native stacking-summon refund refused"
        );
    }
}

fn summon_after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for policy in CAST_SUMMONS
        .iter()
        .filter(|policy| policy.parent == spell.id)
    {
        summon_for_policy(world, caster, spell, policy);
    }
}

fn summon_for_policy(world: &mut World, caster: Entity, spell: &SpellInfo, policy: &CastSummons) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == policy.spec)
    {
        return;
    }
    let count = spell
        .effects
        .iter()
        .find(|effect| effect.index == policy.count_effect)
        .expect("native summon counter effect")
        .base_points;
    assert!(
        count.is_finite() && count >= 0.0 && count <= u32::MAX as f32,
        "invalid native summon count"
    );
    for index in 0..count as u32 {
        let child = if index == 0 {
            policy.first
        } else {
            policy.rest
        };
        cast_summon(world, caster, child);
    }
    if selected(world, caster, policy.companion_talent) {
        let child = policy
            .variants
            .iter()
            .find(|(talent, _)| selected(world, caster, *talent))
            .map_or(policy.companion, |(_, spell)| *spell);
        cast_summon(world, caster, child);
    }
}

fn cast_summon(world: &mut World, caster: Entity, child: u32) {
    if !spell_cast::try_cast_triggered(world, caster, child, Some(caster), 1) {
        warn!(?caster, child, "Native after-cast summon refused");
    }
}

fn grant_stack(world: &mut World, caster: Entity, policy: &Policy) {
    if !spell_cast::try_cast_triggered(world, caster, policy.counter, Some(caster), 1) {
        warn!(
            ?caster,
            counter = policy.counter,
            "Native stacking-summon counter refused"
        );
        return;
    }
    let (instance, stacks, maximum) = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == policy.counter))
        .map(|aura| (aura.instance_id, aura.stacks, aura.max_stacks))
        .expect("successful native counter grant must apply its aura");
    assert!(
        maximum > 1,
        "stacking-summon counter requires a native cumulative limit"
    );
    if stacks < maximum {
        return;
    }
    summon_choice(world, caster, policy);
    // init.cpp785 queues this at zero delay: retain one actual native stack.
    // No cast/resource decision intervenes in this synchronous AfterCast callback.
    for _ in 1..maximum {
        crate::auras::spend_stack(world, caster, instance);
    }
}

fn summon_choice(world: &mut World, caster: Entity, policy: &Policy) {
    let index = crate::random::thread_rng().gen_range(0..policy.choices.len());
    let choice = &policy.choices[index];
    let spell = match choice.duplicate_redirect {
        Some(redirect) if has_living_summon(world, caster, choice.spell) => {
            policy.choices[redirect].spell
        }
        _ => choice.spell,
    };
    if !spell_cast::try_cast_triggered(world, caster, spell, Some(caster), 1) {
        warn!(?caster, spell, "Native stacking-summon child refused");
    }
}

fn has_living_summon(world: &mut World, owner: Entity, spell: u32) -> bool {
    world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .any(|(unit, summon)| {
            summon.owner == owner
                && summon.spell_id == spell
                && spell_cast::targets::is_alive(world, unit)
        })
}
