//! TC a352b1fa boss_noxxion.cpp; TaskScheduler deadlines and aura effect0 hooks.
use bevy::prelude::*;
use shared::components::{Npc, UnitFlags};

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx, ReactState};
use crate::auras::{
    AuraInstance, RemoveReason,
    scripts::{AuraScript, AuraScripts},
};

const NOXXION_SCRIPT: &str = "boss_noxxion";
const TOXIC_VOLLEY: u32 = 21687;
const UPPERCUT: u32 = 22916;
const SUMMON_DUMMY: u32 = 21708;
const SUMMON_SPAWNS: u32 = 21707;

pub(super) fn is_creature_script(name: &str) -> bool {
    name == NOXXION_SCRIPT
}

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = world.get::<crate::maps::InMap>(actor)?;
    let instance = world
        .get_resource::<crate::maps::MapStore>()?
        .instance_scripts
        .get(&map.map_id)?;
    (instance == "instance_maraudon").then(|| {
        Box::new(Noxxion {
            boss: BossAi::new(0),
        }) as Box<dyn CreatureAi>
    })
}

struct Noxxion {
    boss: BossAi,
}

impl CreatureAi for Noxxion {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule_range(TOXIC_VOLLEY, 5_000, 10_000);
        self.boss.events.schedule_range(UPPERCUT, 15_000, 20_000);
        self.boss
            .events
            .schedule_range(SUMMON_DUMMY, 30_000, 40_000);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }
    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        self.boss.just_summoned(ctx, summon);
    }
    fn summoned_creature_despawn(&mut self, _ctx: &mut Ctx, summon: Entity) {
        self.boss.summoned_creature_despawn(summon);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        // BossAI's scheduler validator defers dispatch while casting; its clock
        // still advances. Passive UpdateVictim keeps this engaged scheduler alive.
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Noxxion {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let (min, max) = match event {
            TOXIC_VOLLEY => {
                ctx.do_cast_self(TOXIC_VOLLEY);
                (15_000, 20_000)
            }
            UPPERCUT => {
                ctx.do_cast_victim(UPPERCUT);
                (20_000, 30_000)
            }
            SUMMON_DUMMY => {
                ctx.do_cast_self(SUMMON_DUMMY);
                (50_000, 60_000)
            }
            _ => return,
        };
        self.boss.events.repeat_from_due_range(min, max);
    }
}

pub(super) fn register_aura_scripts(mut scripts: ResMut<AuraScripts>) {
    scripts.0.insert(
        SUMMON_DUMMY,
        AuraScript {
            on_apply: Some(apply_spawn_phase),
            on_remove: Some(remove_spawn_phase),
            ..Default::default()
        },
    );
}

fn is_creature_dummy_zero(world: &World, target: Entity, aura: &AuraInstance) -> bool {
    world.get::<Npc>(target).is_some()
        && aura
            .effects
            .iter()
            .any(|effect| effect.index == 0 && effect.aura_type == 4)
}

fn apply_spawn_phase(world: &mut World, target: Entity, aura: &AuraInstance) {
    if !is_creature_dummy_zero(world, target, aura) {
        return;
    }
    world.entity_mut(target).insert(ReactState::Passive);
    world
        .get_mut::<UnitFlags>(target)
        .expect("creature unit flags")
        .0 |= UnitFlags::NOT_SELECTABLE;
    if let Err(error) =
        crate::spell_cast::creature_cast(world, target, SUMMON_SPAWNS, Some(target), true)
    {
        warn!("Noxxion source21707 triggered summon failed: {error:?}");
    }
}

fn remove_spawn_phase(
    world: &mut World,
    target: Entity,
    aura: &AuraInstance,
    _reason: RemoveReason,
) {
    if !is_creature_dummy_zero(world, target, aura) {
        return;
    }
    world.entity_mut(target).insert(ReactState::Aggressive);
    world
        .get_mut::<UnitFlags>(target)
        .expect("creature unit flags")
        .0 &= !UnitFlags::NOT_SELECTABLE;
}
