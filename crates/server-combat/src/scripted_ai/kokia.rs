//! Current Retail LittleWigs2777a9d Kokia.lua + native70448 child/effect data.
//! KOKIA_TIMELINE_CONDITION_CONTEXT: explicit lead-approved source-order lanes,
//! not executable Blizzard timeline condition/count state.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_kokia_blazehoof";
const RITUAL: u32 = 372863;
const RITUAL_SUMMON: u32 = 1309540;
/// Current372110 Dummy3 explicitly names372107, not a guessed legacy substitute.
const MOLTEN_BOULDER: u32 = 372107;
const SEARING_BLOWS: u32 = 372858;
const RITUAL_REPEAT_MS: u64 = 40_088;
const BOULDER_REPEAT_MS: u64 = 20_650;
const SEARING_REPEAT_MS: u64 = 40_128;

pub(super) fn creature_ai(boss: BossAi) -> Box<dyn CreatureAi> {
    Box::new(Kokia {
        boss,
        ritual_target: None,
    })
}

struct Kokia {
    boss: BossAi,
    ritual_target: Option<Entity>,
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
        self.ritual_target = None;
        self.boss.events.schedule(RITUAL, 8_502);
        self.boss.events.schedule(MOLTEN_BOULDER, 19_431);
        self.boss.events.schedule(SEARING_BLOWS, 28_953);
    }

    fn on_spell_cast(&mut self, ctx: &mut Ctx, spell_id: u32) {
        if spell_id == RITUAL
            && let Some(target) = self.ritual_target.take()
        {
            ctx.do_cast_at(Some(target), RITUAL_SUMMON, true);
        }
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
                // LOG_TIMERS_FROM_MYTHIC_PLUS: the logged ritual completes before
                // its native1309540 summon; interruptions must not summon.
                self.ritual_target = ctx.select_target_random(0.0, false);
                if let Some(target) = self.ritual_target {
                    ctx.do_cast_at(Some(target), RITUAL, false);
                }
                self.boss.events.repeat(RITUAL_REPEAT_MS);
            }
            MOLTEN_BOULDER => {
                // Existing-destination87 defaults to caster without an authored
                // selector. Projectile/area script remains a named source gap.
                ctx.do_cast_self(MOLTEN_BOULDER);
                self.boss.events.repeat(BOULDER_REPEAT_MS);
            }
            SEARING_BLOWS => {
                // Native Any25 explicit target: current victim; preserve aura23
                // periodic372859 weapon damage, duration, costs and cast flags.
                ctx.do_cast_victim(SEARING_BLOWS);
                self.boss.events.repeat(SEARING_REPEAT_MS);
            }
            _ => {}
        }
    }
}
