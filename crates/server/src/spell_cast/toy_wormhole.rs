//! TC a352b1fa spell_item_wormhole_pandaria (126755): the hit unit is sent through one
//! uniformly chosen WormholeTargetLocations teleport. Each destination is the spell's
//! TDB1210 `spell_target_position` row; none is inferred.
use bevy::prelude::*;

use super::effects::EffectCtx;
use super::teleport_positions::SpellTargetPositions;
use super::toy_scripts::{has_effect_shape, single_effect};
use crate::spell_info::{SpellEffect, SpellStore};

pub(super) const WORMHOLE_PANDARIA: u32 = 126755;
/// SPELL_WORMHOLE_PANDARIA_ISLE_OF_RECKONING..WHITEPETAL_LAKE.
const PANDARIA_TELEPORTS: [u32; 9] = [
    126756, 126757, 126758, 126759, 126760, 126761, 126762, 126763, 126764,
];
const EFFECT_TELEPORT_UNITS: u32 = 252;
/// TARGET_UNIT_TARGET_ANY to TARGET_DEST_DB.
const TELEPORT_TARGETS: [u32; 2] = [25, 17];

pub(super) fn unsupported(world: &World) -> Option<String> {
    let store = world.resource::<SpellStore>();
    let positions = world.get_resource::<SpellTargetPositions>();
    let audited = PANDARIA_TELEPORTS.iter().all(|id| {
        let authored = positions.is_some_and(|positions| positions.0.contains_key(&(*id, 0)));
        let teleport = store
            .get(*id)
            .and_then(single_effect)
            .is_some_and(|effect| {
                has_effect_shape(effect, EFFECT_TELEPORT_UNITS, TELEPORT_TARGETS, 0)
            });
        authored && teleport
    });
    (!audited).then(|| "wormhole source teleports or destinations differ".into())
}

pub(super) fn teleport_to_random_pandaria_point(
    world: &mut World,
    ctx: &EffectCtx,
    _effect: &SpellEffect,
) {
    let choice = rand::Rng::gen_range(&mut rand::thread_rng(), 0..PANDARIA_TELEPORTS.len());
    let teleport = PANDARIA_TELEPORTS[choice];
    super::cast_triggered(world, ctx.caster, teleport, Some(ctx.caster), ctx.depth + 1);
}
