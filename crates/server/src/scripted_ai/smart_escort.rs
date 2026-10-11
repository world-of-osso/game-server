//! TrinityCore a352b1fa SmartAI::EndPath and waypoint table arrival delays.
use super::*;

fn distance_between(from: shared::components::Position, to: shared::components::Position) -> f32 {
    Vec3::new(from.x - to.x, from.y - to.y, from.z - to.z).length()
}

fn eligible_for_escort_credit(ctx: &Ctx, player: Entity) -> bool {
    let same_map =
        ctx.world.get::<crate::maps::InMap>(ctx.me) == ctx.world.get::<crate::maps::InMap>(player);
    let alive = ctx
        .world
        .get::<crate::death::PlayerDeathState>(player)
        .is_none_or(|death| death.state == shared::death::DeathState::Alive);
    let near = ctx
        .position()
        .zip(
            ctx.world
                .get::<shared::components::Position>(player)
                .copied(),
        )
        .is_some_and(|(me, target)| distance_between(me, target) <= 100.0);
    same_map && alive && near
}

impl SmartAi {
    pub(super) fn fail_out_of_range_escort(&mut self, ctx: &mut Ctx, diff_ms: u32) -> bool {
        let Some(escort) = &mut self.escort else {
            return false;
        };
        if escort.range_check_ms >= diff_ms {
            escort.range_check_ms -= diff_ms;
            return false;
        }
        escort.range_check_ms = 1000;
        if escort.targets.is_empty() {
            return false;
        }
        let targets = if escort.targets.len() == 1 {
            super::targets::party_targets(ctx.world, escort.targets[0])
        } else {
            escort.targets.clone()
        };
        let range = if ctx
            .world
            .get::<crate::maps::InMap>(ctx.me)
            .is_some_and(|map| map.instance_id != 0)
        {
            120.0
        } else {
            60.0
        };
        let same_map = |player| {
            ctx.world.get::<crate::maps::InMap>(ctx.me)
                == ctx.world.get::<crate::maps::InMap>(player)
        };
        let in_range = targets.iter().any(|&player| {
            same_map(player)
                && ctx
                    .position()
                    .zip(
                        ctx.world
                            .get::<shared::components::Position>(player)
                            .copied(),
                    )
                    .is_some_and(|(me, target)| distance_between(me, target) <= range)
        });
        if in_range {
            return false;
        }
        bevy::log::debug!(creature = ?ctx.me, path = escort.path_id, quest = escort.quest_id, ?targets, ?range, position = ?ctx.position(), driver = "escort_range", "escort failed");
        self.stop_path(ctx, true);
        self.process_events_for(ctx, EVENT_DEATH, Some(ctx.me), 0);
        super::lifetimes::despawn_creature(ctx.world, ctx.me, 0);
        true
    }

    pub(super) fn finish_path(&mut self, ctx: &mut Ctx, failed: bool) {
        let Some(mut escort) = self.escort.take() else {
            return;
        };
        ctx.world.entity_mut(ctx.me).remove::<(
            super::super::movement::PointMovement,
            super::super::movement::ScriptedPath,
        )>();
        if escort.npc_flags != 0 {
            if let Some(mut flags) = ctx.world.get_mut::<shared::protocol::NpcFlags>(ctx.me) {
                flags.0 = escort.npc_flags;
            }
        }
        let targets = if escort.targets.len() == 1 {
            super::targets::party_targets(ctx.world, escort.targets[0])
        } else {
            escort.targets.clone()
        };
        for player in targets {
            if escort.quest_id != 0 && (failed || eligible_for_escort_credit(ctx, player)) {
                crate::quest::persist_scripted_quest_result(
                    ctx.world,
                    player,
                    escort.quest_id,
                    failed,
                );
            }
        }
        if failed {
            return;
        }
        let last = escort.path.nodes[escort.index].id;
        self.path_event_id = Some(escort.path_id);
        self.process_events_for(ctx, EVENT_WAYPOINT_ENDED, None, last);
        self.path_event_id = None;
        if escort.despawn_ms > 0 {
            super::lifetimes::schedule_despawn(ctx.world, ctx.me, escort.despawn_ms, 0, true);
        }
        if self.escort.is_none() && escort.repeat {
            escort.index = 0;
            escort.reached = false;
            escort.pause_ms = None;
            escort.node_delay_ms = None;
            self.escort = Some(escort);
            super::super::movement::retain_path_home(ctx.world, ctx.me);
            self.move_to_waypoint(ctx);
        }
    }

    pub(super) fn update_node_delay(&mut self, ctx: &mut Ctx, diff_ms: u32) -> bool {
        let Some(escort) = &mut self.escort else {
            return false;
        };
        if escort.pause_ms.is_some() {
            return false;
        }
        let Some(timer) = escort.node_delay_ms else {
            return false;
        };
        if ctx.is_engaged() {
            return true;
        }
        if timer > diff_ms {
            escort.node_delay_ms = Some(timer - diff_ms);
        } else {
            escort.node_delay_ms = None;
            self.advance_waypoint(ctx);
        }
        true
    }
}
