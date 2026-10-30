//! Explicitly partial Lua-clock port: LittleWigs2777a9d Umbrelskul.lua +70448.
//! Lua energy clocks request Arcane Eruption/Unleashed Destruction; Lua's Brittle
//! messages (100 - n*25 percent) select the health thresholds.
//! UMBRELSKUL_BRITTLE_DUMMY_CHILD_UNSOURCED: 386746's effect0 dummy has no source
//! script, so no crystal is summoned from its base points.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_umbrelskul";
const DRAGON_STRIKE: u32 = 384978;
const CRYSTALLINE_ROAR: u32 = 384699;
const UNLEASHED_DESTRUCTION: u32 = 385399;
const MYTHIC_UNLEASHED_DESTRUCTION: u32 = 388804;
const ARCANE_ERUPTION: u32 = 385075;
const BRITTLE: u32 = 386746;
/// Lua `100 - brittleCount * 25` messages.
const BRITTLE_THRESHOLDS: [f32; 3] = [75.0, 50.0, 25.0];
const MYTHIC_KEYSTONE: u32 = 8;
const MYTHIC_DUNGEON: u32 = 23;

pub(super) fn creature_ai(boss: BossAi) -> Box<dyn CreatureAi> {
    Box::new(Umbrelskul {
        boss,
        strike_repeat_ms: 0,
        roar_repeat_ms: 0,
        destruction_repeat_ms: 0,
        brittle_count: 0,
    })
}

struct Umbrelskul {
    boss: BossAi,
    strike_repeat_ms: u64,
    roar_repeat_ms: u64,
    destruction_repeat_ms: u64,
    brittle_count: usize,
}

impl CreatureAi for Umbrelskul {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        let map = ctx.map().expect("Umbrelskul instance");
        let difficulty = crate::zones::map_difficulty_id(ctx.world, map);
        let mythic = matches!(difficulty, MYTHIC_KEYSTONE | MYTHIC_DUNGEON);
        self.strike_repeat_ms = if mythic { 15_300 } else { 8_400 };
        self.roar_repeat_ms = if mythic { 105_600 } else { 23_100 };
        self.destruction_repeat_ms = if mythic { 114_100 } else { 54_700 };
        self.brittle_count = 0;
        // Lua: 100 energy after 27.4s gain + 0.8s delay.
        self.boss.events.schedule(ARCANE_ERUPTION, 28_200);
        self.boss.events.schedule(DRAGON_STRIKE, 7_400);
        self.boss.events.schedule(CRYSTALLINE_ROAR, 12_200);
        self.boss.events.schedule(
            if mythic {
                MYTHIC_UNLEASHED_DESTRUCTION
            } else {
                UNLEASHED_DESTRUCTION
            },
            54_800,
        );
    }

    fn on_spell_cast(&mut self, _ctx: &mut Ctx, spell_id: u32) {
        if spell_id == DRAGON_STRIKE {
            // Lua starts this countdown only on SPELL_CAST_SUCCESS. Native
            // interrupted casts do not emit this hook or invent a retry.
            self.boss
                .events
                .reschedule(DRAGON_STRIKE, self.strike_repeat_ms);
        }
    }

    fn damage_taken(
        &mut self,
        ctx: &mut Ctx,
        _attacker: Option<Entity>,
        damage: super::DamageTaken,
    ) {
        while let Some(&threshold) = BRITTLE_THRESHOLDS.get(self.brittle_count) {
            if !damage.health_below_pct_damaged(threshold) {
                break;
            }
            self.brittle_count += 1;
            self.cast_brittle(ctx);
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Umbrelskul {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            DRAGON_STRIKE => ctx.do_cast_victim(DRAGON_STRIKE),
            UNLEASHED_DESTRUCTION | MYTHIC_UNLEASHED_DESTRUCTION => {
                // Lua restarts this countdown at cast start.
                ctx.do_cast_self(event);
                self.boss.events.repeat(self.destruction_repeat_ms);
            }
            ARCANE_ERUPTION => {
                ctx.do_cast_self(ARCANE_ERUPTION);
                self.boss.events.repeat(54_700);
            }
            CRYSTALLINE_ROAR => {
                // Partial direct384699 request from Lua's ability countdown;
                // upstream384696 dummy-selector scripting remains unsourced.
                ctx.do_cast_victim(CRYSTALLINE_ROAR);
                self.boss.events.repeat(self.roar_repeat_ms);
            }
            _ => {}
        }
    }
}

impl Umbrelskul {
    /// Lua: Brittle interrupts the current cast. Arcane Eruption and Unleashed
    /// Destruction are not recast; Dragon Strike is.
    fn cast_brittle(&mut self, ctx: &mut Ctx) {
        let interrupted = ctx
            .world
            .get::<shared::casting::CastState>(ctx.me)
            .map(|cast| cast.spell_id);
        crate::spell_cast::cancel_cast(ctx.world, ctx.me);
        if interrupted == Some(DRAGON_STRIKE) {
            self.boss.events.reschedule(DRAGON_STRIKE, 0);
        }
        ctx.do_cast_at(Some(ctx.me), BRITTLE, true);
    }
}
