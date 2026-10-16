//! Omor's pinned TC a352b1fa EventMap; orbital spell hooks are separate.
use bevy::prelude::*;

use super::super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::super::{CreatureAi, Ctx, DamageTaken, model_reach};

const SHADOW_BOLT: u32 = 30686;
const TREACHEROUS_AURA: u32 = 30695;
const SUMMON_HOUND: u32 = 30707;
const ORBITAL_TARGET: u32 = 35642;
const DEMONIC_SHIELD: u32 = 31901;
const NOMINAL_MELEE_RANGE: f32 = 5.0;
const MELEE_REACH_EXTENSION: f32 = 4.0 / 3.0;

pub(super) fn creature_ai() -> Box<dyn CreatureAi> {
    Box::new(Omor {
        // TC ordinal1 is installed DB2 encounter1891/bit0.
        boss: BossAi::new(0),
        shield_used: false,
    })
}

struct Omor {
    boss: BossAi,
    shield_used: bool,
}

/// Unit.cpp691–709: exact3D distance, source reaches, nominal floor and aura153.
fn within_melee_range(ctx: &Ctx, victim: Entity) -> Option<bool> {
    let origin = crate::spell_cast::targets::position(ctx.world, ctx.me)?;
    let target = crate::spell_cast::targets::position(ctx.world, victim)?;
    let reaches = model_reach::combat_reach(ctx.world, ctx.me)?
        + model_reach::combat_reach(ctx.world, victim)?;
    let bonus = ctx
        .world
        .get::<crate::auras::stat_mods::AuraStatMods>(ctx.me)
        .map_or(0.0, |mods| mods.autoattack_range_bonus);
    let range = (reaches + MELEE_REACH_EXTENSION).max(NOMINAL_MELEE_RANGE) + bonus;
    Some(origin.distance(target) <= range)
}

impl CreatureAi for Omor {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.shield_used = false;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(2);
        self.boss.events.schedule(SHADOW_BOLT, 0);
        self.boss
            .events
            .schedule_range(TREACHEROUS_AURA, 16_000, 23_000);
        self.boss
            .events
            .schedule_range(SUMMON_HOUND, 20_000, 23_000);
        self.boss.events.schedule(ORBITAL_TARGET, 25_000);
        // Source AttackStartCaster(100yd) needs a caster-chase range hook.
    }

    fn damage_taken(&mut self, _ctx: &mut Ctx, _attacker: Option<Entity>, damage: DamageTaken) {
        if !self.shield_used && damage.health_below_pct_damaged(20.0) {
            self.shield_used = true;
            self.boss.events.schedule(DEMONIC_SHIELD, 0);
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        ctx.talk(4);
    }

    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        // Source deliberately does not add hounds to BossAI's despawn list.
        if ctx.is_engaged() {
            ctx.do_zone_in_combat(summon);
        }
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl Omor {
    fn cast_shadow_bolt(&mut self, ctx: &mut Ctx) {
        let Some(victim) = ctx.victim() else {
            return;
        };
        let Some(melee) = within_melee_range(ctx, victim) else {
            error!("Omor source melee predicate missing actor/target model geometry");
            return;
        };
        ctx.do_cast_at(Some(victim), SHADOW_BOLT, false);
        let (min, max) = if melee {
            (6_000, 10_000)
        } else {
            (3_000, 4_000)
        };
        self.boss.events.repeat_range(min, max);
    }
}

impl BossScript for Omor {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            SHADOW_BOLT => self.cast_shadow_bolt(ctx),
            TREACHEROUS_AURA => {
                if let Some(target) = ctx.select_target_random(0.0, false) {
                    ctx.do_cast_at(Some(target), TREACHEROUS_AURA, false);
                }
                self.boss.events.repeat_range(30_000, 40_000);
            }
            SUMMON_HOUND => {
                ctx.do_cast_self(SUMMON_HOUND);
                self.boss.events.repeat_range(20_000, 26_000);
            }
            ORBITAL_TARGET => {
                ctx.do_cast_self(ORBITAL_TARGET);
                self.boss.events.repeat_range(23_000, 30_000);
            }
            DEMONIC_SHIELD => ctx.do_cast_self(DEMONIC_SHIELD),
            _ => {}
        }
    }
}
