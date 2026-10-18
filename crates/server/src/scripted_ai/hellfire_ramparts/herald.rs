//! Bounded Herald INTRO1: source Sentry start, idle patrol, real center movement.
//! HERALD_INTRO2_UNIMPLEMENTED: arrival does not schedule a fake next phase.
use bevy::prelude::*;
use shared::components::{Npc, Position};

use super::super::{CreatureAi, Ctx, EventMap, ReactState, instance_script};
use crate::maps::{InMap, MapStore};
use crate::zones::CreatureOrigin;

pub(super) const SCRIPT: &str = "boss_vazruden_the_herald";
const MAP: u32 = 543;
const HERALD: u32 = 17_307;
const SENTRY: u32 = 17_517;
const ACTION_START_ENCOUNTER: i32 = 1_730_700;
const INTRO_1: u32 = 1;
const CENTER: Position = Position {
    x: -1410.5204,
    y: 89.05998,
    z: -1740.5852,
};

fn is_ramparts(world: &World, map: InMap) -> bool {
    map.map_id == MAP
        && world.get_resource::<MapStore>().is_some_and(|maps| {
            maps.instance_scripts.get(&MAP).map(String::as_str) == Some("instance_ramparts")
        })
}

/// InstanceScript::OnUnitDeath: count entries, not spawn-group depletion.
pub(crate) fn on_unit_death(world: &mut World, unit: Entity) {
    let Some(map) = world.get::<InMap>(unit).copied() else {
        return;
    };
    let sentry = world
        .get::<Npc>(unit)
        .is_some_and(|npc| npc.template_id == SENTRY);
    if !sentry || !is_ramparts(world, map) {
        return;
    }
    let Some(script) = instance_script::instance_script(world, map) else {
        return;
    };
    script.ramparts_sentry_deaths += 1;
    if script.ramparts_sentry_deaths != 2 {
        return;
    }
    // Source resets even when the stored Herald is absent.
    script.ramparts_sentry_deaths = 0;
    if let Some(herald) = find_herald(world, map) {
        super::super::dispatch(
            world,
            herald,
            super::super::Hook::DoAction(ACTION_START_ENCOUNTER),
        );
    }
}

fn find_herald(world: &mut World, map: InMap) -> Option<Entity> {
    world
        .query::<(Entity, &InMap, &CreatureOrigin)>()
        .iter(world)
        .find_map(|(actor, copy, origin)| {
            (*copy == map && origin.original_entry == HERALD).then_some(actor)
        })
}

#[derive(Default)]
pub(super) struct HeraldIntro {
    started: bool,
    special_events: EventMap,
}

impl CreatureAi for HeraldIntro {
    fn do_action(&mut self, _ctx: &mut Ctx, action: i32) {
        if action == ACTION_START_ENCOUNTER && !self.started {
            self.started = true;
            self.special_events.schedule(INTRO_1, 0);
        }
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        // Source special events run independently of UpdateVictim/casting.
        self.special_events.update(u64::from(diff_ms));
        while let Some(event) = self.special_events.execute_event() {
            if event == INTRO_1 {
                ctx.talk(0);
                ctx.set_react_state(ReactState::Passive);
                // MoveIdle replaces the default patrol. AddonPathId remains data;
                // arrival must not reactivate the old WaypointState.
                ctx.world
                    .entity_mut(ctx.me)
                    .remove::<crate::creature_ai::WaypointState>();
                super::super::movement::clear_point_movement(ctx.world, ctx.me);
                ctx.move_point(ctx.me, 0, CENTER);
            }
        }
    }
}
