//! TC a352b1fa boss_vazruden_the_herald.cpp434–450: appearance/engagement.
//! Ordinary below40 Call Nazan and source paired-death completion.
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
        is_other_dead: false,
    })
}

struct Vazruden {
    boss: BossAi,
    has_called_nazan: bool,
    is_other_dead: bool,
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

    fn do_action(&mut self, _ctx: &mut Ctx, action: i32) {
        if action == super::ACTION_OTHER_IS_DEAD {
            self.is_other_dead = true;
        }
    }

    fn enter_evade_mode(&mut self, ctx: &mut Ctx) {
        ctx.talk(0);
        super::evade_encounter(ctx);
    }

    // TC475–480: source KilledUnit is unfiltered, including NPC victims.
    fn killed_unit(&mut self, ctx: &mut Ctx, _victim: Entity) {
        ctx.talk(2);
    }

    // TC482–495: completion precedes source speech, ordinary death Call and notification.
    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        if self.is_other_dead {
            self.boss.just_died(ctx);
        }
        ctx.talk(3);
        ctx.do_cast_self(30_693);
        super::notify_partner_death(ctx, 17_307);
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
                ctx.do_cast_victim(ctx.dungeon_mode(19_130, 40_392));
                self.boss.events.repeat_range(5_000, 10_000);
            }
            _ => unreachable!("unknown Vazruden engagement event{event}"),
        }
    }
}
