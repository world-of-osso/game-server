//! TC a352b1fa SmartScript.cpp ATTACK_START49 and SET_IMMUNE_PC144/NPC145.
use super::*;
use crate::random as rand;
use rand::seq::SliceRandom;
use shared::components::{Health, UnitFlags};

pub(super) fn attack_random_target(ctx: &mut Ctx, targets: &[Entity]) {
    let units: Vec<_> = targets
        .iter()
        .copied()
        .filter(|&target| ctx.world.get::<Health>(target).is_some())
        .collect();
    if let Some(&target) = units.choose(&mut rand::thread_rng()) {
        crate::creature_aggro::attack_start(ctx.world, ctx.me, target);
    }
}

pub(super) fn set_immunity(ctx: &mut Ctx, targets: &[Entity], mask: u32, immune: bool) {
    for &target in targets {
        if !immune {
            crate::creature_aggro::clear_unit_flags(ctx.world, target, mask);
            continue;
        }
        let Some(mut flags) = ctx.world.get_mut::<UnitFlags>(target) else {
            continue;
        };
        let previous = flags.0;
        flags.0 |= mask;
        if flags.0 != previous {
            if let Some(queue) = ctx
                .world
                .get_resource::<lightyear::prelude::ReplicationDirtyQueue>()
            {
                queue.mark(target);
            }
        }
    }
}
