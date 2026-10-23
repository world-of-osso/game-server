//! Named Mainline12.1.0.69933 description consumers; no generic Dummy no-op.
use super::script_effect_value;
use crate::spell_info::SpellInfo;
use bevy::prelude::*;

pub(super) const FLURRY: u32 = 44614;
const DISPERSION: u32 = 47585;
const INTANGIBILITY: u32 = 288733;
const HAVOC: u32 = 80240;
const VAMPIRIC_TOUCH: u32 = 34914;
const VAMPIRIC_HORROR: u32 = 87204;
const EXTRA_INITIAL_PERIOD: u32 = 0x200;
//5740's owned ground pulses use single-selected42223 children for each enemy.
const RAIN_OF_FIRE_AREA_CHILD: u32 = 42223;

///44614s1 names three equal228596 triggers, rather than all four catalog slots.
pub(in crate::spell_cast) fn effect_enabled(
    world: &World,
    caster: Entity,
    spell: u32,
    index: u8,
) -> bool {
    spell != FLURRY
        || index == 0
        || u32::from(index) <= script_effect_value(world, caster, FLURRY, 0) as u32
}

///47585 variable872 defines total healing, including288733s2 and duration
///extensions. Preserve native initial/regular tick clocks; divide the total
///among those equal periodic ticks, not25% of maximum health on every tick.
pub(crate) fn dispersion_tick(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    index: u8,
    amount: f32,
) -> f32 {
    if spell.id != DISPERSION || index != 4 {
        return amount;
    }
    let base = spell
        .misc
        .as_ref()
        .and_then(|m| m.duration)
        .expect("Dispersion duration")
        .duration_ms as f32;
    let duration = crate::auras::spell_mods::modify(
        world,
        caster,
        spell,
        crate::auras::SpellModOp::Duration,
        base,
    );
    let period = spell
        .effects
        .iter()
        .find(|e| e.index == index)
        .expect("Dispersion health effect")
        .aura_period_ms as f32;
    let initial = u32::from(
        spell.misc.as_ref().expect("Dispersion misc").attributes[5] & EXTRA_INITIAL_PERIOD != 0,
    ) as f32;
    let ticks = (duration / period).floor() + initial;
    let additional = if crate::auras::has_aura(world, caster, INTANGIBILITY) {
        script_effect_value(world, caster, INTANGIBILITY, 1)
    } else {
        0.0
    };
    (amount + additional) * duration / base / ticks
}

///34914 explicitly names87204 as the dispeller's Horror, not the former target.
pub(crate) fn on_dispelled(
    world: &mut World,
    dispeller: Entity,
    aura: &crate::auras::AuraInstance,
) {
    if aura.spell_id == VAMPIRIC_TOUCH {
        crate::auras::apply_aura(
            world,
            &crate::auras::AuraApplication {
                caster: aura.caster,
                target: dispeller,
                spell_id: VAMPIRIC_HORROR,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
}

///80240 has native Attributes_5=32 (single-target aura); preserve foreign owners.
pub(crate) fn replace_havoc(world: &mut World, unit: Entity, aura: &crate::auras::AuraInstance) {
    let old: Vec<_> = world
        .query::<(Entity, &crate::auras::ActiveAuras)>()
        .iter(world)
        .filter(|(target, _)| *target != unit)
        .flat_map(|(target, marks)| {
            marks
                .iter()
                .filter(|mark| mark.spell_id == HAVOC && mark.caster == aura.caster)
                .map(move |mark| (target, mark.instance_id))
        })
        .collect();
    for (target, instance) in old {
        crate::auras::remove_instance(
            world,
            target,
            instance,
            crate::auras::RemoveReason::Replaced,
        );
    }
}

///80240s1 copies effective direct single-target Warlock spell damage. The
///destination still checks its own immunity/absorption/mitigation. Copies
///cannot recurse and do not roll crit or the source's modifiers again.
pub(crate) fn copy_havoc(
    world: &mut World,
    ctx: &crate::spell_cast::effects::EffectCtx,
    primary: Entity,
    effective: f32,
) {
    if !is_havoc_single_target(world, ctx.spell) {
        return;
    }
    let copies = collect_havoc_marks(world, ctx.caster, primary);
    for (target, percent) in copies {
        crate::spell_cast::effects::deal_copied_damage(
            world,
            ctx,
            target,
            effective * percent / 100.0,
        );
    }
}

fn is_havoc_single_target(world: &World, spell: &SpellInfo) -> bool {
    if spell.id == RAIN_OF_FIRE_AREA_CHILD {
        return false;
    }
    let family = spell
        .class_options
        .as_ref()
        .map(|options| options.spell_class_set);
    let havoc_family = world
        .resource::<crate::spell_info::SpellStore>()
        .get(HAVOC)
        .and_then(|spell| spell.class_options.as_ref())
        .map(|options| options.spell_class_set);
    if family.is_none() || family != havoc_family {
        return false;
    }
    let damage: Vec<_> = spell
        .effects
        .iter()
        .filter(|effect| matches!(effect.effect, 2 | 9 | 31 | 58 | 121))
        .collect();
    !damage.is_empty()
        && damage
            .iter()
            .all(|effect| effect.implicit_targets == [6, 0] && effect.chain_targets <= 1)
}

fn collect_havoc_marks(world: &mut World, caster: Entity, primary: Entity) -> Vec<(Entity, f32)> {
    world
        .query::<(Entity, &crate::auras::ActiveAuras)>()
        .iter(world)
        .filter(|(target, _)| {
            *target != primary
                && world.get::<crate::maps::InMap>(*target)
                    == world.get::<crate::maps::InMap>(caster)
        })
        .flat_map(|(target, marks)| {
            marks
                .iter()
                .filter(|mark| mark.spell_id == HAVOC && mark.caster == caster)
                .map(move |mark| {
                    (
                        target,
                        mark.effects
                            .iter()
                            .find(|effect| effect.index == 0)
                            .expect("Havoc percentage")
                            .amount,
                    )
                })
        })
        .collect()
}
