//! Partial Azureblade Lua-clock port, explicitly approved for the follower dungeon.
//! LittleWigs2777a9d Dragonflight/TheAzureVault/Azureblade.lua supplies first-phase
//! request clocks/counts only. Native70448 supplies cast times, targets and children.
//! AZUREBLADE_ENERGY_PHASE_UNSOURCED: no invented center/energy/illusion phase.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx, EventMap};

/// Local Lua-backed binding; TDB1210 has no Azureblade C++ script.
pub(super) const SCRIPT: &str = "boss_azureblade";
const ARCANE_CLEAVE: u32 = 372222;
const SUMMON_DRACONIC_IMAGE: u32 = 384223;
const ANCIENT_ORB: u32 = 385578;

pub(super) fn creature_ai(boss: BossAi) -> Box<dyn CreatureAi> {
    Box::new(Azureblade {
        boss,
        images_remaining: 0,
        cleaves_remaining: 0,
        orbs_remaining: 0,
    })
}

struct Azureblade {
    boss: BossAi,
    images_remaining: u8,
    cleaves_remaining: u8,
    orbs_remaining: u8,
}

/// Lua decrements its ability count on each cast start and stops its last bar.
fn repeat_lua_event(events: &mut EventMap, remaining: &mut u8, interval_ms: u64) {
    *remaining -= 1;
    if *remaining > 0 {
        events.repeat(interval_ms);
    }
}

impl CreatureAi for Azureblade {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.images_remaining = 2;
        self.cleaves_remaining = 2;
        self.orbs_remaining = 2;
        self.boss.events.schedule(SUMMON_DRACONIC_IMAGE, 3_300);
        self.boss.events.schedule(ARCANE_CLEAVE, 5_200);
        self.boss.events.schedule(ANCIENT_ORB, 10_600);
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

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Azureblade {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            SUMMON_DRACONIC_IMAGE => {
                // Native self140 ->373658 missile32 ->373582 summon28/190187.
                ctx.do_cast_self(SUMMON_DRACONIC_IMAGE);
                repeat_lua_event(&mut self.boss.events, &mut self.images_remaining, 14_600);
            }
            ARCANE_CLEAVE => {
                // Native target104 is a cone toward the selected destination.
                ctx.do_cast_victim(ARCANE_CLEAVE);
                repeat_lua_event(&mut self.boss.events, &mut self.cleaves_remaining, 13_400);
            }
            ANCIENT_ORB => {
                // Native explicit enemy6 plus destination87/caster-front47.
                ctx.do_cast_victim(ANCIENT_ORB);
                repeat_lua_event(&mut self.boss.events, &mut self.orbs_remaining, 15_800);
            }
            _ => {}
        }
    }
}
