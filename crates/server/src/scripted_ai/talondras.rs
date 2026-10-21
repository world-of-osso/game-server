//! Current LittleWigs2777a9d SentinelTalondras.lua + native70448 partial.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_sentinel_talondras";
const SHARDS: u32 = 372718;
const STOMP: u32 = 372701;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    if world.get::<crate::maps::InMap>(actor)?.map_id != 2451 {
        return None;
    }
    Some(Box::new(Talondras {
        boss: super::instance_script::bound_boss(world, actor)?,
    }))
}

struct Talondras {
    boss: BossAi,
}

impl CreatureAi for Talondras {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        // Healer-only Lua bars filter alerts, not native spell targets. The
        // role/stun/energy context remains named; no Orb first time is invented.
        // TALONDRAS_LUA_DB2_CATEGORY_1152_CONFLICT preserves ordinary native
        // rejections where these Lua clocks collide with shared category3s/5s.
        self.boss.events.schedule(SHARDS, 4_500);
        self.boss.events.schedule(STOMP, 5_100);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Talondras {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            SHARDS => {
                ctx.do_cast_victim(SHARDS);
                self.boss.events.repeat(9_700);
            }
            STOMP => {
                ctx.do_cast_self(STOMP);
                self.boss.events.repeat(12_100);
            }
            _ => {}
        }
    }
}
