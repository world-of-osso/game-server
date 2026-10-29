//! The Darkness, Darkflame Cleft encounter 2788. TC a352b1fa has no boss script and
//! names 210797; WCL WBYDm1N6JTZzCFnd fight3 (pull16) shows 208747 fighting and
//! ENCOUNTER_END kill=true at 54.87% health with no death (LittleWigs: 55%).
//! DARKNESS_ACTIVATION_UNSOURCED: the actor stays absent until The Candle King (2787)
//! is DONE. This matches retail boss order behind the catacombs gate, not a source
//! activation event. Leaving the world after its 55% end is ours too.
//! DARKNESS_CANDLE_MECHANICS_GAP: candles, Massive Candle 212777 and the darkness
//! abilities are not ported; the actor fights with its native template only.
use bevy::prelude::*;

use super::boss_ai::BossAi;
use super::instance_script::{self, EncounterState};
use super::{CreatureAi, Ctx, DamageTaken};
use crate::maps::InMap;

const MAP: u32 = 2651;
const DARKNESS: u32 = 208747;
const CANDLE_KING: u32 = 208745;
const CANDLE_KING_ENCOUNTER: u32 = 2787;
/// WCL pull16 ENCOUNTER_END: 3548121701 / 6466574065 health.
const END_HEALTH_PCT: f32 = 55.0;

/// Copy fill: the actor waits for the Candle King's saved DONE state.
pub(crate) fn withheld_at_fill(content: &crate::zones::MapContent, entry: u32) -> bool {
    content.map.map_id == MAP && entry == DARKNESS && !content.dead_bosses.contains(&CANDLE_KING)
}

/// The Candle King's DONE transition brings the withheld actor into the copy.
pub(super) fn on_boss_state_changed(
    world: &mut World,
    map: InMap,
    encounter: u32,
    state: EncounterState,
) {
    if map.map_id != MAP || encounter != CANDLE_KING_ENCOUNTER || state != EncounterState::Done {
        return;
    }
    crate::zones::spawn_withheld_entry(world, map, DARKNESS);
}

pub(super) fn is_darkness(world: &World, actor: Entity) -> bool {
    world
        .get::<shared::components::Npc>(actor)
        .is_some_and(|npc| npc.template_id == DARKNESS)
        && world
            .get::<InMap>(actor)
            .is_some_and(|map| map.map_id == MAP)
}

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    Some(Box::new(Darkness {
        boss: instance_script::bound_boss(world, actor)?,
        ended: false,
    }))
}

struct Darkness {
    boss: BossAi,
    ended: bool,
}

impl Darkness {
    fn end_encounter(&mut self, ctx: &mut Ctx) {
        self.ended = true;
        let map = ctx.map().expect("Darkflame Cleft actor map");
        instance_script::set_boss_state(ctx.world, map, self.boss.boss_id, EncounterState::Done);
        instance_script::send_disengage_unit(ctx.world, map, ctx.me);
        let me = ctx.me;
        // Leave after the damage that crossed 55% has been applied.
        ctx.world.commands().queue(move |world: &mut World| {
            crate::combat::despawn_creature(world, me);
        });
    }
}

impl CreatureAi for Darkness {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
    }

    fn damage_taken(&mut self, ctx: &mut Ctx, _attacker: Option<Entity>, damage: DamageTaken) {
        if !self.ended && damage.health_below_pct_damaged(END_HEALTH_PCT) {
            self.end_encounter(ctx);
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }
}
