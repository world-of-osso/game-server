//! SimC76f8118 unique_gear_midnight.cpp5600–5614/6600: armor stat procs.
//! Native equipped drivers own eligibility, rates, item scaling and both signed values.
use bevy::prelude::*;

use super::{
    AuraApplication, AuraInstance, AuraSource,
    scripts::{AuraScript, AuraScripts},
};
use crate::spell_info::{SpellInfo, SpellStore};

const DRIVERS: &[u32] = &[1307906, 1307923, 1307928];

pub(super) fn register(scripts: &mut AuraScripts) {
    for &id in DRIVERS {
        scripts.0.insert(
            id,
            AuraScript {
                on_proc: Some(apply_stat_proc),
                ..Default::default()
            },
        );
    }
}

fn build_stat_buff(world: &World, driver: &AuraInstance) -> SpellInfo {
    let positive = driver
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Venomcursed driver retains its positive stat effect");
    let mut buff = world
        .resource::<SpellStore>()
        .get(positive.trigger_spell)
        .expect("native Venomcursed stat buff")
        .clone();
    buff.cast_item_level = driver.cast_item_level;
    for index in 0..2 {
        let amount = driver
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("native Venomcursed driver retains both signed stat amounts")
            .amount;
        buff.effects
            .iter_mut()
            .find(|effect| effect.index == index)
            .expect("native Venomcursed buff retains both rating masks")
            .base_points = amount;
    }
    buff
}

fn apply_stat_proc(world: &mut World, unit: Entity, driver: &AuraInstance, _other: Option<Entity>) {
    let buff = build_stat_buff(world, driver);
    super::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: buff.id,
            effect_mask: 3,
            source: AuraSource::Cast,
        },
        &buff,
    );
}
