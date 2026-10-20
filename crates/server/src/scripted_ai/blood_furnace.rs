//! Blood Furnace bosses, TC a352b1fa boss_the_maker.cpp.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

const INSTANCE: &str = "instance_blood_furnace";
const MAKER: &str = "boss_the_maker";
/// TC DATA_THE_MAKER: installed DB2 encounter1922, completion bit0.
const DATA_THE_MAKER: u32 = 0;
const EXPLODING_BEAKER: u32 = 30925;
const EXPLODING_BEAKER_H: u32 = 40059;

pub(super) fn is_creature_script(name: &str) -> bool {
    name == MAKER
}

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = world.get::<crate::maps::InMap>(actor)?;
    let instance = world
        .get_resource::<crate::maps::MapStore>()?
        .instance_scripts
        .get(&map.map_id)?;
    (instance == INSTANCE).then(|| {
        Box::new(TheMaker {
            boss: BossAi::new(DATA_THE_MAKER),
        }) as Box<dyn CreatureAi>
    })
}

/// `MAKER_DOMINATION_CHARM_UNIMPLEMENTED`: Domination30923 needs player charm
/// (aura6) and is not scheduled.
struct TheMaker {
    boss: BossAi,
}

impl CreatureAi for TheMaker {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(0);
        self.boss
            .events
            .schedule_range(EXPLODING_BEAKER, 5_000, 15_000);
    }

    fn killed_unit(&mut self, ctx: &mut Ctx, victim: Entity) {
        if ctx
            .world
            .get::<shared::components::Player>(victim)
            .is_some()
        {
            ctx.talk(1);
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        ctx.talk(2);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for TheMaker {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        if event == EXPLODING_BEAKER {
            // MAKER_NORMAL_BEAKER_30925_ABSENT_70448: Normal resolves no SpellInfo.
            ctx.do_cast_victim(ctx.dungeon_mode(EXPLODING_BEAKER, EXPLODING_BEAKER_H));
            self.boss.events.repeat_range(10_000, 20_000);
        }
    }
}
