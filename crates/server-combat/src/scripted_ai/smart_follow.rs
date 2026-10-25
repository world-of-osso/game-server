//! TrinityCore a352b1fa FOLLOW: relative distance/angle, cancellation, arrival hook.
use super::*;
use shared::components::{Position, Rotation};

const FOLLOW_POINT_ID: u32 = u32::MAX;
const FOLLOW_ARRIVAL_RADIUS: f32 = 5.0;

pub(super) struct Follow {
    target: Entity,
    distance: f32,
    angle: f32,
    arrived_entry: u32,
    check_ms: u32,
}

impl SmartAi {
    pub(super) fn set_follow(&mut self, ctx: &mut Ctx, event: &SmartEvent, targets: &[Entity]) {
        ctx.world
            .entity_mut(ctx.me)
            .remove::<super::super::movement::PointMovement>();
        let Some(&target) = targets.first() else {
            self.follow = None;
            return;
        };
        let angle = event.action_params[1] as f32;
        self.follow = Some(Follow {
            target,
            distance: event.action_params[0] as f32 + 0.1,
            angle: if angle > 6.0 {
                angle.to_radians()
            } else {
                angle
            },
            arrived_entry: event.action_params[3],
            check_ms: 1000,
        });
        self.update_follow(ctx, 0);
    }

    pub(super) fn update_follow(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        let Some(follow) = &mut self.follow else {
            return;
        };
        if ctx.is_engaged() {
            ctx.world
                .entity_mut(ctx.me)
                .remove::<super::super::movement::PointMovement>();
            return;
        }
        if follow.check_ms < diff_ms {
            follow.check_ms = 1000;
            if follow.arrived_entry != 0
                && super::targets::nearest_creature(
                    ctx.world,
                    ctx.me,
                    follow.arrived_entry,
                    FOLLOW_ARRIVAL_RADIUS,
                    0,
                )
                .is_some()
            {
                self.complete_follow(ctx);
                return;
            }
        } else {
            follow.check_ms -= diff_ms;
        }
        let Some(position) = ctx.world.get::<Position>(follow.target).copied() else {
            return;
        };
        let orientation = ctx
            .world
            .get::<Rotation>(follow.target)
            .map_or(0.0, |r| r.y - std::f32::consts::FRAC_PI_2);
        let angle = orientation + follow.angle;
        let destination = Position {
            x: position.x + follow.distance * angle.cos(),
            y: position.y,
            z: position.z - follow.distance * angle.sin(),
        };
        ctx.move_point(ctx.me, FOLLOW_POINT_ID, destination);
    }

    fn complete_follow(&mut self, ctx: &mut Ctx) {
        self.follow = None;
        ctx.world
            .entity_mut(ctx.me)
            .remove::<super::super::movement::PointMovement>();
        // Pinned StopFollow clears the GUID before player lookup, so credit and
        // FOLLOW_COMPLETED invoker are absent there; do not invent player credit.
        super::lifetimes::schedule_despawn(ctx.world, ctx.me, 5000, 0, true);
        self.process_events_for(ctx, EVENT_FOLLOW_COMPLETED, None, 0);
    }
}
