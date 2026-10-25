//! LittleWigs2777a9d non-Mythic lanes; native70448 owns casts/targets/effects.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_khajin";
const FROST_SHOCK: u32 = 385963;
const HAILSTORM: u32 = 386757;
const GLACIAL_SURGE: u32 = 386559;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    if map.map_id != 2527 || !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205) {
        return None;
    }
    let boss = super::instance_script::bound_boss(world, actor)?;
    Some(Box::new(Khajin {
        boss,
        shock_repeat: 0,
        hail_repeat: 0,
        surge_repeat: 0,
    }))
}

struct Khajin {
    boss: BossAi,
    shock_repeat: usize,
    hail_repeat: usize,
    surge_repeat: usize,
}

impl CreatureAi for Khajin {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.shock_repeat = 0;
        self.hail_repeat = 0;
        self.surge_repeat = 0;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.shock_repeat = 0;
        self.hail_repeat = 0;
        self.surge_repeat = 0;
        self.boss.events.schedule(FROST_SHOCK, 6_000);
        self.boss.events.schedule(HAILSTORM, 10_000);
        self.boss.events.schedule(GLACIAL_SURGE, 22_000);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Khajin {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let repeat = match event {
            FROST_SHOCK => {
                ctx.do_cast_victim(event);
                let repeat = [10_000, 27_000, 18_000][self.shock_repeat];
                self.shock_repeat = (self.shock_repeat + 1) % 3;
                repeat
            }
            HAILSTORM => {
                ctx.do_cast_aoe(event);
                let repeat = [27_000, 28_000][self.hail_repeat];
                self.hail_repeat = (self.hail_repeat + 1) % 2;
                repeat
            }
            GLACIAL_SURGE => {
                ctx.do_cast_self(event);
                let repeat = [22_000, 33_000][self.surge_repeat];
                self.surge_repeat = (self.surge_repeat + 1) % 2;
                repeat
            }
            _ => return,
        };
        self.boss.events.repeat(repeat);
    }
}
