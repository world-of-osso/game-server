//! LOG_TIMERS_FROM_MYTHIC_PLUS and the explicit Ruby joint-death policy.
//! LittleWigs2777a9d KyrakkaAndErkhartStormvein.lua:56-65,369-385,460-469
//! identifies the two actors, 50% stage warning, 181089 stage event and phase spells.
use bevy::prelude::*;
use shared::components::{Health, Npc, Position};

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx, MotionType};
use crate::maps::InMap;

pub(crate) const SCRIPT: &str = "boss_kyrakka_erkhart";
const KYRAKKA: u32 = 190484;
const ERKHART: u32 = 190485;
const FIREBREATH: u32 = 381525;
const FLAMESPIT_ONE: u32 = 381602;
const FLAMESPIT_TWO: u32 = 381605;
const STORMSLAM: u32 = 381512;
const WINDS: u32 = 381517;
const CLOUDBURST: u32 = 381516;
const ENCOUNTER_EVENT: u32 = 181089;
const LANDING_POINT: u32 = 1;
const DEATH_DATA: u32 = 0x524c_5001;
// ERKHART_181089_MEDIAN_DELAY: <=50% to181089 median24.308s,
// observed range17.220–29.734s. This is an approximation, not a sourced trigger.
const STAGE_DELAY_MS: u32 = 24_308;
// LOG_OBSERVED_KYRAKKA_LANDING: all four kills observed this XY before181089;
// interim Z312.8835 is the supplied neighbor_path geometry estimate.
const LANDING: Position = Position {
    x: 1798.47,
    y: 312.8835,
    z: -36.11,
};

pub(super) fn creature_ai(boss: BossAi, entry: u32) -> Box<dyn CreatureAi> {
    Box::new(RubyFinal {
        boss,
        entry,
        phase_pending: false,
        stage_remaining_ms: None,
        landed: false,
    })
}

struct RubyFinal {
    boss: BossAi,
    entry: u32,
    phase_pending: bool,
    stage_remaining_ms: Option<u32>,
    landed: bool,
}

impl CreatureAi for RubyFinal {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.phase_pending = false;
        self.stage_remaining_ms = None;
        self.landed = false;
        if let Some(map) = ctx.map() {
            super::instance_script::set_data(ctx.world, map, DEATH_DATA, 0);
        }
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        if let Some(peer) = find_peer(ctx, self.entry) {
            ctx.do_zone_in_combat(peer);
        }
        if self.entry == KYRAKKA {
            self.boss.events.schedule(FIREBREATH, 2_713);
            self.boss.events.schedule(FLAMESPIT_ONE, 15_608);
        } else {
            self.boss.events.schedule(STORMSLAM, 6_098);
            self.boss.events.schedule(WINDS, 10_991);
            self.boss.events.schedule(CLOUDBURST, 21_880);
        }
    }

    fn movement_inform(&mut self, _ctx: &mut Ctx, kind: MotionType, id: u32) {
        if kind == MotionType::Point && id == LANDING_POINT {
            self.landed = true;
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.events.reset();
        let Some(map) = ctx.map() else { return };
        let bit = if self.entry == KYRAKKA { 1 } else { 2 };
        let dead = super::instance_script::get_data(ctx.world, map, DEATH_DATA) | bit;
        super::instance_script::set_data(ctx.world, map, DEATH_DATA, dead);
        if dead == 3 {
            self.boss.just_died(ctx);
        } else {
            super::instance_script::send_disengage_unit(ctx.world, map, ctx.me);
        }
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if self.entry == KYRAKKA && !self.phase_pending && ctx.is_engaged() && either_at_half(ctx) {
            self.phase_pending = true;
            ctx.move_point(ctx.me, LANDING_POINT, LANDING);
            self.stage_remaining_ms = Some(STAGE_DELAY_MS);
        }
        if ctx.is_engaged() {
            self.advance_stage(ctx, diff_ms);
        }
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl RubyFinal {
    fn advance_stage(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        let Some(remaining) = self.stage_remaining_ms.as_mut() else {
            return;
        };
        *remaining = remaining.saturating_sub(diff_ms);
        if *remaining != 0 || !self.landed {
            return;
        }
        self.stage_remaining_ms = None;
        let Some(peer) = find_peer(ctx, KYRAKKA) else {
            warn!("Ruby stage event cannot find Erkhart");
            return;
        };
        if let Err(error) =
            crate::spell_cast::creature_cast(ctx.world, peer, ENCOUNTER_EVENT, Some(peer), true)
        {
            warn!("Ruby stage event failed: {error:?}");
            return;
        }
        self.boss.events.cancel(FLAMESPIT_ONE);
        self.boss.events.schedule(FLAMESPIT_TWO, 14_560);
    }
}

impl BossScript for RubyFinal {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let repeat = match event {
            FIREBREATH => 21_274,
            FLAMESPIT_ONE => 23_686,
            FLAMESPIT_TWO => 16_987,
            STORMSLAM => 26_096,
            WINDS => 23_112,
            CLOUDBURST => 25_501,
            _ => return,
        };
        if event == STORMSLAM {
            ctx.do_cast_victim(event);
        } else {
            ctx.do_cast_self(event);
        }
        self.boss.events.repeat(repeat);
    }
}

fn find_peer(ctx: &mut Ctx, entry: u32) -> Option<Entity> {
    let map = ctx.map()?;
    let other = if entry == KYRAKKA { ERKHART } else { KYRAKKA };
    ctx.world
        .query::<(Entity, &InMap, &Npc)>()
        .iter(ctx.world)
        .find_map(|(entity, actor_map, npc)| {
            (*actor_map == map && npc.template_id == other).then_some(entity)
        })
}

fn either_at_half(ctx: &mut Ctx) -> bool {
    let Some(map) = ctx.map() else { return false };
    ctx.world
        .query::<(&InMap, &Npc, &Health)>()
        .iter(ctx.world)
        .any(|(actor_map, npc, health)| {
            *actor_map == map
                && [KYRAKKA, ERKHART].contains(&npc.template_id)
                && health.current > 0.0
                && health.current <= health.max * 0.5
        })
}
