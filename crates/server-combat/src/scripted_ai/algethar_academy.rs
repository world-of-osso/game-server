//! Current Retail LittleWigs2777a9d clocks, native70448 effects.
//! Academy unlocks are log-correlated project policy, not sourced scripts.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};

#[cfg(test)]
#[path = "algethar_unlock_tests.rs"]
mod unlock_tests;

pub(super) const ANCIENT_SCRIPT: &str = "boss_overgrown_ancient";
const BARKBREAKER: u32 = 388544;
const GERMINATE: u32 = 388796;
const BRANCH_OUT: u32 = 388623;
const BURST_FORTH: u32 = 388923;

#[derive(Component)]
pub(super) struct CrawthDescent;

const MAP: u32 = 2526;

/// ANCIENT/CRAWTH_UNLOCK_LOG_CORRELATED: (gate entry, boss entry, intro centre, radius).
/// WCL deaths correlate each intro with the last member of its local pack.
/// Positions are Bevy coordinates.
const INTRO_UNLOCKS: [(u32, u32, Vec2, f32); 2] = [
    (197219, 196482, Vec2::new(1766.25, 3086.41), 80.0),
    (192333, 191736, Vec2::new(2365.0, 3050.0), 40.0),
];

/// Gate entries whose intro deaths unlock each boss entry on `map`; read by the route driver.
#[cfg(any(test, feature = "test-support"))]
pub(crate) fn intro_gates(map: u32) -> Vec<(u32, u32)> {
    if map != MAP {
        return Vec::new();
    }
    INTRO_UNLOCKS
        .iter()
        .map(|(gate, boss, _, _)| (*boss, *gate))
        .collect()
}

/// The Crawth landing is the first cited boss cast position, projected onto
/// the native detail mesh, not a logged landing.
pub(crate) fn on_unit_death(world: &mut World, dead: Entity) {
    use shared::components::Npc;
    let Some(npc) = world.get::<Npc>(dead) else {
        return;
    };
    let Some(&(_, boss_entry, center, radius)) = INTRO_UNLOCKS
        .iter()
        .find(|(gate, ..)| *gate == npc.template_id)
    else {
        return;
    };
    let entry = npc.template_id;
    let Some(map) = world.get::<crate::maps::InMap>(dead).copied() else {
        return;
    };
    if map.map_id != MAP || !in_intro_area(world, dead, center, radius) {
        return;
    }
    let living = world
        .query::<(
            Entity,
            &Npc,
            &crate::maps::InMap,
            Option<&crate::combat::DeathState>,
        )>()
        .iter(world)
        .any(|(entity, npc, current, death)| {
            npc.template_id == entry
                && *current == map
                && death.is_none()
                && in_intro_area(world, entity, center, radius)
        });
    if living {
        return;
    }
    let boss = world
        .query::<(Entity, &Npc, &crate::maps::InMap)>()
        .iter(world)
        .find_map(|(entity, npc, current)| {
            (npc.template_id == boss_entry && *current == map).then_some(entity)
        });
    let Some(boss) = boss else { return };
    if world
        .get::<shared::components::UnitFlags>(boss)
        .is_none_or(|flags| flags.0 & shared::components::UnitFlags::IMMUNE_TO_PC == 0)
    {
        return;
    }
    if boss_entry == 191736 {
        if world.get::<CrawthDescent>(boss).is_some() {
            return;
        }
        world.entity_mut(boss).insert(CrawthDescent);
        super::jump_movement::move_jump(
            world,
            boss,
            0,
            shared::components::Position {
                x: 2369.31,
                y: 912.2291,
                z: 3041.47,
            },
            50.0,
            0.0,
        );
    } else {
        unlock_boss(world, boss);
    }
}

fn in_intro_area(world: &World, entity: Entity, center: Vec2, radius: f32) -> bool {
    world
        .get::<crate::creature_ai::SpawnPoint>(entity)
        .is_some_and(|spawn| Vec2::new(spawn.x, spawn.z).distance(center) <= radius)
}

pub(super) fn finish_crawth_descent(
    mut commands: Commands,
    landed: Query<
        (Entity, &shared::components::Position),
        (
            With<CrawthDescent>,
            Without<super::jump_movement::JumpMovement>,
        ),
    >,
) {
    for (entity, position) in &landed {
        let point = Vec3::new(position.x, position.y, position.z);
        if point.distance(Vec3::new(2369.31, 912.2291, 3041.47)) > 0.1 {
            error!("Crawth descent ended away from the logged arena position: {point:?}");
            continue;
        }
        commands.entity(entity).remove::<CrawthDescent>();
        commands.queue(move |world: &mut World| unlock_boss(world, entity));
    }
}

fn unlock_boss(world: &mut World, boss: Entity) {
    use shared::components::{ThreatTable, UnitFlags};
    let Some(mut flags) = world.get_mut::<UnitFlags>(boss) else {
        return;
    };
    if flags.0 & UnitFlags::IMMUNE_TO_PC == 0 {
        return;
    }
    flags.0 &= !(UnitFlags::IMMUNE_TO_PC | UnitFlags::IMMUNE_TO_NPC);
    world.entity_mut(boss).insert((
        ThreatTable::default(),
        crate::creature_ai::CreatureState::Idle,
        crate::creature_aggro::AggroRange,
    ));
    crate::spell_cast::mark_dirty(world, boss);
}

pub(super) fn is_creature_script(name: &str) -> bool {
    matches!(
        name,
        ANCIENT_SCRIPT | super::doragosa::SCRIPT | super::vexamus::SCRIPT
    )
}

pub(super) fn creature_ai(world: &World, actor: Entity, name: &str) -> Option<Box<dyn CreatureAi>> {
    if world.get::<crate::maps::InMap>(actor)?.map_id != 2526 {
        return None;
    }
    let boss = super::instance_script::bound_boss(world, actor)?;
    if name == super::vexamus::SCRIPT {
        return Some(super::vexamus::creature_ai(boss));
    }
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
