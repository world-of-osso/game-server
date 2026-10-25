//! SIMC_RET_DIVINE_RESONANCE_PRODUCER, producer only.
//! SimC76f8118 sc_paladin.cpp1901–1907 (within1880–1905),3686–3688:
//! grant after base execute/cast-success callbacks, initialized at native max.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use crate::class_progression::TraitLoadout;
use crate::spell_cast::{self, TriggeredSpellOverrides};
use crate::spell_info::{SpellInfo, SpellStore};

pub(super) const TOLL: u32 = 375576;
const TALENT: u32 = 384027;
const BUFF: u32 = 1266308;
const RETRIBUTION: u32 = 70;

fn selected(world: &World, caster: Entity) -> bool {
    let is_ret = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == RETRIBUTION);
    let selected = world.get::<TraitLoadout>(caster).is_some_and(|talents| {
        talents
            .active_talent_spells()
            .any(|spell| spell.spell_id == TALENT)
    });
    is_ret && selected
}

fn native_maximum(world: &World) -> Option<u8> {
    let spell = world.resource::<SpellStore>().get(BUFF)?;
    let maximum = spell.aura_options.as_deref()?.cumulative_aura;
    u8::try_from(maximum).ok().filter(|maximum| *maximum > 0)
}

pub(super) fn grant_after_toll(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if !selected(world, caster) {
        return;
    }
    let Some(maximum) = native_maximum(world) else {
        warn!(
            spell_id = BUFF,
            "Divine Resonance native aura/maximum unavailable"
        );
        return;
    };
    let depth = spell_cast::next_trigger_depth(world);
    let overrides = TriggeredSpellOverrides {
        aura_stacks: Some(maximum),
        ..Default::default()
    };
    if !spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        BUFF,
        Some(caster),
        depth,
        overrides,
    ) {
        debug!(
            spell_id = BUFF,
            "Divine Resonance native aura dispatch refused"
        );
    }
}
