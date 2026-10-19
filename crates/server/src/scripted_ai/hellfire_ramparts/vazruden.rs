//! TC a352b1fa boss_vazruden_the_herald.cpp434–450: appearance/engagement.
//! Revenge's ordinary admission remains intact; death/descent/paired credit are excluded.
use bevy::prelude::*;

use super::super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_vazruden";
const SAY_AGGRO: u32 = 13;
const REVENGE: u32 = 14;

pub(super) fn creature_ai() -> Box<dyn CreatureAi> {
    Box::new(Vazruden {
        boss: BossAi::new(1),
    })
}

struct Vazruden {
    boss: BossAi,
}

impl CreatureAi for Vazruden {
    fn just_appeared(&mut self, ctx: &mut Ctx) {
        ctx.do_cast_self(5301);
        ctx.do_zone_in_combat(ctx.me);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(SAY_AGGRO, 5_000);
        self.boss.events.schedule_range(REVENGE, 5_000, 10_000);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Vazruden {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            SAY_AGGRO => ctx.talk(1),
            REVENGE => {
                ctx.do_cast_victim(super::herald::difficulty_spell(ctx, 19_130, 40_392));
                self.boss.events.repeat_range(5_000, 10_000);
            }
            _ => unreachable!("unknown Vazruden engagement event{event}"),
        }
    }
}
