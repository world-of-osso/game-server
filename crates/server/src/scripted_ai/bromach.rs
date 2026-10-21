//! Current LittleWigs2777a9d Bromach.lua + native70448, explicitly partial.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_bromach";
const CALL: u32 = 369605;
const SLAM: u32 = 369703;
const TOTEM: u32 = 382303;
const BLOODLUST: u32 = 369754;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    if world.get::<crate::maps::InMap>(actor)?.map_id != 2451 {
        return None;
    }
    Some(Box::new(Bromach {
        boss: super::instance_script::bound_boss(world, actor)?,
        call_repeat: 0,
        totem_repeat: 0,
        bloodlust_repeat: 0,
        slam_repeat: 0,
    }))
}

struct Bromach {
    boss: BossAi,
    call_repeat: usize,
    totem_repeat: usize,
    bloodlust_repeat: usize,
    slam_repeat: usize,
}

impl CreatureAi for Bromach {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.call_repeat = 0;
        self.totem_repeat = 0;
        self.bloodlust_repeat = 0;
        self.slam_repeat = 0;
        self.boss.events.schedule(CALL, 5_100);
        self.boss.events.schedule(SLAM, 12_100);
        self.boss.events.schedule(TOTEM, 20_400);
        self.boss.events.schedule(BLOODLUST, 27_900);
    }

    fn on_spell_cast(&mut self, _ctx: &mut Ctx, spell: u32) {
        if spell == SLAM {
            // Lua SPELL_CAST_SUCCESS only; a native interruption invents no retry.
            let repeat = next_duration(&mut self.slam_repeat, [14_700, 28_000]);
            self.boss.events.reschedule(SLAM, repeat);
        }
    }

    fn just_summoned(&mut self, _ctx: &mut Ctx, summon: Entity) {
        // Track native382302 summons for reset/death, without inventing totem AI.
        self.boss.summons.summon(summon);
    }

    fn summoned_creature_despawn(&mut self, _ctx: &mut Ctx, summon: Entity) {
        self.boss.summoned_creature_despawn(summon);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Bromach {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let repeat = match event {
            SLAM => {
                ctx.do_cast_self(SLAM);
                return;
            }
            CALL => next_duration(&mut self.call_repeat, [28_000, 37_800]),
            TOTEM => next_duration(&mut self.totem_repeat, [30_300, 40_100]),
            BLOODLUST => next_duration(&mut self.bloodlust_repeat, [30_300, 40_100]),
            _ => return,
        };
        // Tremor's conditional countdown context remains a named gap. Native
        // ground/area selectors and totem child382302 are not overridden.
        ctx.do_cast_self(event);
        self.boss.events.repeat(repeat);
    }
}

fn next_duration(index: &mut usize, durations: [u64; 2]) -> u64 {
    let duration = durations[*index];
    *index = (*index + 1) % durations.len();
    duration
}
