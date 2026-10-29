//! MAGMATUSK_LITTLEWIGS_ALERT_POLICY: request sourced alert lanes using native spells.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

pub(super) const SCRIPT: &str = "boss_magmatusk";
const SPRAY: u32 = 375251;
const CHARGE: u32 = 375436;
const MUTATION: u32 = 374365;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    if map.map_id != 2519 || !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205) {
        return None;
    }
    Some(Box::new(Magmatusk {
        boss: super::instance_script::bound_boss(world, actor)?,
        sprays: 0,
        charges: 0,
    }))
}

struct Magmatusk {
    boss: BossAi,
    sprays: u32,
    charges: u32,
}

impl CreatureAi for Magmatusk {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.sprays = 0;
        self.charges = 0;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.sprays = 0;
        self.charges = 0;
        for (spell, delay) in [(SPRAY, 7_200), (CHARGE, 19_100), (MUTATION, 30_300)] {
            self.boss.events.schedule(spell, delay);
        }
    }

    fn on_spell_cast(&mut self, _ctx: &mut Ctx, spell: u32) {
        if spell == CHARGE {
            self.charges += 1;
            self.boss
                .events
                .schedule(CHARGE, if self.charges == 1 { 23_900 } else { 26_700 });
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Magmatusk {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let target = if event == SPRAY { ctx.victim() } else { None };
        if ctx.do_cast_at(target, event, false) {
            match event {
                SPRAY => {
                    self.sprays += 1;
                    self.boss
                        .events
                        .schedule(SPRAY, if self.sprays == 1 { 29_100 } else { 19_400 });
                }
                MUTATION => self.boss.events.schedule(MUTATION, 32_500),
                _ => {}
            }
        }
    }
}
