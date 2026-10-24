//! SIMC_RET_DAWNLIGHT_WAKE_PRODUCER: pinned SimC76f8118 Ret846–876 supplies
//! Wake→431522. Native modified count wins: raw2 + Ret137027/effect25 +1 =3,
//! unlike SimC's raw base_value() read. No consumer, timer or metadata override.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use crate::class_progression::TraitLoadout;
use crate::spell_info::{SpellInfo, SpellStore};

pub(super) const WAKE: u32 = 255937;
const DAWNLIGHT: u32 = 431377;
const BUDGET: u32 = 431522;
const RETRIBUTION: u32 = 70;

fn selected(world: &World, caster: Entity) -> bool {
    let is_ret = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == RETRIBUTION);
    let has_talent = world.get::<TraitLoadout>(caster).is_some_and(|talents| {
        talents
            .active_talent_spells()
            .any(|spell| spell.spell_id == DAWNLIGHT)
    });
    is_ret && has_talent
}

pub(super) fn grant_budget(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if !selected(world, caster) {
        return;
    }
    let count = super::script_effect_value(world, caster, DAWNLIGHT, 0);
    if !count.is_finite() || !(1.0..=f32::from(u8::MAX)).contains(&count) {
        warn!(?caster, count, "Dawnlight native count invalid");
        return;
    }
    let Some(mut buff) = world.resource::<SpellStore>().get(BUDGET).cloned() else {
        warn!(spell_id = BUDGET, "Dawnlight native budget unavailable");
        return;
    };
    buff.cast_aura_stacks = Some(count as u8);
    if !super::super::launch_triggered_spell(world, caster, &buff, Some(caster), 1) {
        warn!(
            ?caster,
            spell_id = BUDGET,
            "Dawnlight native budget refused"
        );
    }
}
