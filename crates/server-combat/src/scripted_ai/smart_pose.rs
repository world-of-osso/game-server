//! TrinityCore a352b1fa SmartScript.cpp: SET_ORIENTATION and SET_EMOTE_STATE.
use super::*;
use shared::components::{Npc, Player, Rotation, UnitPose};

fn requested_orientation(ctx: &Ctx, event: &SmartEvent, targets: &[Entity]) -> Option<f32> {
    match event.target_type {
        TARGET_SELF => ctx
            .world
            .get::<crate::creature_ai::HomeOrientation>(ctx.me)
            .map(|home| home.0),
        TARGET_POSITION => Some(event.target_position[3] + std::f32::consts::FRAC_PI_2),
        _ => {
            let target = targets.first()?;
            let from = ctx.position()?;
            let to = ctx.world.get::<shared::components::Position>(*target)?;
            crate::creature_ai::facing_along(to.x - from.x, to.z - from.z)
        }
    }
}

fn mark_pose_changed(world: &World, entity: Entity) {
    if let Some(queue) = world.get_resource::<lightyear::prelude::ReplicationDirtyQueue>() {
        queue.mark(entity);
    }
}

impl SmartAi {
    pub(super) fn set_orientation(ctx: &mut Ctx, event: &SmartEvent, targets: &[Entity]) {
        let Some(facing) = requested_orientation(ctx, event, targets) else {
            return;
        };
        if let Some(mut rotation) = ctx.world.get_mut::<Rotation>(ctx.me)
            && rotation.y != facing
        {
            rotation.y = facing;
            mark_pose_changed(ctx.world, ctx.me);
        }
    }

    pub(super) fn set_emote_state(ctx: &mut Ctx, targets: &[Entity], emote: u32) {
        for &target in targets {
            if ctx.world.get::<Npc>(target).is_none() && ctx.world.get::<Player>(target).is_none() {
                continue;
            }
            let Some(mut pose) = ctx.world.get_mut::<UnitPose>(target) else {
                continue;
            };
            if pose.emote_state != emote {
                pose.emote_state = emote;
                mark_pose_changed(ctx.world, target);
            }
        }
    }
}
