//! LittleWigs2777a9d Benk clocks; native70448 owns aura/area/channel effects.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "littlewigs_benk";
const MAP: u32 = 2661;
const ENTRY: u32 = 218002;
const SNACK: u32 = 438025;
const HONEY: u32 = 440134;
const WING: u32 = 439524;
const SNACK_FIRST_MS: u64 = 3_000;
const SNACK_REPEAT_MS: u64 = 33_000;
const HONEY_FIRST_MS: u64 = 10_000;
const HONEY_REPEAT_MS: u64 = 14_000;
const WING_FIRST_MS: u64 = 22_000;
const WING_REPEAT_MS: u64 = 23_000;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    let npc = world.get::<shared::components::Npc>(actor)?;
    if map.map_id != MAP
        || npc.template_id != ENTRY
        || !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205)
    {
        return None;
    }
    Some(Box::new(Benk {
        boss: super::instance_script::bound_boss(world, actor)?,
    }))
}

struct Benk {
    boss: BossAi,
}

impl BossScript for Benk {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, spell: u32) {
        let (target, repeat_ms) = match spell {
            SNACK => (Some(ctx.me), SNACK_REPEAT_MS),
            HONEY => (ctx.victim(), HONEY_REPEAT_MS),
            WING => (Some(ctx.me), WING_REPEAT_MS),
            _ => return,
        };
        // BENK_SNACK_BEE_CONTROLLER_UNSOURCED and
        // BENK_HONEY_DUMMY_CHILD_UNSOURCED: no fabricated bees, destinations,
        // dummy forwarding or damage. Native Wing channel remains ordinary.
        if ctx.do_cast_at(target, spell, false) {
            self.boss.events.schedule(spell, repeat_ms);
        }
    }
}

impl CreatureAi for Benk {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(SNACK, SNACK_FIRST_MS);
        self.boss.events.schedule(HONEY, HONEY_FIRST_MS);
        self.boss.events.schedule(WING, WING_FIRST_MS);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}
