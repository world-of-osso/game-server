//! Explicitly partial Current Retail LittleWigs2777a9d clocks, native70448 effects.
//! Source immunity stays intact: Ancient/Crawth unlock ingress is not authored.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const ANCIENT_SCRIPT: &str = "boss_overgrown_ancient";
const BARKBREAKER: u32 = 388544;
const GERMINATE: u32 = 388796;
const BRANCH_OUT: u32 = 388623;
const BURST_FORTH: u32 = 388923;

pub(super) fn is_creature_script(name: &str) -> bool {
    matches!(name, ANCIENT_SCRIPT | super::doragosa::SCRIPT)
}

pub(super) fn creature_ai(world: &World, actor: Entity, name: &str) -> Option<Box<dyn CreatureAi>> {
    if world.get::<crate::maps::InMap>(actor)?.map_id != 2526 {
        return None;
    }
    let boss = super::instance_script::bound_boss(world, actor)?;
    if name == super::doragosa::SCRIPT {
        return Some(super::doragosa::creature_ai(boss));
    }
    Some(Box::new(Ancient {
        boss,
        bark_repeat: 0,
    }))
}

struct Ancient {
    boss: BossAi,
    bark_repeat: usize,
}

impl CreatureAi for Ancient {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.bark_repeat = 0;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.bark_repeat = 0;
        self.boss.events.schedule(BARKBREAKER, 9_000);
        self.boss.events.schedule(GERMINATE, 18_000);
        self.boss.events.schedule(BRANCH_OUT, 30_000);
        self.boss.events.schedule(BURST_FORTH, 54_000);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Ancient {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            BARKBREAKER => {
                ctx.do_cast_victim(BARKBREAKER);
                // Current Lua duration9 initial, then28/29 in listed order.
                // Timeline cancellation/random-Normal context remains a named gap.
                let repeats = [28_000, 29_000];
                self.boss.events.repeat(repeats[self.bark_repeat]);
                self.bark_repeat = (self.bark_repeat + 1) % repeats.len();
            }
            GERMINATE => {
                ctx.do_cast_self(GERMINATE);
                self.boss.events.repeat(33_000);
            }
            BRANCH_OUT => {
                // Native dummy388623 has no child: no invented Ancient Branch.
                ctx.do_cast_self(BRANCH_OUT);
                self.boss.events.repeat(30_000);
            }
            BURST_FORTH => {
                ctx.do_cast_self(BURST_FORTH);
                self.boss.events.repeat(55_000);
            }
            _ => {}
        }
    }
}
