//! SimC76f8118 unique_gear_midnight.cpp5207–5217/6582: Jan'thrazet, the Soul Fang.
//! The native driver's proc metadata decides when; its item-scaled effect2 becomes
//! Soul Fang Alacrity's haste rating. The driver itself carries only dummy effects.
use bevy::prelude::*;

use super::{
    AuraApplication, AuraInstance, AuraSource,
    scripts::{AuraScript, AuraScripts},
};
use crate::spell_info::SpellStore;

const DRIVER: u32 = 1_298_085;
const SOUL_FANG_ALACRITY: u32 = 1_305_360;

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        DRIVER,
        AuraScript {
            on_proc: Some(apply_soul_fang_alacrity),
            ..Default::default()
        },
    );
}

fn apply_soul_fang_alacrity(
    world: &mut World,
    unit: Entity,
    driver: &AuraInstance,
    _other: Option<Entity>,
) {
    let haste = driver
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("native Jan'thrazet driver retains its haste effect")
        .amount;
    let mut buff = world
        .resource::<SpellStore>()
        .get(SOUL_FANG_ALACRITY)
        .expect("native Soul Fang Alacrity")
        .clone();
    buff.cast_item_level = driver.cast_item_level;
    buff.cast_item_inventory_type = driver.cast_item_inventory_type;
    buff.effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("native Soul Fang Alacrity retains its haste rating effect")
        .base_points = haste;
    super::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: SOUL_FANG_ALACRITY,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
        &buff,
    );
}
