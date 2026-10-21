//! Current Retail LittleWigs2777a9d timelines; native70448 owns targets/effects.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_echo_of_doragosa";
const MISSILES: u32 = 373326;
const ASTRAL_BLAST: u32 = 1282251;
const ENERGY_BOMB: u32 = 374343;
const POWER_VACUUM: u32 = 388822;

pub(super) fn creature_ai(boss: BossAi) -> Box<dyn CreatureAi> {
    Box::new(Doragosa { boss })
}

struct Doragosa {
    boss: BossAi,
}

impl CreatureAi for Doragosa {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(MISSILES, 7_000);
        // Lua explicitly says6.5s Astral Blast bars are always canceled: no cast.
        self.boss.events.schedule(ASTRAL_BLAST, 9_000);
        self.boss.events.schedule(ENERGY_BOMB, 14_000);
        self.boss.events.schedule(POWER_VACUUM, 28_000);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Doragosa {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let repeat = match event {
            MISSILES => 10_000,
            ASTRAL_BLAST => 12_000,
            ENERGY_BOMB => 14_000,
            POWER_VACUUM => 28_000,
            _ => return,
        };
        // Explicit victim for native25/6; area/default selectors stay native.
        ctx.do_cast_victim(event);
        self.boss.events.repeat(repeat);
    }
}
