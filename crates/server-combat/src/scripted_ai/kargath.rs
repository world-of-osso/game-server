//! Kargath/portal: TC a352b1fa boss_warchief_kargath_bladefist.cpp.
use bevy::prelude::*;
use rand::seq::SliceRandom;
use shared::components::{Npc, Position};

use super::boss_ai::BossAi;
use super::event_map::EventMap;
use super::{CreatureAi, Ctx, Hook};
use crate::maps::InMap;

const KARGATH: u32 = 16808;
const ASSASSIN: u32 = 17695;
const BLADE_DANCE_TARGETING: u32 = 30738;
const BLADE_DANCE: u32 = 30739;
const BLADE_DANCE_CHARGE: u32 = 30751;
const CHARGE_H: u32 = 25821;
const EVENT_BLADE_DANCE: u32 = 1;
const EVENT_CHARGE_H: u32 = 2;
const EVENT_SUMMON_PORTAL: u32 = 3;
const EVENT_SUMMON_ASSASSINS: u32 = 4;
const EVENT_LEASH: u32 = 5;
const MAX_DANCE_COUNT: u8 = 8;
const ASSASSIN_WANDER_RADIUS: f32 = 5.0;
const LEASH_DISTANCE: f32 = 40.0;
/// Installed1938/map540 completion bit3 (TC DATA_KARGATH).
const ENCOUNTER_BIT: u32 = 3;
const SUMMON_GROUP_PORTAL: u32 = 0;
const SUMMON_GROUP_ASSASSINS: u32 = 1;
const SUMMON_HEATHEN: u32 = 30737;
const SUMMON_REAVER: u32 = 30785;
const SUMMON_SHARPSHOOTER: u32 = 30786;

pub(super) fn is_creature_script(name: &str) -> bool {
    matches!(
        name,
        "boss_warchief_kargath_bladefist" | "npc_warchiefs_portal"
    )
}

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = world.get::<InMap>(actor)?;
    let script = world
        .get_resource::<crate::maps::MapStore>()?
        .instance_scripts
        .get(&map.map_id)?;
    if script != "instance_shattered_halls" {
        return None;
    }
    match world.get::<super::CreatureScriptName>(actor)?.0.as_str() {
        "boss_warchief_kargath_bladefist" => Some(Box::new(Kargath {
            boss: BossAi::new(ENCOUNTER_BIT),
            dance_count: 0,
        })),
        "npc_warchiefs_portal" => Some(Box::new(Portal::default())),
        _ => None,
    }
}

/// The literal source GUID list is world-accessible during synchronous spell hits.
#[derive(Component, Default)]
struct BladeDanceTargets(Vec<Entity>);

struct Kargath {
    boss: BossAi,
    dance_count: u8,
}

impl Kargath {
    fn blade_dance(&mut self, ctx: &mut Ctx) {
        if self.dance_count == MAX_DANCE_COUNT {
            let heroic = ctx
                .map()
                .is_some_and(|map| crate::zones::map_difficulty_id(ctx.world, map) == 2);
            if heroic {
                self.boss.events.schedule(EVENT_CHARGE_H, 500);
            }
            self.boss
                .events
                .schedule_range(EVENT_BLADE_DANCE, 45_000, 50_000);
            self.dance_count = 0;
            return;
        }
        Self::cast_dance_step(ctx);
        self.boss.events.schedule(EVENT_BLADE_DANCE, 500);
        self.dance_count += 1;
    }

    fn cast_dance_step(ctx: &mut Ctx) {
        ctx.world
            .get_mut::<BladeDanceTargets>(ctx.me)
            .expect("Kargath target list")
            .0
            .clear();
        #[cfg(test)]
        if ctx.is_casting() {
            // UnitAI::DoCast reports SPELL_FAILED_SPELL_IN_PROGRESS; Ctx short-circuits it.
            super::tests::cast_admission::record(
                ctx.world,
                ctx.me,
                BLADE_DANCE_TARGETING,
                &Err(crate::spell_cast::CastFailure::InProgress),
            );
        }
        ctx.do_cast_self(BLADE_DANCE_TARGETING);
        // KARGATH_SELECTOR_30738_NATIVE_CAST1000_SYNCHRONOUS_READ_GAP:
        // source reads immediately; never fake instant/triggered native targeting.
        let target = ctx
            .world
            .get::<BladeDanceTargets>(ctx.me)
            .expect("Kargath target list")
            .0
            .choose(&mut rand::thread_rng())
            .copied();
        if let Some(target) = target {
            ctx.do_cast_at(Some(target), BLADE_DANCE_CHARGE, false);
            ctx.do_cast_self(BLADE_DANCE);
        }
    }

    fn check_leash(&mut self, ctx: &mut Ctx) -> bool {
        let position = ctx.position().expect("Kargath position");
        let home = ctx
            .world
            .get::<crate::creature_ai::SpawnPoint>(ctx.me)
            .expect("Kargath home");
        let from_home = Vec3::new(
            position.x - home.x,
            position.y - home.y,
            position.z - home.z,
        )
        .length();
        let reach = super::model_reach::combat_reach(ctx.world, ctx.me)
            .expect("sourced Kargath combat reach");
        // Object.cpp436: GetDistance(Position) subtracts the caster's combat reach.
        if from_home - reach > LEASH_DISTANCE {
            ctx.talk(5);
            crate::creature_aggro::enter_evade_mode(ctx.world, ctx.me);
            return false;
        }
        self.boss.events.repeat(1_000);
        true
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) -> bool {
        match event {
            EVENT_BLADE_DANCE => self.blade_dance(ctx),
            EVENT_CHARGE_H => {
                if let Some(target) = ctx.select_target_random(0.0, false) {
                    ctx.do_cast_at(Some(target), CHARGE_H, false);
                }
            }
            EVENT_SUMMON_PORTAL => {
                ctx.summon_creature_group(SUMMON_GROUP_PORTAL);
            }
            EVENT_SUMMON_ASSASSINS => {
                ctx.summon_creature_group(SUMMON_GROUP_ASSASSINS);
            }
            EVENT_LEASH => return self.check_leash(ctx),
            _ => {}
        }
        true
    }
}

impl CreatureAi for Kargath {
    fn initialize_ai(&mut self, ctx: &mut Ctx) {
        ctx.world
            .entity_mut(ctx.me)
            .insert(BladeDanceTargets::default());
        super::dispatch(ctx.world, ctx.me, Hook::Reset);
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        ctx.world
            .get_mut::<BladeDanceTargets>(ctx.me)
            .expect("Kargath target list")
            .0
            .clear();
        self.dance_count = 0;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        ctx.talk(0);
        self.boss.just_engaged_with(ctx);
        #[cfg(test)]
        if let Some(seed) = ctx
            .world
            .get::<super::tests::cast_admission::GroundCombatSeed>(ctx.me)
        {
            self.boss.events.seed_rng(seed.0);
        }
        self.boss
            .events
            .schedule_range(EVENT_BLADE_DANCE, 30_000, 35_000);
        self.boss.events.schedule(EVENT_SUMMON_PORTAL, 1_000);
        self.boss.events.schedule(EVENT_SUMMON_ASSASSINS, 3_000);
        self.boss.events.schedule(EVENT_LEASH, 1_000);
    }

    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        if ctx
            .world
            .get::<Npc>(summon)
            .is_some_and(|npc| npc.template_id == ASSASSIN)
        {
            start_assassin_wander(ctx.world, summon);
        }
        // Source override intentionally does not engage these summons.
        self.boss.summons.summon(summon);
    }

    fn summoned_creature_despawn(&mut self, _ctx: &mut Ctx, summon: Entity) {
        self.boss.summons.despawn(summon);
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
        if !ctx.update_victim() {
            return;
        }
        self.boss.events.update(u64::from(diff_ms));
        // Unlike generic BossAI::UpdateAI, this exact source has no casting gate.
        while let Some(event) = self.boss.events.execute_event() {
            if !self.execute_event(ctx, event) {
                break;
            }
        }
    }
}

fn start_assassin_wander(world: &mut World, actor: Entity) {
    let position = *world.get::<Position>(actor).expect("assassin position");
    world
        .get_mut::<crate::creature_ai::SpawnPoint>(actor)
        .expect("assassin default movement")
        .wander_distance = ASSASSIN_WANDER_RADIUS;
    world
        .entity_mut(actor)
        .insert(crate::creature_ai::WanderState {
            phase: crate::creature_ai::WanderPhase::Idle,
            target_x: position.x,
            target_z: position.z,
            idle_remaining: 0.0,
            seed: rand::random(),
        });
}

#[derive(Default)]
struct Portal {
    tasks: EventMap,
    summon_index: usize,
}

impl CreatureAi for Portal {
    fn just_appeared(&mut self, _ctx: &mut Ctx) {
        self.summon_index = 0;
        self.tasks.schedule(1, 20_000);
    }

    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        let Some(map) = ctx.map() else { return };
        let boss = ctx
            .world
            .query::<(Entity, &InMap, &Npc)>()
            .iter(ctx.world)
            .find_map(|(actor, copy, npc)| {
                (*copy == map && npc.template_id == KARGATH).then_some(actor)
            });
        if let Some(boss) = boss {
            super::dispatch(ctx.world, boss, Hook::JustSummoned(summon));
        }
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.tasks.update(u64::from(diff_ms));
        while !ctx.is_casting() && self.tasks.execute_event().is_some() {
            // KARGATH_SUMMON_HEATHEN_30737_ABSENT_70448,
            // KARGATH_SUMMON_REAVER_30785_ABSENT_70448,
            // KARGATH_SUMMON_SHARPSHOOTER_30786_ABSENT_70448: no substitutes.
            ctx.do_cast_self(
                [SUMMON_HEATHEN, SUMMON_REAVER, SUMMON_SHARPSHOOTER][self.summon_index],
            );
            self.summon_index = (self.summon_index + 1) % 3;
            self.tasks.repeat_from_due_range(20_000, 20_000);
        }
    }
}

/// TC SpellHitTarget30738 only appends native hits; it does not launch a charge.
pub(crate) fn collect_dance_target(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    if let Some(mut targets) = world.get_mut::<BladeDanceTargets>(caster) {
        targets.0.push(target);
    }
}
