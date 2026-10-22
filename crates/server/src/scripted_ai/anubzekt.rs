//! Local LittleWigs2777a9d Anubzekt.lua clocks; native70448 targets/effects.
//! Dummy children, center movement and434408 phase-exit ingress are unsourced.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "littlewigs_anubzekt";
const IMPALE: u32 = 435012;
const BURROW: u32 = 439506;
const INFESTATION: u32 = 433740;
const EYE: u32 = 433766;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    let npc = world.get::<shared::components::Npc>(actor)?;
    if map.map_id != 2660
        || npc.template_id != 215405
        || !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205)
    {
        return None;
    }
    Some(Box::new(Anubzekt {
        boss: super::instance_script::bound_boss(world, actor)?,
        burrows_remaining: 1,
        combo_impale: false,
        eye_started: false,
    }))
}

struct Anubzekt {
    boss: BossAi,
    burrows_remaining: u8,
    combo_impale: bool,
    eye_started: bool,
}

impl Anubzekt {
    fn reset_counts(&mut self) {
        self.burrows_remaining = 1;
        self.combo_impale = false;
        self.eye_started = false;
    }

    fn restart_eye_countdowns(&mut self) {
        self.eye_started = true;
        self.burrows_remaining = 2;
        self.boss.events.reschedule(IMPALE, 10_900);
        self.boss.events.reschedule(INFESTATION, 7_300);
        self.boss.events.reschedule(BURROW, 46_900);
        self.boss.events.schedule(EYE, 78_900);
    }

    // ANUBZEKT_INFESTATION_MINIMUM_ALERT_CONTEXT: Lua's healer-only
    //5.0/9.8 lower-bound/range alerts do not invent exact alternate casts.
    // No fabricated25s Eye exit; the434408 controller remains a named gap.
    fn schedule_after_start(&mut self, spell: u32) {
        match spell {
            IMPALE => {
                let repeat = if self.combo_impale { 5_800 } else { 8_500 };
                self.combo_impale = false;
                self.boss.events.schedule(IMPALE, repeat);
            }
            BURROW => {
                self.combo_impale = true;
                self.burrows_remaining = self.burrows_remaining.saturating_sub(1);
                if self.burrows_remaining > 0 {
                    self.boss.events.schedule(BURROW, 30_200);
                }
                self.boss.events.reschedule(IMPALE, 4_800);
            }
            EYE => self.restart_eye_countdowns(),
            INFESTATION => {
                let repeat = if self.eye_started { 8_500 } else { 10_100 };
                self.boss.events.schedule(INFESTATION, repeat);
            }
            _ => {}
        }
    }
}

impl BossScript for Anubzekt {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, spell: u32) {
        let target = match spell {
            IMPALE | BURROW => ctx.victim(),
            INFESTATION | EYE => Some(ctx.me),
            _ => return,
        };
        if ctx.do_cast_at(target, spell, false) {
            self.schedule_after_start(spell);
        }
    }
}

impl CreatureAi for Anubzekt {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.reset_counts();
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.reset_counts();
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(INFESTATION, 0);
        self.boss.events.schedule(IMPALE, 4_600);
        self.boss.events.schedule(BURROW, 14_300);
        self.boss.events.schedule(EYE, 29_100);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.reset_counts();
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}
