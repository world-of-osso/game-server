//! LittleWigs2777a9d WatcherIrideus.lua stage-one native partial.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx, DamageTaken};

pub(super) const SCRIPT: &str = "boss_irideus";
const FIST: u32 = 384524;
const SURGE: u32 = 384014;
const OVERLOAD: u32 = 389179;
const VOLLEY: u32 = 384351;
const STAGE_TWO_HEALTH_FRACTION: f32 = 0.15;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    if map.map_id != 2527 || !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205) {
        return None;
    }
    let boss = super::instance_script::bound_boss(world, actor)?;
    Some(Box::new(Irideus {
        boss,
        stage_one_stopped: false,
    }))
}

struct Irideus {
    boss: BossAi,
    stage_one_stopped: bool,
}

impl CreatureAi for Irideus {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.stage_one_stopped = false;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.stage_one_stopped = false;
        for (spell, delay) in [
            (FIST, 6_100),
            (SURGE, 10_600),
            (OVERLOAD, 25_500),
            (VOLLEY, 29_200),
        ] {
            self.boss.events.schedule(spell, delay);
        }
    }

    fn on_spell_cast(&mut self, _ctx: &mut Ctx, spell: u32) {
        if spell == OVERLOAD && !self.stage_one_stopped {
            self.boss.events.reschedule(OVERLOAD, 27_700);
        }
    }

    fn damage_taken(&mut self, _ctx: &mut Ctx, _attacker: Option<Entity>, damage: DamageTaken) {
        if damage.health_before - damage.damage <= damage.max_health * STAGE_TWO_HEALTH_FRACTION {
            self.stage_one_stopped = true;
            self.boss.events.reset();
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if !self.stage_one_stopped {
            boss_update_ai(self, ctx, diff_ms);
        }
    }
}

impl BossScript for Irideus {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let repeat_ms = match event {
            FIST => 17_000,
            SURGE => 27_900,
            VOLLEY => 31_500,
            OVERLOAD => {
                ctx.do_cast_aoe(event);
                return;
            }
            _ => return,
        };
        // Native implicit targets own cone/area/self selection. These three
        // Lua SPELL_CAST_START clocks exclude completion/channel duration.
        if ctx.do_cast_at(None, event, false) {
            self.boss.events.schedule(event, repeat_ms);
        }
    }
}
