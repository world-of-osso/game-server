//! SIMC_RET_EXECUTION_SENTENCE_ACCUMULATION: own nonphysical damage only.
//! SimC76f8118 sc_paladin.cpp3590–3612; retribution.cpp1048–1076.
//! TC a352b1fa spell_paladin.cpp771–804 owns native percentage/expiry discharge.
use bevy::prelude::*;

use crate::auras::procs::AuraEvent;
use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::auras::{ActiveAuras, AuraInstance, RemoveReason};
use crate::spell_info::SpellStore;

const SENTENCE: u32 = 343527;
const EXPLOSION: u32 = 387113;
const PHYSICAL_SCHOOL: u32 = 1;

pub(crate) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        SENTENCE,
        AuraScript {
            on_remove: Some(discharge_on_expiry),
            ..Default::default()
        },
    );
}

/// An outgoing-damage assessor, not a fabricated proc mask/chance. Physical
/// hybrids are excluded too (SimC dbc::is_school tests inclusion of the bit).
pub(crate) fn record_damage(world: &mut World, event: &AuraEvent) {
    if event.damage <= 0.0 || !qualifies_school(world, event.spell_id) {
        return;
    }
    let Some(target) = event.target else {
        return;
    };
    let now = crate::spell_cast::now(world);
    let Some(mut auras) = world.get_mut::<ActiveAuras>(target) else {
        return;
    };
    for aura in auras.auras.iter_mut().filter(|aura| {
        aura.spell_id == SENTENCE
            && aura.caster == event.actor
            && aura.expires_at.is_none_or(|expiry| now <= expiry)
    }) {
        accumulate(aura, event.damage);
    }
}

fn qualifies_school(world: &World, spell: Option<u32>) -> bool {
    spell
        .and_then(|id| world.resource::<SpellStore>().get(id))
        .is_some_and(|spell| spell.school_mask() & PHYSICAL_SCHOOL == 0)
}

fn accumulate(aura: &mut AuraInstance, damage: f32) {
    let Some(percent) = aura
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .map(|effect| effect.amount)
    else {
        return;
    };
    if let Some(counter) = aura.effects.iter_mut().find(|effect| effect.index == 0) {
        counter.amount += damage * percent / 100.0;
    }
}

fn discharge_on_expiry(
    world: &mut World,
    target: Entity,
    aura: &AuraInstance,
    reason: RemoveReason,
) {
    if reason != RemoveReason::Expired {
        return;
    }
    let Some(amount) = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .map(|effect| effect.amount)
        .filter(|amount| *amount > 0.0)
    else {
        return;
    };
    // TC IGNORE_CAST_IN_PROGRESS only: normal power/GCD/aura/cooldown admission,
    // preserving the BP0 override and ordinary child proc behavior.
    let result = crate::spell_cast::creature::creature_cast_with_effect0_ignoring_in_progress(
        world,
        aura.caster,
        EXPLOSION,
        Some(target),
        amount,
    );
    if let Err(reason) = result {
        debug!(
            ?reason,
            "Execution Sentence partial-trigger expiry cast refused"
        );
    }
}
