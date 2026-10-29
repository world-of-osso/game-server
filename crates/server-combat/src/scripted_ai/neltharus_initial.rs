//! LittleWigs2777a9d Chargath/Sargha initial alert policy; energy controllers unported.
use bevy::prelude::*;

use crate::auras::AuraInstance;
use crate::auras::scripts::{AuraScript, AuraScripts};

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const CHARGATH_SCRIPT: &str = "boss_chargath";
pub(super) const SARGHA_SCRIPT: &str = "boss_sargha";
const SHIELD_EXPIRED: i32 = 1;

struct InitialPhaseProfile {
    first_casts: &'static [(u32, u64)],
    repeated_spell: u32,
    repeat_ms: u64,
    victim_spell: u32,
}

const CHARGATH_INITIAL_PHASE: InitialPhaseProfile = InitialPhaseProfile {
    first_casts: &[
        (373742, 5_000),
        (373733, 12_000),
        (373424, 24_200),
        (375056, 31_100),
    ],
    repeated_spell: 373742,
    repeat_ms: 12_100,
    victim_spell: 373733,
};

const SARGHA_INITIAL_PHASE: InitialPhaseProfile = InitialPhaseProfile {
    first_casts: &[
        (377204, 7_100),
        (377017, 14_200),
        (377473, 21_500),
        (376780, 37_700),
    ],
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
        expired_shield_phase: false,
        gold_repeated: false,
    }))
}

struct InitialPhaseAi {
    boss: BossAi,
    profile: &'static InitialPhaseProfile,
    repeat_scheduled: bool,
    expired_shield_phase: bool,
    gold_repeated: bool,
}

impl CreatureAi for InitialPhaseAi {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.repeat_scheduled = false;
        self.expired_shield_phase = false;
        self.gold_repeated = false;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.repeat_scheduled = false;
        self.expired_shield_phase = false;
        self.gold_repeated = false;
        for &(spell, delay) in self.profile.first_casts {
            self.boss.events.schedule(spell, delay);
        }
    }

    fn do_action(&mut self, ctx: &mut Ctx, action: i32) {
        if action != SHIELD_EXPIRED || self.profile.victim_spell != 377017 || !ctx.is_engaged() {
            return;
        }
        self.expired_shield_phase = true;
        self.repeat_scheduled = false;
        self.gold_repeated = false;
        for spell in [377017, 377473, 377204] {
            self.boss.events.reschedule(spell, 2_400);
        }
        self.boss.events.reschedule(376780, 32_600);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

pub(super) fn register_aura_scripts(mut scripts: ResMut<AuraScripts>) {
    scripts.0.insert(
        376780,
        AuraScript {
            on_natural_end: Some(shield_expired),
            ..Default::default()
        },
    );
}

fn shield_expired(world: &mut World, unit: Entity, _aura: &AuraInstance) {
    super::dispatch(world, unit, super::Hook::DoAction(SHIELD_EXPIRED));
}

impl BossScript for InitialPhaseAi {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let target = if event == self.profile.victim_spell || event == 375056 {
            ctx.victim()
        } else {
            None
        };
        let accepted = ctx.do_cast_at(target, event, false);
        let should_repeat = event == self.profile.repeated_spell && !self.repeat_scheduled;
        if accepted && should_repeat {
            self.boss.events.schedule(event, self.profile.repeat_ms);
            self.repeat_scheduled = true;
        }
        if accepted && event == 377017 && self.expired_shield_phase && !self.gold_repeated {
            self.boss.events.schedule(event, 26_700);
            self.gold_repeated = true;
        }
    }
}
