//! TC a352b1fa TheStoneVault: inherited intro deaths, arrival and Normal parent clocks.
//! Conversation25768 remains explicitly unported; its source3s delay is retained.
use bevy::prelude::*;
use shared::components::{Npc, Position, PowerType, UnitFlags};

use super::boss_ai::BossAi;
use super::{CreatureAi, Ctx, EventMap, Hook, MotionType, instance_script};
use crate::maps::{InMap, MapStore};
use crate::zones::CreatureStringIds;

pub(super) const SCRIPT: &str = "boss_edna";
const MAP: u32 = 2652;
const EDNA: u32 = 210108;
const INTRO_STATE: u32 = 6;
const REQUIRED_KILLS: u8 = 5;
const NOT_STARTED: u32 = 0;
const IN_PROGRESS: u32 = 1;
const DONE: u32 = 3;
const START_INTRO: i32 = 0;
const START_COMBAT_POINT: u32 = 1;
const SKARDYN_SPAWN: u32 = 447230;
const ANCHOR: u32 = 45313;
const VOLATILE_SPIKE: u32 = 424903;
const REFRACTING_BEAM: u32 = 452738;
const SEISMIC_SMASH: u32 = 424888;
// Local scheduler keys for the two authored TaskScheduler lambdas.
const WALK: u32 = 1;
const UNLOCK: u32 = 2;
const COMBAT_POSITION: Position = Position {
    x: 1.94097,
    y: 361.66537,
    z: -0.512153,
};

fn is_source_copy(world: &World, map: InMap) -> bool {
    map.map_id == MAP
        && matches!(crate::zones::map_difficulty_id(world, map), 1 | 205)
        && world.get_resource::<MapStore>().is_some_and(|maps| {
            maps.instance_scripts.get(&MAP).map(String::as_str) == Some("instance_the_stonevault")
        })
}

/// InstanceScript::OnUnitDeath counts HasStringId, including template inheritance.
pub(crate) fn on_unit_death(world: &mut World, unit: Entity) {
    let Some(map) = world.get::<InMap>(unit).copied() else {
        return;
    };
    let intro_trash = world.get::<Npc>(unit).is_some()
        && world
            .get::<CreatureStringIds>(unit)
            .is_some_and(|ids| ids.has("edna_intro_trash"));
    if !intro_trash || !is_source_copy(world, map) {
        return;
    }
    let Some(script) = instance_script::instance_script(world, map) else {
        return;
    };
    if script
        .data
        .get(&INTRO_STATE)
        .copied()
        .unwrap_or(NOT_STARTED)
        != NOT_STARTED
    {
        return;
    }
    script.edna_intro_kills += 1;
    if script.edna_intro_kills < REQUIRED_KILLS {
        return;
    }
    // TC sets IN_PROGRESS even if its stored EDNA actor is absent.
    script.data.insert(INTRO_STATE, IN_PROGRESS);
    if let Some(actor) = find_edna(world, map) {
        super::dispatch(world, actor, Hook::DoAction(START_INTRO));
    }
}

fn find_edna(world: &mut World, map: InMap) -> Option<Entity> {
    world
        .query::<(Entity, &InMap, &Npc)>()
        .iter(world)
        .find_map(|(actor, copy, npc)| (*copy == map && npc.template_id == EDNA).then_some(actor))
}

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<InMap>(actor)?;
    if world.get::<Npc>(actor)?.template_id != EDNA || !is_source_copy(world, map) {
        return None;
    }
    Some(Box::new(Edna {
        boss: instance_script::bound_boss(world, actor)?,
        intro: EventMap::default(),
    }))
}

struct Edna {
    boss: BossAi,
    intro: EventMap,
}

fn remove_spawn_aura(world: &mut World, actor: Entity) {
    crate::auras::remove_auras(
        world,
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == SKARDYN_SPAWN,
    );
}

fn remove_pc_immunity(world: &mut World, actor: Entity) {
    crate::creature_aggro::clear_unit_flags(world, actor, UnitFlags::IMMUNE_TO_PC);
}

fn set_appearance_energy(world: &mut World, actor: Entity) {
    let Some((current, _)) =
        crate::spell_cast::powers::power_value(world, actor, PowerType::Energy)
    else {
        warn!(
            ?actor,
            "EDNA source Energy4 requires native class4 power ingress"
        );
        return;
    };
    crate::spell_cast::powers::change_power(world, actor, PowerType::Energy, 4.0 - current);
}

impl Edna {
    fn cast_parent(&mut self, ctx: &mut Ctx, event: u32) {
        let repeat_ms = match event {
            VOLATILE_SPIKE => {
                ctx.talk(2);
                ctx.do_cast(VOLATILE_SPIKE);
                14_600
            }
            REFRACTING_BEAM => {
                ctx.talk(3);
                ctx.do_cast(REFRACTING_BEAM);
                10_900
            }
            SEISMIC_SMASH => {
                ctx.talk(4);
                ctx.talk(5);
                ctx.do_cast_victim(SEISMIC_SMASH);
                23_100
            }
            _ => unreachable!("unknown EDNA Normal parent event"),
        };
        self.boss.events.schedule(event, repeat_ms);
    }

    fn advance_intro(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.intro.update(u64::from(diff_ms));
        while let Some(event) = self.intro.execute_event() {
            match event {
                WALK => {
                    remove_spawn_aura(ctx.world, ctx.me);
                    super::movement::set_walk(ctx.world, ctx.me, true);
                    super::movement::move_point(
                        ctx.world,
                        ctx.me,
                        START_COMBAT_POINT,
                        COMBAT_POSITION,
                    );
                }
                UNLOCK => self.finish_intro(ctx),
                _ => unreachable!("unknown EDNA intro task"),
            }
        }
    }

    fn finish_intro(&mut self, ctx: &mut Ctx) {
        if let Some(map) = ctx.map() {
            instance_script::set_data(ctx.world, map, INTRO_STATE, DONE);
        }
        ctx.talk(0);
        ctx.do_cast_at(Some(ctx.me), ANCHOR, true);
        remove_pc_immunity(ctx.world, ctx.me);
    }
}

impl CreatureAi for Edna {
    fn encounter_schedule(&self) -> Option<&EventMap> {
        Some(&self.boss.events)
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        // TC BossAI::_Reset cancels TaskScheduler tasks, not instance intro data.
        self.intro.reset();
    }

    fn just_appeared(&mut self, ctx: &mut Ctx) {
        if let Some(map) = ctx.map()
            && instance_script::get_data(ctx.world, map, INTRO_STATE) == DONE
        {
            remove_spawn_aura(ctx.world, ctx.me);
            remove_pc_immunity(ctx.world, ctx.me);
        }
        set_appearance_energy(ctx.world, ctx.me);
    }

    fn do_action(&mut self, _ctx: &mut Ctx, action: i32) {
        if action != START_INTRO {
            return;
        }
        warn!("CONVERSATION25768_CONTROLLER_UNPORTED: retaining authored3s EDNA intro delay");
        self.intro.schedule(WALK, 3_000);
    }

    fn movement_inform(&mut self, _ctx: &mut Ctx, kind: MotionType, point: u32) {
        if kind == MotionType::Point && point == START_COMBAT_POINT {
            self.intro.schedule(UNLOCK, 1_000);
        }
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(1);
        self.boss.events.schedule(VOLATILE_SPIKE, 8_100);
        self.boss.events.schedule(REFRACTING_BEAM, 11_800);
        self.boss.events.schedule(SEISMIC_SMASH, 15_400);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.intro.reset();
        self.boss.just_died(ctx);
        ctx.talk(9);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        // Source scheduler advances before UpdateVictim, including idle intro.
        self.advance_intro(ctx, diff_ms);
        if !ctx.update_victim() {
            return;
        }
        self.boss.events.update(u64::from(diff_ms));
        if ctx.is_casting() {
            return;
        }
        // Source executes one due EventMap event, not every overdue parent.
        if let Some(event) = self.boss.events.execute_event() {
            self.cast_parent(ctx, event);
        }
    }
}
