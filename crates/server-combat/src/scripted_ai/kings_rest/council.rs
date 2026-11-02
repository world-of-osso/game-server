//! Council order and lethal transitions inferred from WCL 2L8BJvP1XcbtQ4jd/1/13.
//! TC a352b1fa names the older entries; Journal2170 ties them to the same displays.
use bevy::prelude::*;
use shared::components::{Health, Npc, UnitFlags};

use super::super::boss_ai::BossAi;
use super::super::instance_script::{self, EncounterState};
use super::super::{CreatureAi, Ctx, ReactState};
use crate::maps::InMap;

pub(super) const SCRIPT: &str = "boss_council_of_tribes";
const ORDER: [u32; 3] = [135475, 135470, 135472];
const BLOCKED: u32 = UnitFlags::NON_ATTACKABLE
    | UnitFlags::NOT_SELECTABLE
    | UnitFlags::IMMUNE_TO_PC
    | UnitFlags::IMMUNE_TO_NPC;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let entry = world.get::<Npc>(actor)?.template_id;
    let index = ORDER.iter().position(|id| *id == entry)?;
    Some(Box::new(Council {
        boss: instance_script::bound_boss(world, actor)?,
        index,
        defeated: false,
    }))
}

struct Council {
    boss: BossAi,
    index: usize,
    defeated: bool,
}

impl CreatureAi for Council {
    fn just_appeared(&mut self, ctx: &mut Ctx) {
        set_active(ctx.world, ctx.me, self.index == 0);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        if !self.defeated {
            self.boss.just_engaged_with(ctx);
        }
    }

    fn just_died(&mut self, _ctx: &mut Ctx, _killer: Option<Entity>) {}

    fn update_ai(&mut self, ctx: &mut Ctx, _diff_ms: u32) {
        if self.defeated
            || !ctx.is_engaged()
            || !ctx
                .world
                .get::<Health>(ctx.me)
                .is_some_and(|h| h.current > 0.0 && h.current <= 1.0)
        {
            return;
        }
        self.defeated = true;
        let Some(map) = ctx.map() else { return };
        let target = ctx.victim();
        super::super::lost_dwarves::leave_combat_friendly(ctx.world, ctx.me);
        set_active(ctx.world, ctx.me, false);
        if let Some(next) = ORDER.get(self.index + 1) {
            let next = ctx
                .world
                .query::<(Entity, &InMap, &Npc)>()
                .iter(ctx.world)
                .find_map(|(entity, actor_map, npc)| {
                    (*actor_map == map && npc.template_id == *next).then_some(entity)
                });
            if let Some(next) = next {
                set_active(ctx.world, next, true);
                if let Some(target) = target {
                    crate::creature_aggro::attack_start(ctx.world, next, target);
                }
            }
        } else {
            instance_script::set_boss_state(
                ctx.world,
                map,
                self.boss.boss_id,
                EncounterState::Done,
            );
        }
    }
}

fn set_active(world: &mut World, actor: Entity, active: bool) {
    if let Some(mut flags) = world.get_mut::<UnitFlags>(actor) {
        if active {
            flags.0 &= !BLOCKED;
        } else {
            flags.0 |= BLOCKED;
        }
    }
    world.entity_mut(actor).insert(if active {
        ReactState::Aggressive
    } else {
        ReactState::Passive
    });
}
