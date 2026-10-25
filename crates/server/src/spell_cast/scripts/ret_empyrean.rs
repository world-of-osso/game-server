//! SIMC_RET_EMPYREAN_LEGACY_RELATIONSHIP.
//! SimC76f8118 sc_paladin.cpp311–312; retribution.cpp713–716,1083–1088.
//! TC has no handler/TDB proc row; native387170 effect1 owns the percentage.
use bevy::prelude::*;

use crate::auras::{self, RemoveReason};
use crate::class_progression::TraitLoadout;
use crate::spell_info::{SpellInfo, SpellStore};

const LEGACY_TALENT: u32 = 387170;
const LEGACY_BUFF: u32 = 387178;
const DIVINE_STORM: u32 = 53385;

fn selected(world: &World, caster: Entity) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == LEGACY_TALENT)
    })
}

/// The script, not metadata-only every-cast/any-melee predicates, owns this
/// driver and its buff's consumption. Other aura proc dispatch is unchanged.
pub(crate) fn owns_empyrean_proc(spell: u32) -> bool {
    matches!(spell, LEGACY_TALENT | LEGACY_BUFF)
}

pub(super) fn grant_on_wrath(world: &mut World, caster: Entity, depth: u8) {
    if selected(world, caster) {
        crate::spell_cast::cast_triggered(world, caster, LEGACY_BUFF, Some(caster), depth + 1);
    }
}

pub(in crate::spell_cast) fn schedule_empyrean_legacy(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    target: Option<Entity>,
) {
    if !matches!(spell.id, 85256 | 383328)
        || !selected(world, caster)
        || !auras::has_aura(world, caster, LEGACY_BUFF)
    {
        return;
    }
    let Some(target) = target else {
        warn!(
            ?caster,
            "RET_EMPYREAN_PRIMARY_TARGET_UNAVAILABLE: no native Verdict recipient"
        );
        return;
    };
    let percent = super::script_effect_value(world, caster, LEGACY_TALENT, 1);
    let multiplier = 1.0 + percent / 100.0;
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == LEGACY_BUFF
    });
    // schedule_execute() has no authored delay. Defer behind the foreground
    // action rather than introducing a timer or running a paid second spender.
    world.commands().queue(move |world: &mut World| {
        if world.get_entity(caster).is_ok() && world.get_entity(target).is_ok() {
            cast_background_storm(world, caster, target, multiplier);
        }
    });
}

fn cast_background_storm(world: &mut World, caster: Entity, target: Entity, multiplier: f32) {
    let Some(mut storm) = world.resource::<SpellStore>().get(DIVINE_STORM).cloned() else {
        warn!(
            ?caster,
            "native Empyrean Legacy Divine Storm53385 is unavailable"
        );
        return;
    };
    storm.cast_damage_multiplier = Some(multiplier);
    if crate::spell_cast::launch_triggered_spell(world, caster, &storm, Some(target), 1) {
        super::ret_tier::after_background_storm(world, caster, target);
        // SimC76f8118 Ret573–590/609–625: Legacy constructs the same
        // Second Sunrise producer as foreground Storm; echoes do not recurse.
        super::apply_echo_policies(world, caster, &storm);
    } else {
        warn!(
            ?caster,
            ?target,
            "native Empyrean Legacy background Storm refused"
        );
    }
}
