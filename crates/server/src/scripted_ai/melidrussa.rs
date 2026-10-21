//! Current Retail LittleWigs2777a9d Melidrussa.lua + native70448 child requests.
//! MELIDRUSSA_TIMELINE_CONDITION_CONTEXT: the lead explicitly approves ongoing
//! source-order repeat lanes, without claiming executable Retail timeline context.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx, EventMap};

pub(super) const SCRIPT: &str = "boss_melidrussa_chillworn";
const HAILBURST: u32 = 396044;
const CHILLSTORM: u32 = 1311543;
const NORMAL_HAILBURST_REPEATS: &[u64] = &[24_000, 22_000, 25_000];
const NORMAL_CHILLSTORM_REPEATS: &[u64] = &[24_000, 32_000, 25_000];
const MYTHIC_REPEATS: &[u64] = &[24_000];
const MYTHIC_KEYSTONE: u32 = 8;
const MYTHIC_DUNGEON: u32 = 23;

pub(super) fn creature_ai(boss: BossAi) -> Box<dyn CreatureAi> {
    Box::new(Melidrussa {
        boss,
        hailburst_step: 0,
        chillstorm_step: 0,
        hailburst_repeats: NORMAL_HAILBURST_REPEATS,
        chillstorm_repeats: NORMAL_CHILLSTORM_REPEATS,
    })
}

struct Melidrussa {
    boss: BossAi,
    hailburst_step: usize,
    chillstorm_step: usize,
    hailburst_repeats: &'static [u64],
    chillstorm_repeats: &'static [u64],
}

fn repeat_source_order(events: &mut EventMap, step: &mut usize, repeats: &[u64]) {
    events.repeat(repeats[*step]);
    *step = (*step + 1) % repeats.len();
}

impl CreatureAi for Melidrussa {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        let map = ctx.map().expect("Melidrussa instance");
        let difficulty = crate::zones::map_difficulty_id(ctx.world, map);
        let mythic = matches!(difficulty, MYTHIC_KEYSTONE | MYTHIC_DUNGEON);
        (self.hailburst_repeats, self.chillstorm_repeats) = if mythic {
            (MYTHIC_REPEATS, MYTHIC_REPEATS)
        } else {
            (NORMAL_HAILBURST_REPEATS, NORMAL_CHILLSTORM_REPEATS)
        };
        self.hailburst_step = 0;
        self.chillstorm_step = 0;
        self.boss.events.schedule(HAILBURST, 5_000);
        self.boss.events.schedule(CHILLSTORM, 15_000);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Melidrussa {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            HAILBURST => {
                // Native1307297 Dummy3 names396044; retain the child's ordinary
                // 3s cast and actual area-enemy damage, not instant wrapper casts.
                ctx.do_cast_self(HAILBURST);
                repeat_source_order(
                    &mut self.boss.events,
                    &mut self.hailburst_step,
                    self.hailburst_repeats,
                );
            }
            CHILLSTORM => {
                // Native1307308 names1311543; its implicit22/15 targets remain
                // authoritative. Missing downstream selector scripts are gaps.
                ctx.do_cast_self(CHILLSTORM);
                repeat_source_order(
                    &mut self.boss.events,
                    &mut self.chillstorm_step,
                    self.chillstorm_repeats,
                );
            }
            _ => {}
        }
    }
}
