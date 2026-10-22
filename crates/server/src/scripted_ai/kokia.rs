//! Current Retail LittleWigs2777a9d Kokia.lua + native70448 child/effect data.
//! KOKIA_TIMELINE_CONDITION_CONTEXT: explicit lead-approved source-order lanes,
//! not executable Blizzard timeline condition/count state.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_kokia_blazehoof";
const RITUAL: u32 = 372864;
/// Current372110 Dummy3 explicitly names372107, not a guessed legacy substitute.
const MOLTEN_BOULDER: u32 = 372107;
const SEARING_BLOWS: u32 = 372858;
const MOLTEN_DURATIONS_MS: &[u64] = &[19_000, 20_000];
const SHARED_REPEAT_MS: u64 = 40_000;

pub(super) fn creature_ai(boss: BossAi) -> Box<dyn CreatureAi> {
    Box::new(Kokia {
        boss,
        molten_step: 1,
    })
}

struct Kokia {
    boss: BossAi,
    molten_step: usize,
}

impl CreatureAi for Kokia {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.molten_step = 1;
        self.boss.events.schedule(RITUAL, 8_000);
        self.boss
            .events
            .schedule(MOLTEN_BOULDER, MOLTEN_DURATIONS_MS[0]);
        self.boss.events.schedule(SEARING_BLOWS, 28_000);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Kokia {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            RITUAL => {
                // Native current372864 is area Dummy3 without a child. Do not
                // manufacture legacy372863/1309540 forwarding or Firestorm adds.
                ctx.do_cast_self(RITUAL);
                self.boss.events.repeat(SHARED_REPEAT_MS);
            }
            MOLTEN_BOULDER => {
                // Existing-destination87 defaults to caster without an authored
                // selector. Projectile/area script remains a named source gap.
                ctx.do_cast_self(MOLTEN_BOULDER);
                let delay = MOLTEN_DURATIONS_MS[self.molten_step];
                self.molten_step = (self.molten_step + 1) % MOLTEN_DURATIONS_MS.len();
                self.boss.events.repeat(delay);
            }
            SEARING_BLOWS => {
                // Native Any25 explicit target: current victim; preserve aura23
                // periodic372859 weapon damage, duration, costs and cast flags.
                ctx.do_cast_victim(SEARING_BLOWS);
                self.boss.events.repeat(SHARED_REPEAT_MS);
            }
            _ => {}
        }
    }
}
