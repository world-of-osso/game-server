//! SimC76f8118 unique_gear_midnight.cpp288–366: one immediate Gleam, then native periods.
use crate::auras::{
    AuraInstance,
    scripts::{AuraScript, AuraScripts},
};
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;

pub(crate) const ROOT: u32 = 1295132;

pub(crate) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        ROOT,
        AuraScript {
            on_apply: Some(apply_gleam),
            ..Default::default()
        },
    );
}

fn apply_gleam(world: &mut World, actor: Entity, aura: &AuraInstance) {
    let remaining =
        aura.expires_at.expect("native Liquid Luster duration") - crate::spell_cast::now(world);
    // The final native tick must not create a zero-duration permanent Gleam.
    if remaining <= 0.0 {
        return;
    }
    let gleam = read_gleam_payload(world, aura, remaining);
    super::apply_spell(world, actor, &gleam);
}

fn read_gleam_payload(world: &World, aura: &AuraInstance, remaining: f64) -> SpellInfo {
    let driver = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Liquid Luster periodic effect");
    let mut gleam = world
        .resource::<SpellStore>()
        .get(driver.trigger_spell)
        .expect("native Lustrous Gleam payload")
        .clone();
    let rating = gleam
        .effects
        .iter_mut()
        .find(|effect| effect.aura == 189)
        .expect("native Gleam rating effect");
    // The driver's native, item-scaled amount is the per-stack rating, not the
    // triggered payload's zero base points. Native max stacks remain unchanged.
    rating.base_points = driver.amount;
    let duration = gleam
        .misc
        .as_mut()
        .and_then(|misc| misc.duration.as_mut())
        .expect("native Gleam duration");
    duration.duration_ms = (remaining * 1000.0).ceil() as i32;
    gleam
}

pub(crate) fn tick(world: &mut World, actor: Entity, aura: &AuraInstance, _index: u8) {
    apply_gleam(world, actor, aura);
}
