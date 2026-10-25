//! LittleWigs2777a9d Emberon.lua Normal/Follower partial; native70448 effects.
use bevy::prelude::*;
use shared::components::Health;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_emberon";
const CLAP: u32 = 369061;
const EMBERS: u32 = 369110;
const PURGING: u32 = 368990;
const NORMAL_PURGING_MS: u32 = 24_300;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    if map.map_id != 2451 || !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205) {
        return None;
    }
    Some(Box::new(Emberon {
        boss: super::instance_script::bound_boss(world, actor)?,
        phase_remaining_ms: None,
        purging_disabled: false,
        claps_remaining: 2,
        embers_remaining: 3,
    }))
}

struct Emberon {
    boss: BossAi,
    phase_remaining_ms: Option<u32>,
    purging_disabled: bool,
    claps_remaining: u8,
    embers_remaining: u8,
}

impl Emberon {
    fn reset_phase(&mut self) {
        self.phase_remaining_ms = None;
        self.purging_disabled = false;
        self.claps_remaining = 2;
        self.embers_remaining = 3;
    }

    fn disable_purging_below_ten_percent(&mut self, ctx: &Ctx) {
        let below_ten = ctx
            .world
            .get::<Health>(ctx.me)
            .is_some_and(|health| health.current < health.max * 0.1);
        if below_ten && !self.purging_disabled {
            self.purging_disabled = true;
            self.boss.events.cancel(PURGING);
        }
    }

    fn finish_normal_purging(&mut self, ctx: &mut Ctx) {
        // EMBERON_NORMAL_PURGING_24P3S_LUA: Lua105–106 explicitly describes
        // Normal's fixed24.3s CastBar from SPELL_CAST_START. DB2 stays indefinite.
        crate::spell_cast::end_cast(ctx.world, ctx.me);
        // Native Duration-1 keeps its aura without a CastState (start_channel
        // rejects nonpositive duration). End that actual channel aura as well.
        crate::auras::channel_ended(ctx.world, ctx.me, PURGING);
        self.phase_remaining_ms = None;
        self.boss.events.reset();
        self.claps_remaining = 2;
        self.embers_remaining = 4;
        self.boss.events.schedule(EMBERS, 1_200);
        self.boss.events.schedule(CLAP, 5_400);
        if !self.purging_disabled {
            self.boss.events.schedule(PURGING, 37_700);
        }
    }

    fn cast_normal_lane(&mut self, ctx: &mut Ctx, spell: u32) {
        let (remaining, repeat_ms) = match spell {
            CLAP => (&mut self.claps_remaining, 23_000),
            EMBERS => (&mut self.embers_remaining, 11_700),
            _ => return,
        };
        if *remaining == 0 && !self.purging_disabled {
            return;
        }
        if ctx.do_cast_at(Some(ctx.me), spell, false) {
            *remaining = remaining.saturating_sub(1);
            // Lua retains nextSearingClap/nextUnstableEmbers after the last
            // count. Keep that future deadline: below10% can resume it, but an
            // already elapsed deadline does not invent a new cast.
            self.boss.events.repeat(repeat_ms);
        }
    }
}

impl CreatureAi for Emberon {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.reset_phase();
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.reset_phase();
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(CLAP, 4_500);
        self.boss.events.schedule(EMBERS, 12_100);
        self.boss.events.schedule(PURGING, 38_500);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.reset_phase();
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.disable_purging_below_ten_percent(ctx);
        if let Some(remaining) = self.phase_remaining_ms {
            if !ctx.update_victim() {
                return;
            }
            let remaining = remaining.saturating_sub(diff_ms);
            self.phase_remaining_ms = Some(remaining);
            if remaining == 0 {
                self.finish_normal_purging(ctx);
            }
            return;
        }
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Emberon {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            CLAP | EMBERS => self.cast_normal_lane(ctx, event),
            PURGING if !self.purging_disabled => {
                if ctx.do_cast_at(Some(ctx.me), PURGING, false) {
                    self.boss.events.reset();
                    self.phase_remaining_ms = Some(NORMAL_PURGING_MS);
                }
            }
            _ => {}
        }
    }
}
