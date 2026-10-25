//! TC a352b1fa KhazAlgar/DarkflameCleft/boss_the_candle_king.cpp:85–222.
//! Normal/Follower parent requests only; native70448 owns targets and effects.
//! Scripted selectors/children, wax actors and area controllers remain unported.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_the_candle_king";
const CANDLE_KING: u32 = 208745;
const DARKLIGHT: u32 = 426152;
const MOLDS: u32 = 420659;
const MIND: u32 = 426145;
const PICKAXE: u32 = 421274;
const THROW: u32 = 420696;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = world.get::<crate::maps::InMap>(actor)?;
    let npc = world.get::<shared::components::Npc>(actor)?;
    if map.map_id != 2651 || npc.template_id != CANDLE_KING {
        return None;
    }
    let difficulty = crate::zones::map_difficulty_id(world, *map);
    if !matches!(difficulty, 1 | 205) {
        return None;
    }
    Some(Box::new(CandleKing {
        boss: super::instance_script::bound_boss(world, actor)?,
    }))
}

struct CandleKing {
    boss: BossAi,
}

fn remaining_cooldown_ms(ctx: &Ctx, spell_id: u32) -> u64 {
    let Some(spell) = ctx
        .world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
    else {
        return 0; // TC GetCooldownOfSpell: absent native data still goes through ordinary DoCast.
    };
    let now = crate::spell_cast::history_now(ctx.world);
    ctx.world
        .get::<crate::spell_cast::SpellHistory>(ctx.me)
        .map_or(0, |history| history.remaining_cooldown_ms(spell, now))
}

impl BossScript for CandleKing {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, spell: u32) {
        let cooldown = remaining_cooldown_ms(ctx, spell);
        if cooldown > 0 {
            self.boss.events.schedule(spell, cooldown);
            return;
        }
        // TC TryCastSpellIfReady repeats after DoCastSelf even if normal admission fails.
        ctx.do_cast_self(spell);
        let repeat = match spell {
            MOLDS => 31_500,
            MIND => 20_600,
            PICKAXE | THROW => 17_000,
            _ => return,
        };
        self.boss.events.schedule(spell, repeat);
    }
}

impl CreatureAi for CandleKing {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(0);
        ctx.do_cast_self(DARKLIGHT);
        for (spell, first) in [
            (MOLDS, 6_000),
            (MIND, 10_500),
            (PICKAXE, 13_200),
            (THROW, 19_300),
        ] {
            self.boss.events.schedule(spell, first);
        }
    }

    fn killed_unit(&mut self, ctx: &mut Ctx, victim: Entity) {
        if ctx
            .world
            .get::<shared::components::Player>(victim)
            .is_some()
        {
            ctx.talk(4);
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        ctx.talk(5);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if !ctx.update_victim() {
            return;
        }
        self.boss.events.update(u64::from(diff_ms));
        if ctx.is_casting() {
            return;
        }
        // TC checks casting before its while loop, not between due events.
        while let Some(spell) = self.boss.events.execute_event() {
            self.execute_event(ctx, spell);
        }
    }
}
