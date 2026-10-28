//! LittleWigs2777a9d Chargath/Sargha initial phases; energy controllers unported.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const CHARGATH_SCRIPT: &str = "boss_chargath";
pub(super) const SARGHA_SCRIPT: &str = "boss_sargha";

struct InitialPhaseProfile {
    first_casts: &'static [(u32, u64)],
    repeated_spell: u32,
    repeat_ms: u64,
    victim_spell: u32,
}

const CHARGATH_INITIAL_PHASE: InitialPhaseProfile = InitialPhaseProfile {
    first_casts: &[(373742, 5_000), (373733, 12_000), (373424, 24_200)],
    repeated_spell: 373742,
    repeat_ms: 12_100,
    victim_spell: 373733,
};

const SARGHA_INITIAL_PHASE: InitialPhaseProfile = InitialPhaseProfile {
    first_casts: &[(377204, 7_100), (377017, 14_200), (377473, 21_500)],
    repeated_spell: 377204,
    repeat_ms: 21_900,
    victim_spell: 377017,
};

pub(super) fn creature_ai(
    world: &World,
    actor: Entity,
    script: &str,
) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    if map.map_id != 2519 || !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205) {
        return None;
    }
    let profile = match script {
        CHARGATH_SCRIPT => &CHARGATH_INITIAL_PHASE,
        SARGHA_SCRIPT => &SARGHA_INITIAL_PHASE,
        _ => return None,
    };
    let boss = super::instance_script::bound_boss(world, actor)?;
    Some(Box::new(InitialPhaseAi {
        boss,
        profile,
        repeat_scheduled: false,
    }))
}

struct InitialPhaseAi {
    boss: BossAi,
    profile: &'static InitialPhaseProfile,
    repeat_scheduled: bool,
}

impl CreatureAi for InitialPhaseAi {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.repeat_scheduled = false;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.repeat_scheduled = false;
        for &(spell, delay) in self.profile.first_casts {
            self.boss.events.schedule(spell, delay);
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for InitialPhaseAi {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let target = if event == self.profile.victim_spell {
            ctx.victim()
        } else {
            None
        };
        let accepted = ctx.do_cast_at(target, event, false);
        let should_repeat = event == self.profile.repeated_spell && !self.repeat_scheduled;
        if accepted && should_repeat {
            // Both sources expose two casts of this lane before the missing
            // energy phase. No invented repeat after its second accepted start.
            self.boss.events.schedule(event, self.profile.repeat_ms);
            self.repeat_scheduled = true;
        }
    }
}
