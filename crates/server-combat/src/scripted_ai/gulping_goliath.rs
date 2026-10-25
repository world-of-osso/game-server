//! LittleWigs2777a9d GulpingGoliath.lua; native70448 owns effects/targets/occupancy.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_gulping_goliath";
const OVERPOWERING_CROAK: u32 = 385181;
const GULP: u32 = 385551;
const TOXIC_EFFLUVIA: u32 = 385442;
const BELLY_SLAM: u32 = 385531;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    if map.map_id != 2527 || !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205) {
        return None;
    }
    let boss = super::instance_script::bound_boss(world, actor)?;
    Some(Box::new(Goliath {
        boss,
        gulp_count: 0,
        toxic_count: 0,
    }))
}

struct Goliath {
    boss: BossAi,
    gulp_count: u32,
    toxic_count: u32,
}

impl CreatureAi for Goliath {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.gulp_count = 0;
        self.toxic_count = 0;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.gulp_count = 0;
        self.toxic_count = 0;
        self.boss.events.schedule(OVERPOWERING_CROAK, 8_400);
        self.boss.events.schedule(GULP, 18_100);
        self.boss.events.schedule(TOXIC_EFFLUVIA, 30_300);
        self.boss.events.schedule(BELLY_SLAM, 38_800);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Goliath {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let repeat = match event {
            OVERPOWERING_CROAK | BELLY_SLAM => 38_800,
            GULP => {
                self.gulp_count += 1;
                if self.gulp_count == 1 { 47_300 } else { 38_800 }
            }
            TOXIC_EFFLUVIA => {
                self.toxic_count += 1;
                if self.toxic_count == 1 {
                    26_700
                } else {
                    38_800
                }
            }
            _ => return,
        };
        match event {
            BELLY_SLAM => ctx.do_cast_victim(event),
            // Native EXCLUDE_CASTER rejects an explicit self target. Source18/8
            // owns the Gulp cone; omit explicit target rather than invent one.
            GULP => ctx.do_cast_aoe(event),
            _ => ctx.do_cast_self(event),
        }
        self.boss.events.repeat(repeat);
    }
}
