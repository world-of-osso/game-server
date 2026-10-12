//! TC script schedule (partial; Retail 12.1 timeline inputs unavailable).
//! Pinned a352b1fa boss_golden_serpent.cpp. No Retail alert clock is inferred.
use crate::scripted_ai::boss_ai::{BossAi, BossScript, boss_update_ai};
use crate::scripted_ai::instance_script::{self, EncounterState};
use crate::scripted_ai::{CreatureAi, Ctx};
use bevy::prelude::*;
use shared::components::{Position, Rotation, UnitFactionTemplate, UnitFlags};

pub(super) const ENTRY: u32 = 135322;
const BOSS_ID: u32 = 0;
pub(super) const ACTION_ANNOUNCE_ABSORB: i32 = 1;
const SPIT_GOLD: u32 = 265773;
const SERPENTINE_GUST: u32 = 265781;
const TAIL_THRASH: u32 = 265910;
const LUCRES_CALL: u32 = 265923;
const EMERGE_STATE: u32 = 271598;
const FRIENDLY_FACTION: u32 = 35;
const RESPAWN_POSITION: [f32; 4] = [-1058.8403, 2615.1667, 810.1519, 5.113396];
const EVADE_RESPAWN_SECONDS: u32 = 30;
const EVENT_SPIT_GOLD: u32 = 1;
const EVENT_SERPENTINE_GUST: u32 = 2;
const EVENT_TAIL_THRASH: u32 = 3;
const EVENT_LUCRES_CALL: u32 = 4;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let Some(boss) = instance_script::bound_boss(world, actor) else {
        error!("Golden Serpent requires its authenticated primary encounter binding");
        return None;
    };
    if boss.boss_id != BOSS_ID {
        error!(
            "Golden Serpent source ordinal0 differs from binding{}",
            boss.boss_id
        );
        return None;
    }
    Some(Box::new(GoldenSerpent {
        boss,
        announced_luster: false,
    }))
}

struct GoldenSerpent {
    boss: BossAi,
    announced_luster: bool,
}

impl GoldenSerpent {
    fn cast_spit_gold(&mut self, ctx: &mut Ctx) {
        if let Some(target) = ctx.select_target_random(300.0, true) {
            ctx.do_cast_at(Some(target), SPIT_GOLD, false);
        }
        self.boss.events.schedule(EVENT_SPIT_GOLD, 11_000);
    }
}

impl BossScript for GoldenSerpent {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            EVENT_SPIT_GOLD => self.cast_spit_gold(ctx),
            EVENT_SERPENTINE_GUST => {
                ctx.do_cast_self(SERPENTINE_GUST);
                self.boss.events.schedule(EVENT_SERPENTINE_GUST, 21_500);
            }
            EVENT_TAIL_THRASH => {
                ctx.do_cast_victim(TAIL_THRASH);
                self.boss.events.schedule(EVENT_TAIL_THRASH, 17_000);
            }
            EVENT_LUCRES_CALL => {
                ctx.do_cast_self(LUCRES_CALL);
                self.announced_luster = false;
                self.boss.events.schedule(EVENT_LUCRES_CALL, 41_500);
            }
            _ => {}
        }
    }
}

impl CreatureAi for GoldenSerpent {
    fn reset(&mut self, _ctx: &mut Ctx) {
        self.announced_luster = false;
    }

    fn do_action(&mut self, ctx: &mut Ctx, action: i32) {
        if action == ACTION_ANNOUNCE_ABSORB && !self.announced_luster {
            ctx.talk(0);
            self.announced_luster = true;
        }
    }

    fn initialize_ai(&mut self, ctx: &mut Ctx) {
        let map = ctx.map().expect("Golden Serpent instance map");
        let state = instance_script::get_boss_state(ctx.world, map, BOSS_ID)
            .expect("Golden Serpent authenticated boss state");
        if state != EncounterState::NotStarted {
            ctx.world.entity_mut(ctx.me).insert((
                Position {
                    x: RESPAWN_POSITION[0],
                    y: RESPAWN_POSITION[2],
                    z: -RESPAWN_POSITION[1],
                },
                Rotation {
                    x: 0.0,
                    y: RESPAWN_POSITION[3] + std::f32::consts::FRAC_PI_2,
                    z: 0.0,
                },
            ));
        }
    }

    fn just_appeared(&mut self, ctx: &mut Ctx) {
        let map = ctx.map().expect("Golden Serpent instance map");
        if instance_script::get_boss_state(ctx.world, map, BOSS_ID)
            != Some(EncounterState::NotStarted)
        {
            return;
        }
        let mut flags = *ctx
            .world
            .get::<UnitFlags>(ctx.me)
            .expect("population unit flags");
        flags.0 |= UnitFlags::NOT_SELECTABLE;
        ctx.world
            .entity_mut(ctx.me)
            .insert((UnitFactionTemplate(FRIENDLY_FACTION), flags));
        if !ctx.do_cast_at(Some(ctx.me), EMERGE_STATE, false) {
            warn!("Golden Serpent source emerge271598 unavailable; intro remains closed");
        }
        // Source gravity/flying and three authored paths remain unconverted.
        // Do not synthesize a landing callback or unlock friendly intro combat.
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(EVENT_SPIT_GOLD, 9_000);
        self.boss.events.schedule(EVENT_SERPENTINE_GUST, 12_500);
        self.boss.events.schedule(EVENT_TAIL_THRASH, 15_500);
        self.boss.events.schedule(EVENT_LUCRES_CALL, 40_000);
    }

    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        self.boss.just_summoned(ctx, summon);
    }

    fn summoned_creature_despawn(&mut self, _ctx: &mut Ctx, summon: Entity) {
        self.boss.summoned_creature_despawn(summon);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn enter_evade_mode(&mut self, ctx: &mut Ctx) {
        let map = ctx.map().expect("Golden Serpent instance map");
        instance_script::send_disengage_unit(ctx.world, map, ctx.me);
        self.boss.summons.despawn_all(ctx.world);
        // Generic evade already ran _EnterEvadeMode before dispatching this hook.
        if ctx.world.get::<crate::zones::TempSummon>(ctx.me).is_some() {
            crate::zones::unsummon(ctx.world, ctx.me);
            return;
        }
        crate::scripted_ai::smart_ai::lifetimes::despawn_creature(
            ctx.world,
            ctx.me,
            EVADE_RESPAWN_SECONDS,
        );
        instance_script::set_boss_state(ctx.world, map, BOSS_ID, EncounterState::Fail);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}
