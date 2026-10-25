//! TC a352b1fa CityOfThreads/boss_orator_krix_vizk.cpp:111–269,359–397.
//! Normal/Follower parent/energy lanes and sourced Terrorize/Indoctrination hooks.
//! Chains/area/barrier/respawn/display controllers remain named partial boundaries.
use bevy::prelude::*;
use shared::components::PowerType;

use super::boss_ai::BossAi;
use super::{CreatureAi, Ctx};
use crate::auras::AuraInstance;
use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::spell_cast::powers::{change_power, power_value};

pub(super) const SCRIPT: &str = "boss_orator_krix_vizk";
const ORATOR: u32 = 216619;
const ORATION: u32 = 454689;
const CHAINS: u32 = 434691;
const SUBJUGATE: u32 = 434722;
pub(crate) const TERRORIZE_SELECTOR: u32 = 434808;
const TERRORIZE: u32 = 434779;
const INDOCTRINATION: u32 = 434829;
const INDOCTRINATION_DAMAGE: u32 = 434832;
const LINGERING_INFLUENCE: u32 = 434923;
// Internal EventMap work, not a spell or an invented alert lane.
const ENERGIZE_EVENT: u32 = u32::MAX;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    let npc = world.get::<shared::components::Npc>(actor)?;
    if map.map_id != 2669 || npc.template_id != ORATOR {
        return None;
    }
    if !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205) {
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
    subjugate_count: u8,
    terrorize_count: u8,
    energize_count: u8,
}

fn source_energy(world: &World, actor: Entity) -> f32 {
    power_value(world, actor, PowerType::Energy)
        .expect("Orator sourceclass4 requires native NPC Energy ingress")
        .0
}

fn set_source_energy(world: &mut World, actor: Entity, amount: f32) {
    let current = source_energy(world, actor);
    change_power(world, actor, PowerType::Energy, amount - current);
}

impl Orator {
    fn reset_counts(&mut self) {
        self.subjugate_count = 1;
        self.terrorize_count = 1;
        self.energize_count = 0;
    }

    fn energize(&mut self, ctx: &mut Ctx) {
        if source_energy(ctx.world, ctx.me) == 100.0 {
            ctx.talk(3);
            ctx.do_cast_self(INDOCTRINATION);
            self.boss.events.reschedule(ENERGIZE_EVENT, 6_000);
            return;
        }
        change_power(ctx.world, ctx.me, PowerType::Energy, 4.0);
        self.energize_count = self.energize_count.wrapping_add(1);
        let delay = if self.energize_count % 2 == 1 {
            1_200
        } else {
            800
        };
        self.boss.events.schedule(ENERGIZE_EVENT, delay);
    }

    fn cast_subjugate(&mut self, ctx: &mut Ctx) {
        ctx.talk(1);
        ctx.do_cast_victim(SUBJUGATE);
        self.subjugate_count = self.subjugate_count.wrapping_add(1);
        let delay = if self.subjugate_count % 2 == 0 {
            17_100
        } else {
            12_000
        };
        self.boss.events.schedule(SUBJUGATE, delay);
    }

    fn cast_terrorize(&mut self, ctx: &mut Ctx) {
        ctx.talk(2);
        ctx.do_cast(TERRORIZE_SELECTOR);
        self.terrorize_count = self.terrorize_count.wrapping_add(1);
        let delay = if self.terrorize_count % 2 == 0 {
            8_100
        } else {
            21_100
        };
        self.boss.events.schedule(TERRORIZE_SELECTOR, delay);
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            SUBJUGATE => self.cast_subjugate(ctx),
            TERRORIZE_SELECTOR => self.cast_terrorize(ctx),
            ENERGIZE_EVENT => self.energize(ctx),
            _ => {}
        }
    }
}

impl CreatureAi for Orator {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.reset_counts();
        // TC resets energy in JustAppeared after its evade despawn, not Reset.
        // Do not forge that missing respawn while resetting the same native actor.
    }

    fn just_appeared(&mut self, ctx: &mut Ctx) {
        ctx.do_cast_self(ORATION);
        set_source_energy(ctx.world, ctx.me, 0.0);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(0);
        ctx.do_cast_self(CHAINS);
        self.boss.events.schedule(ENERGIZE_EVENT, 1_000);
        self.boss.events.schedule(SUBJUGATE, 4_500);
        self.boss.events.schedule(TERRORIZE_SELECTOR, 9_400);
    }

    fn killed_unit(&mut self, ctx: &mut Ctx, victim: Entity) {
        if ctx
            .world
            .get::<shared::components::Player>(victim)
            .is_some()
        {
            ctx.talk(4);
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        ctx.talk(5);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if !ctx.update_victim() {
            return;
        }
        self.boss.events.update(u64::from(diff_ms));
        if ctx.is_casting() {
            return;
        }
        // TC checks casting before the while loop, not between due events.
        while let Some(event) = self.boss.events.execute_event() {
            self.execute_event(ctx, event);
        }
    }
}

pub(super) fn register_aura_scripts(mut scripts: ResMut<AuraScripts>) {
    scripts.0.insert(
        INDOCTRINATION,
        AuraScript {
            on_periodic: Some(indoctrination_tick),
            on_remove: Some(indoctrination_removed),
            ..Default::default()
        },
    );
}

fn cast_source_child(world: &mut World, caster: Entity, spell: u32, target: Entity) {
    // Exact TC TRIGGERED_IGNORE_CAST_IN_PROGRESS; no FULL_MASK/cooldown bypass.
    if let Err(reason) =
        crate::spell_cast::creature_cast_ignoring_in_progress(world, caster, spell, Some(target))
    {
        warn!("Orator sourced child{spell} from{caster} to{target} refused: {reason:?}");
    }
}

pub(crate) fn terrorize_hit(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    cast_source_child(world, caster, TERRORIZE, target);
}

fn indoctrination_tick(world: &mut World, unit: Entity, aura: &AuraInstance, index: u8) {
    if index == 0 && world.get_entity(aura.caster).is_ok() {
        cast_source_child(world, aura.caster, INDOCTRINATION_DAMAGE, unit);
    }
}

fn indoctrination_removed(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    _reason: crate::auras::RemoveReason,
) {
    if world.get_entity(aura.caster).is_ok() {
        // Source target casts the area spell back at its original caster.
        cast_source_child(world, unit, LINGERING_INFLUENCE, aura.caster);
    }
    if world.get::<shared::components::Npc>(unit).is_some() {
        set_source_energy(world, unit, 0.0);
    }
}
