//! INFERRED_WCL parent-cast schedules for Temple keystone bosses.
//! Sources: M4cdBNJkTjavYgQV/11 pulls 2,4,7; ax1vNFnfPdXrzRQ2/16
//! pulls 2,4,6; ZgBCXQvcnhaAYT1r/5 pulls 2,5,7. Native DB2 owns effects.
//! The shield transfer, Merektha burrow/adds, Galvazzt conductors and
//! encounter-specific dummy spell children remain unported.
use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};
use bevy::prelude::*;
use shared::components::Npc;

const ADDERIS: u32 = 262530;
const ASPIX: u32 = 262822;
const MEREKTHA: u32 = 133384;
const GALVAZZT: u32 = 263658;
const GUST: u32 = 1288457;
const GALE_FORCE: u32 = 1289062;
const TEMPEST_WINDS: u32 = 1288864;
const THUNDER_LIGHTNING: u32 = 1288087;
const OVERLOAD: u32 = 1288428;
const LIGHTNING_BITE: u32 = 1290797;
const KNOT_OF_SNAKES: u32 = 1290031;
const THUNDER_SPIT: u32 = 1289602;
const SERPENTSTORM: u32 = 1293048;
const INDUCTION: u32 = 1290531;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    let entry = world.get::<Npc>(actor)?.template_id;
    if map.map_id != 1877
        || crate::zones::map_difficulty_id(world, map) != 8
        || !matches!(entry, ADDERIS | ASPIX | MEREKTHA | GALVAZZT)
    {
        return None;
    }
    Some(Box::new(TempleBoss {
        entry,
        boss: super::instance_script::bound_boss(world, actor)?,
    }))
}

struct TempleBoss {
    entry: u32,
    boss: BossAi,
}

impl BossScript for TempleBoss {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, spell: u32) {
        match spell {
            GUST => {
                if let Some(target) = ctx.select_target_random(0.0, false) {
                    ctx.do_cast_at(Some(target), spell, false);
                }
                self.boss.events.repeat(3_640);
            }
            LIGHTNING_BITE => ctx.do_cast_victim(spell),
            GALE_FORCE | TEMPEST_WINDS | THUNDER_LIGHTNING | OVERLOAD | KNOT_OF_SNAKES
            | THUNDER_SPIT | SERPENTSTORM => ctx.do_cast_self(spell),
            INDUCTION => {
                ctx.do_cast_self(spell);
                self.boss.events.repeat(23_100);
            }
            _ => {}
        }
    }
}

impl CreatureAi for TempleBoss {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }
    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        let schedule: &[(u32, u64)] = match self.entry {
            ADDERIS => &[(THUNDER_LIGHTNING, 9_470), (OVERLOAD, 43_440)],
            ASPIX => &[(GUST, 2_170), (GALE_FORCE, 5_820), (TEMPEST_WINDS, 30_090)],
            MEREKTHA => &[
                (LIGHTNING_BITE, 5_017),
                (KNOT_OF_SNAKES, 13_011),
                (THUNDER_SPIT, 25_008),
                (SERPENTSTORM, 36_020),
            ],
            GALVAZZT => &[(INDUCTION, 20_892)],
            _ => &[],
        };
        for &(spell, at) in schedule {
            self.boss.events.schedule(spell, at);
        }
    }
    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }
    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}
