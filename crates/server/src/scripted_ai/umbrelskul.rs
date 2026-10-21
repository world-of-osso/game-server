//! Explicitly partial Lua-clock port: LittleWigs2777a9d Umbrelskul.lua +70448.
//! Energy/Brittle/crystal phases have no authored executable source here.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_umbrelskul";
const DRAGON_STRIKE: u32 = 384978;
const CRYSTALLINE_ROAR: u32 = 384699;
const MYTHIC_KEYSTONE: u32 = 8;
const MYTHIC_DUNGEON: u32 = 23;

pub(super) fn creature_ai(boss: BossAi) -> Box<dyn CreatureAi> {
    Box::new(Umbrelskul {
        boss,
        strike_repeat_ms: 0,
        roar_repeat_ms: 0,
    })
}

struct Umbrelskul {
    boss: BossAi,
    strike_repeat_ms: u64,
    roar_repeat_ms: u64,
}

impl CreatureAi for Umbrelskul {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        let map = ctx.map().expect("Umbrelskul instance");
        let difficulty = crate::zones::map_difficulty_id(ctx.world, map);
        let mythic = matches!(difficulty, MYTHIC_KEYSTONE | MYTHIC_DUNGEON);
        self.strike_repeat_ms = if mythic { 15_300 } else { 8_400 };
        self.roar_repeat_ms = if mythic { 105_600 } else { 23_100 };
        self.boss.events.schedule(DRAGON_STRIKE, 7_400);
        self.boss.events.schedule(CRYSTALLINE_ROAR, 12_200);
    }

    fn on_spell_cast(&mut self, _ctx: &mut Ctx, spell_id: u32) {
        if spell_id == DRAGON_STRIKE {
            // Lua starts this countdown only on SPELL_CAST_SUCCESS. Native
            // interrupted casts do not emit this hook or invent a retry.
            self.boss
                .events
                .reschedule(DRAGON_STRIKE, self.strike_repeat_ms);
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Umbrelskul {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            DRAGON_STRIKE => ctx.do_cast_victim(DRAGON_STRIKE),
            CRYSTALLINE_ROAR => {
                // Partial direct384699 request from Lua's ability countdown;
                // upstream384696 dummy-selector scripting remains unsourced.
                ctx.do_cast_victim(CRYSTALLINE_ROAR);
                self.boss.events.repeat(self.roar_repeat_ms);
            }
            _ => {}
        }
    }
}
