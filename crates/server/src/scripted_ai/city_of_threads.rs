//! TC a352b1fa CityOfThreads/boss_orator_krix_vizk.cpp.
//! Source event counters, Energy and Normal/Follower native spell hooks.
use bevy::prelude::*;
use shared::components::{Npc, Player, PowerType};

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx};
use crate::auras::{
    AuraInstance, RemoveReason,
    scripts::{AuraScript, AuraScripts},
};
use crate::spell_cast::powers::{self, PowerPools, PowerTypeTable};

pub(super) const ORATOR_SCRIPT: &str = "boss_orator_krix_vizk";
const ORATOR: u32 = 216619;
const ORATION: u32 = 454689;
const CHAINS: u32 = 434691;
const CHAINS_PERIODIC: u32 = 440310;
const CHAINS_DAMAGE: u32 = 434710;
const CHAINS_CHARGE: u32 = 434712;
const SUBJUGATE: u32 = 434722;
pub(crate) const TERRORIZE_SELECTOR: u32 = 434808;
const TERRORIZE: u32 = 434779;
const SHADOWS_SELECTOR: u32 = 448560;
const INDOCTRINATION: u32 = 434829;
const INDOCTRINATION_DAMAGE: u32 = 434832;
const LINGERING_INFLUENCE: u32 = 434923;
const EVENT_SUBJUGATE: u32 = 1;
const EVENT_TERRORIZE: u32 = 2;
const EVENT_SHADOWS: u32 = 3;
const EVENT_ENERGIZE: u32 = 4;
const FULL_ENERGY: f32 = 100.0;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = world.get::<crate::maps::InMap>(actor)?;
    let maps = world.get_resource::<crate::maps::MapStore>()?;
    if map.map_id != 2669
        || maps.instance_scripts.get(&map.map_id).map(String::as_str)
            != Some("instance_city_of_threads")
        || world.get::<Npc>(actor)?.template_id != ORATOR
    {
        return None;
    }
    Some(Box::new(Orator {
        boss: super::instance_script::bound_boss(world, actor)?,
        subjugate_count: 1,
        terrorize_count: 1,
        energize_count: 0,
    }))
}

struct Orator {
    boss: BossAi,
    subjugate_count: u32,
    terrorize_count: u32,
    energize_count: u32,
}

fn read_energy(world: &World, actor: Entity) -> f32 {
    powers::power_value(world, actor, PowerType::Energy)
        .expect("Orator source appearance initializes native Energy")
        .0
}

impl Orator {
    fn energize(&mut self, ctx: &mut Ctx) {
        if read_energy(ctx.world, ctx.me) == FULL_ENERGY {
            ctx.talk(3);
            ctx.do_cast_self(INDOCTRINATION);
            self.boss.events.reschedule(EVENT_ENERGIZE, 6_000);
            return;
        }
        powers::change_power(ctx.world, ctx.me, PowerType::Energy, 4.0);
        self.energize_count += 1;
        let delay = if self.energize_count % 2 == 1 {
            1_200
        } else {
            800
        };
        self.boss.events.schedule(EVENT_ENERGIZE, delay);
    }
}

impl BossScript for Orator {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            EVENT_SUBJUGATE => {
                ctx.talk(1);
                ctx.do_cast_victim(SUBJUGATE);
                self.subjugate_count += 1;
                let delay = if self.subjugate_count % 2 == 0 {
                    17_100
                } else {
                    12_000
                };
                self.boss.events.schedule(EVENT_SUBJUGATE, delay);
            }
            EVENT_TERRORIZE => {
                ctx.talk(2);
                ctx.do_cast(TERRORIZE_SELECTOR);
                self.terrorize_count += 1;
                let delay = if self.terrorize_count % 2 == 0 {
                    8_100
                } else {
                    21_100
                };
                self.boss.events.schedule(EVENT_TERRORIZE, delay);
            }
            EVENT_SHADOWS => {
                ctx.do_cast(SHADOWS_SELECTOR);
                self.boss.events.schedule(EVENT_SHADOWS, 30_300);
            }
            EVENT_ENERGIZE => self.energize(ctx),
            _ => {}
        }
    }
}

impl CreatureAi for Orator {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.subjugate_count = 1;
        self.terrorize_count = 1;
        self.energize_count = 0;
    }

    fn just_appeared(&mut self, ctx: &mut Ctx) {
        ctx.do_cast_self(ORATION);
        let table = ctx.world.resource::<PowerTypeTable>();
        table
            .get(PowerType::Energy)
            .expect("Orator requires native Energy metadata");
        let mut pools = PowerPools::from_table(table, vec![PowerType::Energy]);
        pools
            .get_mut(PowerType::Energy)
            .expect("created Energy pool")
            .current = 0.0;
        ctx.world.entity_mut(ctx.me).insert(pools);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(0);
        ctx.do_cast_self(CHAINS);
        self.boss.events.schedule(EVENT_ENERGIZE, 1_000);
        self.boss.events.schedule(EVENT_SUBJUGATE, 4_500);
        self.boss.events.schedule(EVENT_TERRORIZE, 9_400);
        let difficulty = ctx
            .map()
            .map(|map| crate::zones::map_difficulty_id(ctx.world, map));
        if matches!(difficulty, Some(8 | 23)) {
            self.boss.events.schedule(EVENT_SHADOWS, 15_200);
        }
    }

    fn killed_unit(&mut self, ctx: &mut Ctx, victim: Entity) {
        if ctx.world.get::<Player>(victim).is_some() {
            ctx.talk(4);
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        ctx.talk(5);
        // Source barrier35803 removal needs authored instance-area actor data.
    }

    fn enter_evade_mode(&mut self, ctx: &mut Ctx) {
        let map = ctx.map().expect("Orator instance map");
        super::instance_script::send_disengage_unit(ctx.world, map, ctx.me);
        self.boss.summons.despawn_all(ctx.world);
        if ctx.world.get::<crate::zones::TempSummon>(ctx.me).is_some() {
            crate::zones::unsummon(ctx.world, ctx.me);
            return;
        }
        // ScriptedCreature.cpp649–671: default30s and FAIL, never instant healing.
        super::smart_ai::lifetimes::despawn_creature(ctx.world, ctx.me, 30);
        super::instance_script::set_boss_state(
            ctx.world,
            map,
            self.boss.boss_id,
            super::instance_script::EncounterState::Fail,
        );
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

fn cast_source_child(world: &mut World, caster: Entity, target: Entity, spell: u32) {
    if let Err(reason) =
        crate::spell_cast::creature_cast_ignoring_in_progress(world, caster, spell, Some(target))
    {
        warn!(
            ?caster,
            ?target,
            spell,
            ?reason,
            "Orator source child rejected by native admission"
        );
    }
}

pub(crate) fn cast_terrorize(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    cast_source_child(world, caster, target, TERRORIZE);
}

pub(super) fn register_aura_scripts(mut scripts: ResMut<AuraScripts>) {
    scripts.0.insert(
        CHAINS_PERIODIC,
        AuraScript {
            on_periodic: Some(tick_chains),
            ..Default::default()
        },
    );
    scripts.0.insert(
        INDOCTRINATION,
        AuraScript {
            on_periodic: Some(tick_indoctrination),
            on_remove: Some(finish_indoctrination),
            ..Default::default()
        },
    );
}

fn tick_chains(world: &mut World, target: Entity, aura: &AuraInstance, index: u8) {
    if index != 0 {
        return;
    }
    cast_source_child(world, aura.caster, target, CHAINS_DAMAGE);
    cast_source_child(world, target, aura.caster, CHAINS_CHARGE);
}

fn tick_indoctrination(world: &mut World, target: Entity, aura: &AuraInstance, index: u8) {
    if index == 0 {
        cast_source_child(world, aura.caster, target, INDOCTRINATION_DAMAGE);
    }
}

fn finish_indoctrination(
    world: &mut World,
    target: Entity,
    aura: &AuraInstance,
    _reason: RemoveReason,
) {
    cast_source_child(world, target, aura.caster, LINGERING_INFLUENCE);
    if world.get::<Npc>(target).is_some() {
        let current = read_energy(world, target);
        powers::change_power(world, target, PowerType::Energy, -current);
    }
}
