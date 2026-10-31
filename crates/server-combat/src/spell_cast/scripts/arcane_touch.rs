//! SimC76f8118 sc_mage.cpp1408–1424/1914–1923: caster-owned Touch pool.
use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::auras::{self, ActiveAuras, AuraInstance, RemoveReason};
use crate::spell_cast::{self, effects::EffectCtx};
use bevy::prelude::*;

const TOUCH: u32 = 210824;
const EXPLOSION: u32 = 210833;

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        TOUCH,
        AuraScript {
            on_remove: Some(discharge),
            ..Default::default()
        },
    );
}

pub(super) fn apply(
    world: &mut World,
    ctx: &EffectCtx,
    _: &crate::spell_info::SpellEffect,
    target: Entity,
) {
    // sc_mage.cpp5192–5199: discharge the old pool before replacing its mark.
    auras::remove_auras(world, target, RemoveReason::Replaced, |aura| {
        aura.spell_id == TOUCH && aura.caster == ctx.caster
    });
    auras::apply_aura(
        world,
        &auras::AuraApplication {
            caster: ctx.caster,
            target,
            spell_id: TOUCH,
            effect_mask: u32::MAX,
            source: auras::AuraSource::Cast,
        },
    );
}

pub(in crate::spell_cast) fn record_damage(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    amount: f32,
) {
    let mage_spell = ctx
        .spell
        .class_options
        .as_ref()
        .is_some_and(|options| options.spell_class_set == 3);
    if !mage_spell || amount <= 0.0 || ctx.spell.id == EXPLOSION {
        return;
    }
    // Factor out only positive native aura87 vulnerability; caster-specific
    // aura270/271 modifiers remain in the stored hit (sc_mage.cpp1919–1923).
    let taken = world
        .get::<auras::AuraStatMods>(target)
        .map_or(1.0, |mods| mods.damage_taken(ctx.spell.school_mask()));
    let now = spell_cast::now(world);
    let Some(mut auras) = world.get_mut::<ActiveAuras>(target) else {
        return;
    };
    let instance = auras
        .iter()
        .find(|aura| {
            aura.spell_id == TOUCH
                && aura.caster == ctx.caster
                && aura.expires_at.is_none_or(|end| end >= now)
        })
        .map(|aura| aura.instance_id);
    let Some(aura) = instance.and_then(|id| auras.get_mut(id)) else {
        return;
    };
    if let Some(pool) = aura.effects.iter_mut().find(|effect| effect.index == 0) {
        pool.amount += amount / taken.max(1.0);
    }
}

fn discharge(world: &mut World, target: Entity, aura: &AuraInstance, _: RemoveReason) {
    let Some(pool) = aura.effects.iter().find(|effect| effect.index == 0) else {
        return;
    };
    if pool.amount <= 0.0 {
        return;
    }
    let fraction = super::script_effect_value(world, aura.caster, 321507, 0) / 100.0;
    // SimC sc_mage.cpp1416–1425 executes the area on debuff expiry, including
    // demise. Retain the primary for its full-damage policy but do not validate
    // the dead unit as the explicit target of the area.
    let Some(position) = spell_cast::targets::position(world, target) else {
        warn!(
            ?target,
            "Touch of the Magi explosion has no target position"
        );
        return;
    };
    let Some(mut spell) = spell_cast::load_triggered_spell(world, EXPLOSION, 1) else {
        return;
    };
    let amount = pool.amount * fraction;
    let Some(effect) = spell.effects.iter_mut().find(|effect| effect.index == 0) else {
        return;
    };
    effect.base_points = amount;
    spell.cast_effect0_amount = Some(amount);
    spell.cast_destination = Some(position.to_array());
    spell_cast::launch_triggered_spell_with_primary(
        world,
        aura.caster,
        &spell,
        None,
        Some(target),
        1,
    );
}
