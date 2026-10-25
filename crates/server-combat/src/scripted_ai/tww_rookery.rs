//! LittleWigs2777a9d5 WarWithin/TheRookery/Kyrioss.lua:35–53,87–124.
//! Explicit boss-mod-derived combat schedule, not a nonexistent TC Rookery port.
//! DB270448 owns targets/cast times/effects. Dummy children, Dash controller and
//! Torrent's444034-driven phase/repeat remain unimplemented source boundaries.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

const KYRIOSS: u32 = 209230;
pub(super) const KYRIOSS_SCRIPT: &str = "littlewigs_kyrioss";
const CRASHING_THUNDER: u32 = 1214325;
const WILD_LIGHTNING: u32 = 474018;
const LIGHTNING_TORRENT_START: u32 = 1214315;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = world.get::<crate::maps::InMap>(actor)?;
    let npc = world.get::<shared::components::Npc>(actor)?;
    if map.map_id != 2648 || npc.template_id != KYRIOSS {
        return None;
    }
    Some(Box::new(Kyrioss {
        boss: super::instance_script::bound_boss(world, actor)?,
        thunder_count: 0,
        wild_count: 0,
    }))
}

struct Kyrioss {
    boss: BossAi,
    thunder_count: u32,
    wild_count: u32,
}

fn next_repeat(count: u32, first_repeat: u64) -> u64 {
    match count {
        1 => first_repeat,
        count if count % 2 == 0 => 15_800,
        _ => 40_000,
    }
}

impl BossScript for Kyrioss {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, spell_id: u32) {
        ctx.do_cast_self(spell_id);
        // Lua handlers repeat from actual SPELL_CAST_START, not native completion
        // or rejected admission. All three sourced parents have native cast time.
        let started = ctx
            .world
            .get::<shared::casting::CastState>(ctx.me)
            .is_some_and(|cast| cast.spell_id == spell_id);
        if !started {
            return;
        }
        let repeat_ms = match spell_id {
            CRASHING_THUNDER => {
                self.thunder_count += 1;
                next_repeat(self.thunder_count, 42_500)
            }
            WILD_LIGHTNING => {
                self.wild_count += 1;
                next_repeat(self.wild_count, 41_300)
            }
            _ => return,
        };
        self.boss.events.schedule(spell_id, repeat_ms);
    }
}

impl CreatureAi for Kyrioss {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.thunder_count = 0;
        self.wild_count = 0;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(CRASHING_THUNDER, 5_100);
        self.boss.events.schedule(WILD_LIGHTNING, 9_400);
        // Lua's bar444123 is a label; OnBossEnable names the actual cast1214315.
        self.boss.events.schedule(LIGHTNING_TORRENT_START, 16_000);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}
