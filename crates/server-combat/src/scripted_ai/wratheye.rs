//! LittleWigs2777a9d DecatriarchWratheye.lua Decaystrike-only partial.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_wratheye";
const DECAYSTRIKE: u32 = 373912;
const FIRST_STRIKE_MS: u64 = 10_600;
const SHORT_REPEAT_MS: u64 = 19_400;
const LONG_REPEAT_MS: u64 = 25_500;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    if map.map_id != 2520 || !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205) {
        return None;
    }
    let boss = super::instance_script::bound_boss(world, actor)?;
    Some(Box::new(Wratheye {
        boss,
        next_repeat_ms: SHORT_REPEAT_MS,
    }))
}

struct Wratheye {
    boss: BossAi,
    next_repeat_ms: u64,
}

impl CreatureAi for Wratheye {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.next_repeat_ms = SHORT_REPEAT_MS;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.next_repeat_ms = SHORT_REPEAT_MS;
        self.boss.events.schedule(DECAYSTRIKE, FIRST_STRIKE_MS);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Wratheye {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let victim = ctx.victim();
        if event != DECAYSTRIKE || !ctx.do_cast_at(victim, event, false) {
            return;
        }
        // Lua's SPELL_CAST_START arms 19.4/25.5 alternation, not success/end.
        self.boss.events.schedule(event, self.next_repeat_ms);
        self.next_repeat_ms = if self.next_repeat_ms == SHORT_REPEAT_MS {
            LONG_REPEAT_MS
        } else {
            SHORT_REPEAT_MS
        };
    }
}
