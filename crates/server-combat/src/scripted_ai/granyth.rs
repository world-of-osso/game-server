//! LittleWigs2777a9d Granyth: finite pre-Lance lanes; native70448 effects.
//! No fabricated100 Energy, Eruption controller, Lance reset or Mythic adds.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_granyth";
const SHARDS_OF_STONE: u32 = 388817;
const TECTONIC_STOMP: u32 = 385916;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    if world.get::<crate::maps::InMap>(actor)?.map_id != 2516 {
        return None;
    }
    let boss = super::instance_script::bound_boss(world, actor)?;
    Some(Box::new(Granyth {
        boss,
        shards_remaining: 0,
    }))
}

struct Granyth {
    boss: BossAi,
    shards_remaining: u8,
}

impl CreatureAi for Granyth {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.shards_remaining = 0;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.shards_remaining = 2;
        self.boss.events.schedule(SHARDS_OF_STONE, 10_600);
        self.boss.events.schedule(TECTONIC_STOMP, 15_500);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Granyth {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            SHARDS_OF_STONE => {
                ctx.do_cast_self(SHARDS_OF_STONE);
                self.shards_remaining -= 1;
                if self.shards_remaining > 0 {
                    self.boss.events.repeat(13_300);
                }
            }
            TECTONIC_STOMP => ctx.do_cast_self(TECTONIC_STOMP),
            _ => {}
        }
    }
}
