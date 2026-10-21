//! TC a352b1fa SpellAuraEffects.cpp4949–4952 and SpellEffects.cpp3165–3169.
//! Hat removal casts the authored despawn child; source conditions select owned Coyotes.
use bevy::prelude::*;

use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::auras::{AuraInstance, RemoveReason};
use crate::spell_info::{SpellEffect, SpellInfo, SpellStore};

use super::effects::EffectCtx;

pub(crate) const HAT: u32 = 52172;
pub(super) const DESPAWN: u32 = 52173;
const SUMMON: u32 = 51149;
const COYOTE: u32 = 28267;
const PROPERTIES: i32 = 41;

fn is_hat_aura(effect: &SpellEffect) -> bool {
    let source = (
        effect.index,
        effect.effect,
        effect.aura,
        effect.implicit_targets,
    ) == (0, 6, 4, [1, 0]);
    let child = effect.base_points == DESPAWN as f32 && effect.trigger_spell == 0;
    source && child && no_misc_payload(effect)
}

fn no_misc_payload(effect: &SpellEffect) -> bool {
    effect.misc_values == [0, 0] && effect.item_type == 0 && effect.mechanic == 0
}

fn is_summon_trigger(effect: &SpellEffect) -> bool {
    let source = (
        effect.index,
        effect.effect,
        effect.aura,
        effect.implicit_targets,
    ) == (1, 64, 0, [0, 0]);
    let child = (effect.trigger_spell, effect.base_points) == (SUMMON, 0.0);
    source && child && no_misc_payload(effect)
}

fn has_hat_signature(spell: &SpellInfo) -> bool {
    let effects: Vec<_> = spell
        .effects
        .iter()
        .filter(|effect| effect.effect != 0)
        .collect();
    effects.len() == 2
        && effects.iter().any(|effect| is_hat_aura(effect))
        && effects.iter().any(|effect| is_summon_trigger(effect))
}

fn has_summon_signature(spell: &SpellInfo) -> bool {
    super::toy_scripts::single_effect(spell).is_some_and(|effect| {
        let source = (
            effect.index,
            effect.effect,
            effect.aura,
            effect.implicit_targets,
        ) == (0, 28, 0, [32, 0]);
        let creature = effect.misc_values == [COYOTE as i32, PROPERTIES];
        let payload = (
            effect.base_points,
            effect.trigger_spell,
            effect.item_type,
            effect.mechanic,
        ) == (1.0, 0, 0, 0);
        source && creature && payload
    })
}

pub(super) fn supports_effect(spell: u32, effect: &SpellEffect) -> bool {
    match spell {
        HAT => is_hat_aura(effect),
        DESPAWN => super::toy_scripts::has_effect_shape(effect, 77, [22, 7], 0),
        _ => false,
    }
}

pub(super) fn unsupported(world: &World, spell: &SpellInfo) -> Option<String> {
    let store = world.resource::<SpellStore>();
    if spell.id == HAT {
        if !has_hat_signature(spell) {
            return Some("Coyote Hat source signature differs".into());
        }
        if !store.get(SUMMON).is_some_and(has_summon_signature) {
            return Some("Coyote summon51149 source signature unavailable".into());
        }
    }
    let child = store
        .get(DESPAWN)
        .and_then(super::toy_scripts::single_effect);
    if !child.is_some_and(|effect| supports_effect(DESPAWN, effect)) {
        return Some("Coyote removal child52173 source signature unavailable".into());
    }
    let conditions = world.get_resource::<crate::conditions::SpellTargetConditions>();
    let owned_filter =
        conditions.is_some_and(|conditions| conditions.has_owned_entry_filter(DESPAWN, 0, COYOTE));
    (!owned_filter)
        .then(|| "Coyote removal child52173 source owned-entry conditions unavailable".into())
}

pub(crate) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        HAT,
        AuraScript {
            on_remove: Some(cast_removal_child),
            ..default()
        },
    );
}

fn cast_removal_child(world: &mut World, unit: Entity, aura: &AuraInstance, _reason: RemoveReason) {
    let Some(effect) = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0 && effect.aura_type == 4)
    else {
        warn!("Coyote Hat52172 removal has no source dummy effect0");
        return;
    };
    // TC casts the computed aura amount, not an independently invented action.
    super::cast_triggered(world, unit, effect.amount as u32, None, 0);
}

pub(super) fn despawn(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    if !supports_effect(ctx.spell.id, effect) {
        warn!("Coyote removal52173 source effect signature differs");
        return;
    }
    let temporary = world
        .get::<super::summon::SummonedCreature>(target)
        .is_some()
        || world.get::<crate::zones::TempSummon>(target).is_some();
    if world.get::<shared::components::Npc>(target).is_some() && temporary {
        super::summon::despawn_summon(world, target);
    }
}
