//! LittleWigs2777a9d TheLostDwarves.lua + native70448.
//! LOST_DWARVES_SURRENDER_AT_10PCT_LUA is explicit project completion policy.
use bevy::prelude::*;
use shared::components::{Health, Npc, ThreatTable};

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};
use crate::maps::InMap;

pub(super) const SCRIPT: &str = "boss_lost_dwarves";
const BAELOG: u32 = 184581;
const ERIC: u32 = 184582;
const OLAF: u32 = 184580;
const HEAVY_ARROW: u32 = 369573;
const WILD_CLEAVE: u32 = 369563;
const SKULLCRACKER: u32 = 369791;
const SHIELD: u32 = 369677;
const BULWARK: u32 = 369602;
const LONGBOAT: u32 = 375924;
const SURRENDER_FRACTION: f32 = 0.1;

#[derive(Component)]
struct Surrendered {
    reset_pending: bool,
}

pub(super) fn limit_loss(health: &Health, loss: f32) -> f32 {
    let surrender_health = (health.max * SURRENDER_FRACTION).floor();
    loss.min((health.current - surrender_health).max(0.0))
}

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    if world.get::<InMap>(actor)?.map_id != 2451 {
        return None;
    }
    let entry = world.get::<Npc>(actor)?.template_id;
    if ![BAELOG, ERIC, OLAF].contains(&entry) {
        return None;
    }
    Some(Box::new(Dwarf {
        boss: super::instance_script::bound_boss(world, actor)?,
        entry,
        primary_repeat: 0,
        secondary_repeat: 0,
    }))
}

struct Dwarf {
    boss: BossAi,
    entry: u32,
    primary_repeat: usize,
    secondary_repeat: usize,
}

impl CreatureAi for Dwarf {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.events.reset();
        if ctx.world.get::<Surrendered>(ctx.me).is_some() {
            return;
        }
        reset_surrendered_peers(ctx);
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.primary_repeat = 0;
        self.secondary_repeat = 0;
        match self.entry {
            BAELOG => {
                self.boss.events.schedule(WILD_CLEAVE, 8_500);
                self.boss.events.schedule(HEAVY_ARROW, 20_600);
            }
            ERIC => self.boss.events.schedule(SKULLCRACKER, 6_100),
            OLAF => {
                self.boss.events.schedule(SHIELD, 12_100);
                self.boss.events.schedule(BULWARK, 17_200);
            }
            _ => unreachable!(),
        }
        self.boss.events.schedule(LONGBOAT, 24_800);
    }

    fn just_died(&mut self, _ctx: &mut Ctx, _killer: Option<Entity>) {
        // Forced administrative death is not the authored surrender condition.
        self.boss.events.reset();
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if update_surrender_state(ctx) {
            return;
        }
        let at_threshold = ctx.world.get::<Health>(ctx.me).is_some_and(|health| {
            health.current > 0.0 && health.current <= health.max * SURRENDER_FRACTION
        });
        // A wipe restores peers while native MoveTargetedHome still owns them.
        // Their old low health is not a new in-combat surrender condition.
        if ctx.is_engaged() && at_threshold {
            self.boss.events.reset();
            surrender(ctx, self.boss.boss_id);
            return;
        }
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Dwarf {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        // Lua's phase-count/boat context is not executable here. Consume each
        // ability's listed repeat durations in order, without inventing phases.
        let repeat = match event {
            WILD_CLEAVE => next_repeat(&[17_000, 24_900], &mut self.primary_repeat),
            HEAVY_ARROW => next_repeat(&[20_600, 35_800], &mut self.secondary_repeat),
            SKULLCRACKER => next_repeat(&[26_600, 24_900], &mut self.primary_repeat),
            SHIELD => next_repeat(&[17_000, 29_800], &mut self.primary_repeat),
            BULWARK => next_repeat(&[33_900, 34_700], &mut self.secondary_repeat),
            LONGBOAT => 78_800,
            _ => return,
        };
        if matches!(event, BULWARK | LONGBOAT) {
            ctx.do_cast_self(event);
        } else {
            ctx.do_cast_victim(event);
        }
        self.boss.events.repeat(repeat);
    }
}

fn next_repeat(durations: &[u64], index: &mut usize) -> u64 {
    let duration = durations[*index];
    *index = (*index + 1) % durations.len();
    duration
}

fn surrender(ctx: &mut Ctx, boss_id: u32) {
    let map = ctx.map().expect("Lost Dwarves instance");
    ctx.world.entity_mut(ctx.me).insert(Surrendered {
        reset_pending: false,
    });
    crate::creature_aggro::enter_evade_mode(ctx.world, ctx.me);
    // Stop re-engagement without inventing a friendly faction or immunity flags.
    ctx.world.entity_mut(ctx.me).remove::<ThreatTable>();
    let surrendered = ctx
        .world
        .query::<(&InMap, &Npc, &Surrendered)>()
        .iter(ctx.world)
        .filter(|(actor_map, _, surrendered)| **actor_map == map && !surrendered.reset_pending)
        .fold(0, |mask, (_, npc, _)| {
            mask | match npc.template_id {
                BAELOG => 1,
                ERIC => 2,
                OLAF => 4,
                _ => 0,
            }
        });
    if surrendered == 7 {
        super::instance_script::set_boss_state(
            ctx.world,
            map,
            boss_id,
            super::instance_script::EncounterState::Done,
        );
    }
}

fn reset_surrendered_peers(ctx: &mut Ctx) {
    let map = ctx.map().expect("Lost Dwarves instance");
    for (actor_map, mut surrendered) in ctx
        .world
        .query::<(&InMap, &mut Surrendered)>()
        .iter_mut(ctx.world)
    {
        if *actor_map == map {
            surrendered.reset_pending = true;
        }
    }
}

fn update_surrender_state(ctx: &mut Ctx) -> bool {
    let Some(surrendered) = ctx.world.get::<Surrendered>(ctx.me) else {
        return false;
    };
    let recovered = ctx
        .world
        .get::<Health>(ctx.me)
        .is_some_and(|health| health.current > health.max * SURRENDER_FRACTION);
    if surrendered.reset_pending && recovered {
        // Native home/regen must cross10% before sight-aggro is restored;
        // otherwise it re-surrenders before the first out-of-combat regen tick.
        ctx.world
            .entity_mut(ctx.me)
            .remove::<Surrendered>()
            .insert(ThreatTable::default());
    }
    true
}
