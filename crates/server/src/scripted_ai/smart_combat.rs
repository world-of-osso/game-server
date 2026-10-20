//! TC a352b1fa SmartScript.cpp ATTACK_START49 and SET_IMMUNE_NPC145.
use super::*;
use crate::random as rand;
use rand::seq::SliceRandom;
use shared::components::{Health, UnitFlags};

const UNIT_FLAG_IMMUNE_TO_NPC: u32 = 0x200;

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

pub(super) fn set_npc_immunity(ctx: &mut Ctx, targets: &[Entity], immune: bool) {
    for &target in targets {
        let Some(mut flags) = ctx.world.get_mut::<UnitFlags>(target) else {
            continue;
        };
        let previous = flags.0;
        if immune {
            flags.0 |= UNIT_FLAG_IMMUNE_TO_NPC;
        } else {
            flags.0 &= !UNIT_FLAG_IMMUNE_TO_NPC;
        }
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
