//! LittleWigs2777a9d Aldryr clocks; native70448 owns targets/effects.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "littlewigs_aldryr";
const MAP: u32 = 2661;
const ENTRY: u32 = 210271;
const KEG_SMASH: u32 = 432229;
const THROW_CINDERBREW: u32 = 432179;
const BLAZING_BELCH: u32 = 432198;
const KEG_FIRST_MS: u32 = 5_100;
const KEG_REPEAT_MS: u32 = 14_500;
const THROW_FIRST_MS: u32 = 10_000;
const THROW_REPEAT_MS: u32 = 18_200;
const BELCH_FIRST_MS: u32 = 14_400;
const BELCH_REPEAT_MS: u32 = 23_000;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    let npc = world.get::<shared::components::Npc>(actor)?;
    if map.map_id != MAP
        || npc.template_id != ENTRY
        || !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205)
    {
        return None;
    }
    Some(Box::new(Aldryr {
        boss: super::instance_script::bound_boss(world, actor)?,
    }))
}

struct Aldryr {
    boss: BossAi,
}

impl BossScript for Aldryr {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, spell: u32) {
        let (target, repeat_ms) = match spell {
            KEG_SMASH => (ctx.victim(), KEG_REPEAT_MS),
            THROW_CINDERBREW => (Some(ctx.me), THROW_REPEAT_MS),
            BLAZING_BELCH => (Some(ctx.me), BELCH_REPEAT_MS),
            _ => return,
        };
        // ALDRYR_HAPPY_HOUR_BAR_PATRON_CONTROLLER_UNSOURCED: no invented
        // phase casts, movement, patrons or resume timers at66/33percent.
        // ALDRYR_THROW_DUMMY_CHILD_UNSOURCED and
        // ALDRYR_BELCH_DESTINATION_SELECTOR_UNSOURCED retain native defaults.
        if ctx.do_cast_at(target, spell, false) {
            self.boss.events.schedule(spell, repeat_ms);
        }
    }
}

impl CreatureAi for Aldryr {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(KEG_SMASH, KEG_FIRST_MS);
        self.boss.events.schedule(THROW_CINDERBREW, THROW_FIRST_MS);
        self.boss.events.schedule(BLAZING_BELCH, BELCH_FIRST_MS);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}
