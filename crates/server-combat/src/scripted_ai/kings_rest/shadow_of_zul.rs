//! TC a352b1fa kings_rest.cpp `npc_kings_rest_shadow_of_zul` (137020).
//! Pool GO state/collision follows the source; native pool animation is unproved.
use bevy::prelude::*;
use shared::components::ModelDisplay;

use crate::scripted_ai::{CreatureAi, Ctx, EventMap};

pub(crate) const SCRIPT: &str = "npc_kings_rest_shadow_of_zul";
const SERPENT_BOSS_EVENT: &str = "SerpentBossEvent";
const SAY_ZUL_PRE_BOSS_EVENT_SPAWN: u8 = 0;
const ZUL_SHADOWFORM: u32 = 269058;
const ZUL_TRASH_EVENT_STATE: u32 = 269905;
const ZUL_DESPAWN_COSMETIC_TRASH: u32 = 270692;
const ZUL_SUMMON_VISUAL_CHANNEL: u32 = 269623;
const INVISIBLE_DISPLAY: u32 = 11686;
const LIQUID_POOL: u32 = 289347;
const TASK_EVENT_START: u32 = 1;
const TASK_SPAWN_TEMPLE: u32 = 2;
const TASK_DROP_SHADOWFORM: u32 = 3;
const TASK_VANISH: u32 = 4;

#[derive(Default)]
pub(crate) struct ShadowOfZul {
    tasks: EventMap,
}

impl ShadowOfZul {
    /// `TriggerSerpentBossEvent`: delays accumulate 1s, +9s, +5s, +1s.
    fn trigger_serpent_boss_event(&mut self) {
        self.tasks.schedule(TASK_EVENT_START, 1_000);
        self.tasks.schedule(TASK_SPAWN_TEMPLE, 10_000);
        self.tasks.schedule(TASK_DROP_SHADOWFORM, 15_000);
        self.tasks.schedule(TASK_VANISH, 16_000);
    }

    /// InstanceScript's DATA_KINGS_REST_LIQUID_POOL lookup, scoped to this copy.
    fn activate_liquid_pool(ctx: &mut Ctx) {
        let Some(map) = ctx.map() else {
            return;
        };
        let pool = ctx
            .world
            .query::<(
                Entity,
                &crate::maps::InMap,
                &shared::protocol::GameObjectInfo,
            )>()
            .iter(ctx.world)
            .find(|(_, copy, info)| **copy == map && info.entry == LIQUID_POOL)
            .map(|(entity, _, _)| entity);
        if let Some(pool) = pool {
            crate::game_objects::set_go_state(
                ctx.world,
                pool,
                crate::game_objects::GO_STATE_ACTIVE,
            );
        }
    }

    fn spawn_temple(ctx: &mut Ctx) {
        let map = ctx.map().expect("Shadow of Zul instance map");
        let difficulty = crate::zones::map_difficulty_id(ctx.world, map);
        let replicate = ctx
            .world
            .get::<lightyear::prelude::Replicate>(ctx.me)
            .is_some();
        let spawned = crate::zones::spawn_group_spawn(
            ctx.world,
            map,
            difficulty,
            replicate,
            super::SPAWN_GROUP_PRE_FIRST_BOSS,
        );
        for creature in spawned {
            ctx.do_cast_at(Some(creature), ZUL_SUMMON_VISUAL_CHANNEL, false);
        }
    }
}

impl CreatureAi for ShadowOfZul {
    fn just_appeared(&mut self, ctx: &mut Ctx) {
        let event = ctx
            .world
            .get::<crate::zones::CreatureStringIds>(ctx.me)
            .is_some_and(|ids| ids.has(SERPENT_BOSS_EVENT));
        if event {
            self.trigger_serpent_boss_event();
        }
    }

    // TC UpdateAI only advances its task scheduler, never UpdateVictim.
    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.tasks.update(u64::from(diff_ms));
        while let Some(task) = self.tasks.execute_event() {
            match task {
                TASK_EVENT_START => {
                    ctx.talk(SAY_ZUL_PRE_BOSS_EVENT_SPAWN);
                    ctx.do_cast_at(None, ZUL_TRASH_EVENT_STATE, false);
                    ctx.do_cast_at(None, ZUL_DESPAWN_COSMETIC_TRASH, false);
                    Self::activate_liquid_pool(ctx);
                }
                TASK_SPAWN_TEMPLE => Self::spawn_temple(ctx),
                TASK_DROP_SHADOWFORM => {
                    crate::auras::remove_auras(
                        ctx.world,
                        ctx.me,
                        crate::auras::RemoveReason::Cancelled,
                        |aura| aura.spell_id == ZUL_SHADOWFORM,
                    );
                }
                TASK_VANISH => {
                    ctx.world.entity_mut(ctx.me).insert(ModelDisplay {
                        display_id: INVISIBLE_DISPLAY,
                    });
                    super::super::smart_ai::lifetimes::schedule_despawn(
                        ctx.world, ctx.me, 1_000, 0, false,
                    );
                }
                _ => {}
            }
        }
    }
}
