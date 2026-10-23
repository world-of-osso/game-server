//! LittleWigs2777a9d lanes; native70448 casts/cones/auras remain authoritative.
//! TREEMOUTH_MINIMUM_ALERT_CONTEXT uses only source lower-bound lane holds.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_treemouth";
const VINE_WHIP: u32 = 377559;
const DECAY_SPRAY: u32 = 376811;
const INFECTIOUS_SPIT: u32 = 377859;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    if map.map_id != 2520 || !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205) {
        return None;
    }
    let boss = super::instance_script::bound_boss(world, actor)?;
    Some(Box::new(Treemouth { boss }))
}

struct Treemouth {
    boss: BossAi,
}

impl CreatureAi for Treemouth {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(VINE_WHIP, 5_000);
        self.boss.events.schedule(DECAY_SPRAY, 12_100);
        // This local partial covers Normal1/Follower205, both non-Mythic in Lua.
        self.boss.events.schedule(INFECTIOUS_SPIT, 20_900);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Treemouth {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let (repeat, hold, other_lanes) = match event {
            VINE_WHIP => (15_800, 4_850, [DECAY_SPRAY, INFECTIOUS_SPIT]),
            DECAY_SPRAY => (42_500, 8_300, [VINE_WHIP, INFECTIOUS_SPIT]),
            INFECTIOUS_SPIT => (20_600, 3_630, [VINE_WHIP, DECAY_SPRAY]),
            _ => return,
        };
        ctx.do_cast_victim(event);
        self.boss.events.repeat(repeat);
        // Lua only moves bars already due inside the stated minimum context.
        for lane in other_lanes {
            if self
                .boss
                .events
                .remaining_ms(lane)
                .is_some_and(|left| left < hold)
            {
                self.boss.events.reschedule(lane, hold);
            }
        }
    }
}
