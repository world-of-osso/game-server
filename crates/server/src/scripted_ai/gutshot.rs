//! LittleWigs2777a9d Gutshot.lua non-Mythic partial; native70448 effects.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_gutshot";
const TRAP_PRECAST: u32 = 383979;
const GUT_SHOT: u32 = 384353;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    if map.map_id != 2520 || !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205) {
        return None;
    }
    let boss = super::instance_script::bound_boss(world, actor)?;
    Some(Box::new(Gutshot { boss }))
}

struct Gutshot {
    boss: BossAi,
}

impl CreatureAi for Gutshot {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(TRAP_PRECAST, 8_400);
        self.boss.events.schedule(GUT_SHOT, 12_000);
    }

    fn on_spell_cast(&mut self, _ctx: &mut Ctx, spell: u32) {
        let repeat = match spell {
            TRAP_PRECAST => 17_000,
            GUT_SHOT => 15_700,
            _ => return,
        };
        // Lua explicitly arms Gut Shot's countdown only after success; the boss
        // can interrupt itself. No completion means no invented repeat/retry.
        self.boss.events.reschedule(spell, repeat);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Gutshot {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            TRAP_PRECAST => ctx.do_cast_aoe(event),
            GUT_SHOT => ctx.do_cast_victim(event),
            _ => {}
        }
    }
}
