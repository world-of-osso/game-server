//! LittleWigs2777a9d current Retail alert lane; native70448 owns cone/cast/damage.
//! Full-energy Fissure, Maxdormu's Orbs and Bomb controllers remain named gaps.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_vexamus";
const ARCANE_EXPULSION: u32 = 385958;
const MANA_BOMBS: u32 = 386173;

pub(super) fn creature_ai(boss: BossAi) -> Box<dyn CreatureAi> {
    Box::new(Vexamus { boss })
}

struct Vexamus {
    boss: BossAi,
}

impl CreatureAi for Vexamus {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(ARCANE_EXPULSION, 5_000);
        self.boss.events.schedule(MANA_BOMBS, 15_000);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Vexamus {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            ARCANE_EXPULSION => ctx.do_cast_victim(ARCANE_EXPULSION),
            MANA_BOMBS => ctx.do_cast_self(MANA_BOMBS),
            _ => return,
        }
        self.boss.events.repeat(18_000);
    }
}
