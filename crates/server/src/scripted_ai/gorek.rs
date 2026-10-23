//! LittleWigs2777a9d Gorek; native70448 owns aura/child effects and legality.
//! GOREK_MINIMUM_ALERT_CONTEXT selects the source lower-bound post-Forge lanes.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_gorek";
const MIGHT: u32 = 374635;
const AEGIS: u32 = 374842;
const HEATED: u32 = 374533;
const FORGESTORM: u32 = 374969;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    if map.map_id != 2519 || !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205) {
        return None;
    }
    let boss = super::instance_script::bound_boss(world, actor)?;
    Some(Box::new(Gorek {
        boss,
        forge_count: 0,
    }))
}

struct Gorek {
    boss: BossAi,
    forge_count: u32,
}

impl CreatureAi for Gorek {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.forge_count = 0;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.forge_count = 0;
        for (spell, delay) in [
            (MIGHT, 3_600),
            (AEGIS, 12_100),
            (HEATED, 20_500),
            (FORGESTORM, 29_500),
        ] {
            self.boss.events.schedule(spell, delay);
        }
    }

    fn on_spell_cast(&mut self, _ctx: &mut Ctx, spell: u32) {
        match spell {
            MIGHT => {
                self.forge_count += 1;
                let first = self.forge_count == 1;
                self.boss.events.reschedule(MIGHT, 30_400);
                self.boss.events.reschedule(AEGIS, 8_300);
                self.boss
                    .events
                    .reschedule(HEATED, if first { 16_600 } else { 15_800 });
                self.boss
                    .events
                    .reschedule(FORGESTORM, if first { 25_200 } else { 24_300 });
            }
            AEGIS | FORGESTORM => self.boss.events.reschedule(spell, 30_400),
            _ => {}
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Gorek {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            MIGHT | FORGESTORM => ctx.do_cast_self(event),
            AEGIS => ctx.do_cast_victim(event),
            HEATED => {
                ctx.do_cast_victim(event);
                self.boss.events.repeat(30_400);
            }
            _ => {}
        }
    }
}
