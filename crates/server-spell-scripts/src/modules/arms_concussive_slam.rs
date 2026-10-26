//! MID2 Arms 2pc launches native Concussive Slam on secondary recipients only.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterElementalLaunch(cast_after_slam),
    SpellScriptHook::TargetFilter(exclude_primary),
];

const TWO_PIECE: u32 = 1296643;
const CONCUSSIVE_SLAM: u32 = 1300660;

fn equipped(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 71)
        && combat::auras::has_aura(world, caster, TWO_PIECE)
}

fn cast_after_slam(world: &mut World, caster: Entity, spell: &SpellInfo, _: bool) {
    // SimC76f8118 sc_warrior.cpp3534–3535/3551–3562: slam_base_t
    // launches once per execution, including inherited Heroic/background Slam.
    if !matches!(spell.id, 1464 | 1269383) || !equipped(world, caster) {
        return;
    }
    let Some(primary) = spell.cast_primary_target else {
        warn!(?caster, "Native Concussive Slam lacks its source primary");
        return;
    };
    if !combat::spell_cast::try_cast_triggered(world, caster, CONCUSSIVE_SLAM, Some(primary), 1) {
        warn!(?caster, ?primary, "Native Concussive Slam payload refused");
    }
}

fn exclude_primary(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    index: u8,
    primary: Option<Entity>,
    recipients: &mut Vec<Entity>,
) {
    // sc_warrior.cpp3443–3453: secondary_targets_only(). Native payload
    // selection still supplies its radius and admitted enemy recipients.
    if spell.id == CONCUSSIVE_SLAM && index == 0 && equipped(world, caster) {
        recipients.retain(|target| Some(*target) != primary);
    }
}
