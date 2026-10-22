//! TC a352b1fa boss_leymor.cpp:201–259, npc_arcane_tender.
use bevy::prelude::*;
use shared::components::Npc;

use super::super::{CreatureAi, Ctx, EventMap};

pub(super) const SCRIPT: &str = "npc_arcane_tender";
const STASIS_RITUAL: u32 = 375732;
const ERRATIC_GROWTH: u32 = 375596;
pub(crate) const WILD_ERUPTION: u32 = 375652;
const WILD_ERUPTION_MISSILE: u32 = 375650;
const EVENT_ERRATIC_GROWTH: u32 = 1;
const EVENT_WILD_ERUPTION: u32 = 2;

/// TC boss_leymor.cpp:398–415, spell_wild_eruption OnEffectHitTarget.
pub(crate) fn hit_wild_eruption(
    world: &mut World,
    ctx: &crate::spell_cast::effects::EffectCtx,
    _effect: &crate::spell_info::SpellEffect,
    _target: Entity,
) {
    let destination = ctx
        .dest
        .expect("Wild Eruption375652 requires its native hit destination");
    crate::spell_cast::cast_triggered_at_destination(
        world,
        ctx.caster,
        WILD_ERUPTION_MISSILE,
        destination.position,
        ctx.depth + 1,
    );
}

#[derive(Default)]
pub(super) struct ArcaneTender {
    events: EventMap,
}

impl CreatureAi for ArcaneTender {
    fn reset(&mut self, _ctx: &mut Ctx) {
        self.events.reset();
    }

    fn just_appeared(&mut self, ctx: &mut Ctx) {
        let Some(map) = ctx.world.get::<crate::maps::InMap>(ctx.me).copied() else {
            return;
        };
        let leymor = ctx
            .world
            .query::<(Entity, &Npc, &crate::maps::InMap)>()
            .iter(ctx.world)
            .find_map(|(entity, npc, copy)| {
                (npc.template_id == super::LEYMOR && *copy == map).then_some(entity)
            });
        if let Some(leymor) = leymor {
            ctx.do_cast_at(Some(leymor), STASIS_RITUAL, false);
        }
    }

    fn just_reached_home(&mut self, ctx: &mut Ctx) {
        self.just_appeared(ctx);
    }

    fn just_engaged_with(&mut self, _ctx: &mut Ctx, _who: Entity) {
        self.events.schedule(EVENT_ERRATIC_GROWTH, 22_000);
        self.events.schedule(EVENT_WILD_ERUPTION, 12_000);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if !ctx.update_victim() {
            return;
        }
        self.events.update(u64::from(diff_ms));
        if ctx.is_casting() {
            return;
        }
        while let Some(event) = self.events.execute_event() {
            match event {
                EVENT_ERRATIC_GROWTH => {
                    ctx.do_cast_aoe(ERRATIC_GROWTH);
                    self.events.schedule(EVENT_ERRATIC_GROWTH, 22_000);
                }
                EVENT_WILD_ERUPTION => {
                    ctx.do_cast_aoe(WILD_ERUPTION);
                    self.events.schedule(EVENT_WILD_ERUPTION, 12_000);
                }
                _ => {}
            }
            if ctx.is_casting() {
                return;
            }
        }
    }
}
