//! Ramparts bosses, TC a352b1fa. EventMap repeats anchor to dispatch.
mod herald;
pub(crate) mod liquid_fire;
mod omor;
mod vazruden;
use crate::maps::InMap;
use crate::zones::CreatureOrigin;
use bevy::prelude::*;
pub(crate) use herald::{HeraldUnkillable, on_unit_death};
use shared::components::ThreatTable;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::instance_script::{self, EncounterState};
use super::{CreatureAi, Ctx, DamageTaken};

/// TC557–568: effect0 SCRIPT_EFFECT sends action0 to the actual hit creature.
pub(crate) fn call_nazan(world: &mut World, _caster: Entity, target: Entity, _depth: u8) {
    if world.get::<shared::components::Npc>(target).is_some() {
        super::dispatch(world, target, super::Hook::DoAction(0));
    }
}

/// TC Herald276–280/Vazruden467–474: source despawn, then copy FAIL.
fn evade_encounter(ctx: &mut Ctx) {
    let map = ctx.map().expect("registered Ramparts actor map");
    crate::combat::despawn_creature(ctx.world, ctx.me);
    instance_script::set_boss_state(ctx.world, map, 1, EncounterState::Fail);
}

const HERALD_ENCOUNTER: u32 = 1892;
const HERALD_SPAWN_GROUP: u32 = 357;
const HERALD_RESET_MS: u32 = 30_000;

/// TC instance89–105: schedule only after SetBossState accepts the transition.
pub(super) fn on_boss_state_changed(
    world: &mut World,
    map: InMap,
    encounter: u32,
    state: EncounterState,
) {
    if encounter != HERALD_ENCOUNTER
        || state != EncounterState::Fail
        || !herald::is_ramparts(world, map)
    {
        return;
    }
    let Some(script) = instance_script::instance_script(world, map) else {
        return;
    };
    script.ramparts_reset_delays.push(HERALD_RESET_MS);
}

/// TC InstanceMap2993–3000: unit hooks run before the copy's Events.Update(diff).
/// This independent clock survives the actor's despawn and executes one due event.
pub(crate) fn update_instance_resets(world: &mut World) {
    let diff = world.resource::<Time<Fixed>>().delta().as_millis() as u32;
    let mut due = Vec::new();
    for (map, script) in &mut world.resource_mut::<instance_script::InstanceScripts>().0 {
        for delay in &mut script.ramparts_reset_delays {
            *delay = delay.saturating_sub(diff);
        }
        if script.ramparts_reset_delays.first() == Some(&0) {
            script.ramparts_reset_delays.remove(0);
            due.push(*map);
        }
    }
    for map in due {
        reset_herald_copy(world, map);
    }
}

fn reset_herald_copy(world: &mut World, map: InMap) {
    crate::zones::reset_spawn_group(world, map, HERALD_SPAWN_GROUP);
    instance_script::instance_script(world, map)
        .expect("loaded reset copy")
        .ramparts_sentry_deaths = 0;
    let remaining: Vec<_> = world
        .query::<(
            Entity,
            &InMap,
            &shared::components::Npc,
            &crate::zones::TempSummon,
        )>()
        .iter(world)
        .filter_map(|(actor, copy, npc, _)| {
            (*copy == map && npc.template_id == 17_537).then_some(actor)
        })
        .collect();
    for actor in remaining {
        crate::combat::despawn_creature(world, actor);
    }
}

const ACTION_OTHER_IS_DEAD: i32 = 1;

/// Ordinary creature roles retain their original entry across Herald's living morph.
fn find_source_actor(world: &mut World, map: InMap, entry: u32) -> Option<Entity> {
    world
        .query::<(Entity, &InMap, &CreatureOrigin)>()
        .iter(world)
        .find_map(|(actor, copy, origin)| {
            (*copy == map && origin.original_entry == entry).then_some(actor)
        })
}

/// Temporary17537 has no ordinary CreatureOrigin; its actual summoner owns the role.
fn find_summoned_vazruden(world: &mut World, map: InMap, herald: Entity) -> Option<Entity> {
    world
        .query::<(
            Entity,
            &InMap,
            &shared::components::Npc,
            &crate::zones::TempSummon,
        )>()
        .iter(world)
        .find_map(|(actor, copy, npc, summon)| {
            (*copy == map && npc.template_id == 17_537 && summon.summoner == herald)
                .then_some(actor)
        })
}

/// TC death hooks notify the same-copy partner; only its later JustDied completes.
fn notify_partner_death(ctx: &mut Ctx, entry: u32) {
    let map = ctx.map().expect("registered Ramparts actor map");
    let partner = if entry == 17_537 {
        find_summoned_vazruden(ctx.world, map, ctx.me)
    } else {
        find_source_actor(ctx.world, map, entry)
    };
    if let Some(partner) = partner {
        super::dispatch(
            ctx.world,
            partner,
            super::Hook::DoAction(ACTION_OTHER_IS_DEAD),
        );
    }
}

const SCRIPT: &str = "boss_watchkeeper_gargolmar";
const TAUNT: u32 = 1;
const MORTAL_WOUND: u32 = 30641;
const SURGE: u32 = 34645;
const REQUEST_HEAL: u32 = 30642;
const RETALIATION: u32 = 22857;

pub(super) fn is_creature_script(name: &str) -> bool {
    matches!(name, SCRIPT | "boss_omor_the_unscarred")
        || name == herald::SCRIPT
        || name == vazruden::SCRIPT
}

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = world.get::<crate::maps::InMap>(actor)?;
    let instance = world
        .get_resource::<crate::maps::MapStore>()?
        .instance_scripts
        .get(&map.map_id)?;
    if instance != "instance_ramparts" {
        return None;
    }
    let script = &world.get::<super::CreatureScriptName>(actor)?.0;
    if script == herald::SCRIPT {
        return Some(Box::new(herald::HeraldIntro::default()));
    }
    if script == vazruden::SCRIPT {
        return Some(vazruden::creature_ai());
    }
    if script == "boss_omor_the_unscarred" {
        return Some(omor::creature_ai());
    }
    Some(Box::new(Gargolmar {
        // TC ordinal 0 is installed DB2 encounter 1893, completion bit 2.
        boss: BossAi::new(2),
        requested_heal: false,
        retaliation: false,
    }))
}

struct Gargolmar {
    boss: BossAi,
    requested_heal: bool,
    retaliation: bool,
}

fn farthest_threat_target(ctx: &Ctx) -> Option<Entity> {
    let threat = ctx.world.get::<ThreatTable>(ctx.me)?;
    let origin = crate::spell_cast::targets::position(ctx.world, ctx.me)?;
    let living = threat.entries().iter().filter_map(|entry| {
        let unit = Entity::try_from_bits(entry.entity)?;
        if !crate::spell_cast::targets::is_alive(ctx.world, unit) {
            return None;
        }
        let position = crate::spell_cast::targets::position(ctx.world, unit)?;
        Some((unit, position.distance_squared(origin)))
    });
    living
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
        .map(|(unit, _)| unit)
}

impl CreatureAi for Gargolmar {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.requested_heal = false;
        self.retaliation = false;
    }

    fn just_appeared(&mut self, _ctx: &mut Ctx) {
        self.boss.events.schedule_range(TAUNT, 600_000, 900_000);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(3);
        self.boss.events.schedule_range(MORTAL_WOUND, 5_000, 10_000);
        self.boss.events.schedule_range(SURGE, 3_000, 5_000);
    }

    fn damage_taken(&mut self, _ctx: &mut Ctx, _attacker: Option<Entity>, damage: DamageTaken) {
        if !self.requested_heal && damage.health_below_pct_damaged(40.0) {
            self.requested_heal = true;
            self.boss.events.schedule(REQUEST_HEAL, 0);
        }
        if !self.retaliation && damage.health_below_pct_damaged(20.0) {
            self.retaliation = true;
            self.boss.events.schedule(RETALIATION, 0);
        }
    }

    fn on_spell_cast(&mut self, ctx: &mut Ctx, spell_id: u32) {
        match spell_id {
            REQUEST_HEAL => ctx.talk(1),
            SURGE => {
                // TC OnSpellCast rolls independently of Surge's impact result.
                let roll = rand::Rng::gen_range(&mut rand::thread_rng(), 0..100);
                if roll < 50 {
                    ctx.talk(2);
                }
            }
            _ => {}
        }
    }

    fn killed_unit(&mut self, ctx: &mut Ctx, _victim: Entity) {
        ctx.talk(4);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        // Native death sound10336 remains unimplemented.
    }

    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        self.boss.just_summoned(ctx, summon);
    }

    fn summoned_creature_despawn(&mut self, _ctx: &mut Ctx, summon: Entity) {
        self.boss.summoned_creature_despawn(summon);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if ctx.update_victim() {
            boss_update_ai(self, ctx, diff_ms);
            return;
        }
        self.boss.events.update(u64::from(diff_ms));
        while let Some(event) = self.boss.events.execute_event() {
            if event == TAUNT {
                ctx.talk(0);
            }
        }
    }
}

impl BossScript for Gargolmar {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            MORTAL_WOUND => {
                ctx.do_cast_victim(MORTAL_WOUND);
                self.boss.events.repeat_range(5_000, 15_000);
            }
            SURGE => {
                if let Some(target) = farthest_threat_target(ctx) {
                    ctx.do_cast_at(Some(target), SURGE, false);
                }
                self.boss.events.repeat_range(15_000, 40_000);
            }
            REQUEST_HEAL => ctx.do_cast_self(REQUEST_HEAL),
            RETALIATION => {
                ctx.do_cast_self(RETALIATION);
                self.boss.events.repeat(30_000);
            }
            _ => {}
        }
    }
}
