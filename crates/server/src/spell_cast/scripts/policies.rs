//! Explicit missing-script policies; native spell records still own payloads.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::AfterCastHook;
use crate::class_progression::TraitLoadout;
use crate::spell_info::{SpellInfo, SpellStore};

struct CounterProducer {
    name: &'static str,
    spec: u32,
    talent: u32,
    parent: u32,
    counter: u32,
    count_effect: u8,
}

// SimC76f8118 sc_paladin_retribution.cpp846–876 supplies this missing edge.
// Native modified431377/effect0 is3; raw2 and counter maximum6 are not grants.
const POLICIES: &[CounterProducer] = &[CounterProducer {
    name: "SIMC_RET_DAWNLIGHT_WAKE_PRODUCER",
    spec: 70,
    talent: 431377,
    parent: 255937,
    counter: 431522,
    count_effect: 0,
}];

fn producers(parent: u32) -> impl Iterator<Item = &'static CounterProducer> {
    POLICIES
        .iter()
        .filter(move |policy| policy.parent == parent)
}

pub(super) fn after_cast_hook(parent: u32) -> Option<AfterCastHook> {
    producers(parent)
        .next()
        .map(|_| apply_after_cast as AfterCastHook)
}

fn selected(world: &World, caster: Entity, policy: &CounterProducer) -> bool {
    let correct_spec = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == policy.spec);
    let owns_talent = world.get::<TraitLoadout>(caster).is_some_and(|talents| {
        talents
            .active_talent_spells()
            .any(|spell| spell.spell_id == policy.talent)
    });
    correct_spec && owns_talent
}

fn apply_after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for policy in producers(spell.id) {
        if selected(world, caster, policy) {
            grant_counter(world, caster, policy);
        }
    }
}

fn grant_counter(world: &mut World, caster: Entity, policy: &CounterProducer) {
    let count = super::script_effect_value(world, caster, policy.talent, policy.count_effect);
    if !count.is_finite() || !(1.0..=f32::from(u8::MAX)).contains(&count) {
        warn!(
            policy = policy.name,
            ?caster,
            count,
            "Native policy counter amount invalid"
        );
        return;
    }
    let Some(mut counter) = world.resource::<SpellStore>().get(policy.counter).cloned() else {
        warn!(
            policy = policy.name,
            spell_id = policy.counter,
            "Native policy counter unavailable"
        );
        return;
    };
    counter.cast_aura_stacks = Some(count as u8);
    if !super::super::launch_triggered_spell(world, caster, &counter, Some(caster), 1) {
        warn!(
            policy = policy.name,
            ?caster,
            "Native policy counter refused"
        );
    }
}
