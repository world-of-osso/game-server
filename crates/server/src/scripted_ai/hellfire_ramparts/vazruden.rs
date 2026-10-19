//! TC a352b1fa boss_vazruden_the_herald.cpp434–450: appearance/engagement.
//! Ordinary below40 Call Nazan is latched once; death/paired credit are excluded.
use bevy::prelude::*;

use super::super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::super::{CreatureAi, Ctx, DamageTaken};

pub(super) const SCRIPT: &str = "boss_vazruden";
const SAY_AGGRO: u32 = 13;
const REVENGE: u32 = 14;
const CALL_NAZAN: u32 = 15;

pub(super) fn creature_ai() -> Box<dyn CreatureAi> {
    Box::new(Vazruden {
        boss: BossAi::new(1),
        has_called_nazan: false,
    })
}

struct Vazruden {
    boss: BossAi,
    has_called_nazan: bool,
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

    // TC452–459: DamageTaken schedules once, without forcing cast admission.
    fn damage_taken(&mut self, _ctx: &mut Ctx, _attacker: Option<Entity>, damage: DamageTaken) {
        if !self.has_called_nazan && damage.health_below_pct_damaged(40.0) {
            self.has_called_nazan = true;
            self.boss.events.schedule(CALL_NAZAN, 0);
        }
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
            CALL_NAZAN => ctx.do_cast_self(30_693),
            REVENGE => {
                ctx.do_cast_victim(super::herald::difficulty_spell(ctx, 19_130, 40_392));
                self.boss.events.repeat_range(5_000, 10_000);
            }
            _ => unreachable!("unknown Vazruden engagement event{event}"),
        }
    }
}
