//! LittleWigs2777a9d Ki'katal clocks; native70448 owns targets/effects.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "littlewigs_kikatal";
const MAP: u32 = 2660;
const ENTRY: u32 = 215407;
const WEBS: u32 = 432130;
const VOLLEY: u32 = 432227;
const SINGULARITY: u32 = 432117;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    let npc = world.get::<shared::components::Npc>(actor)?;
    if map.map_id != MAP
        || npc.template_id != ENTRY
        || !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205)
    {
        return None;
    }
    Some(Box::new(Kikatal {
        boss: super::instance_script::bound_boss(world, actor)?,
    }))
}

struct Kikatal {
    boss: BossAi,
}

impl BossScript for Kikatal {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, spell: u32) {
        let repeat_ms = match spell {
            WEBS => 18_200,
            VOLLEY => 23_000,
            SINGULARITY => 46_100,
            _ => return,
        };
        // KIKATAL_SINGULARITY_MINIMUM_ALERT_CONTEXT: range alerts do not
        // establish exact alternate casts. Preserve the exact ordinary lanes.
        // KIKATAL_WEBS_DUMMY_CHILD_UNSOURCED,
        // KIKATAL_GRASPING_BLOOD_CONTROLLER_UNSOURCED and
        // KIKATAL_SINGULARITY_ESCAPE_CONTROLLER_UNSOURCED remain gaps.
        if ctx.do_cast_at(Some(ctx.me), spell, false) {
            self.boss.events.schedule(spell, repeat_ms);
        }
    }
}

impl CreatureAi for Kikatal {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(WEBS, 6_100);
        self.boss.events.schedule(VOLLEY, 12_000);
        self.boss.events.schedule(SINGULARITY, 27_900);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}
