//! LittleWigs2777a9d Dailcry clocks; native70448 owns targets/effects.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "littlewigs_dailcry";
const MAP: u32 = 2649;
const ENTRY: u32 = 207946;
const PIERCE: u32 = 424414;
const SPEAR: u32 = 1238780;
const BATTLE_CRY: u32 = 424419;
const MAULING: u32 = 447439;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    let npc = world.get::<shared::components::Npc>(actor)?;
    if map.map_id != MAP
        || npc.template_id != ENTRY
        || !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205)
    {
        return None;
    }
    Some(Box::new(Dailcry {
        boss: super::instance_script::bound_boss(world, actor)?,
    }))
}

struct Dailcry {
    boss: BossAi,
}

impl BossScript for Dailcry {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, spell: u32) {
        let (target, repeat_ms) = match spell {
            PIERCE => (ctx.victim(), Some(12_100)),
            SPEAR => (Some(ctx.me), Some(25_500)),
            BATTLE_CRY => (Some(ctx.me), Some(26_700)),
            MAULING => (ctx.victim(), None),
            _ => return,
        };
        // DAILCRY_MAULING_ENERGY_SHIELD_CONTROLLER_UNSOURCED: initial13s
        // is not a repeated energy/shield controller. Native Battle Cry owns
        // its50Energy; do not grant power or invent447443 application/removal.
        // DAILCRY_SPEAR_DESTINATION_SELECTOR_UNSOURCED: native ground default.
        // DAILCRY_MINIBOSS_ENCOUNTER_CONTROLLER_UNSOURCED: no fabricated adds.
        if ctx.do_cast_at(target, spell, false) {
            if let Some(repeat_ms) = repeat_ms {
                self.boss.events.schedule(spell, repeat_ms);
            }
        }
    }
}

impl CreatureAi for Dailcry {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(PIERCE, 5_200);
        self.boss.events.schedule(SPEAR, 9_300);
        self.boss.events.schedule(BATTLE_CRY, 12_000);
        self.boss.events.schedule(MAULING, 13_000);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}
