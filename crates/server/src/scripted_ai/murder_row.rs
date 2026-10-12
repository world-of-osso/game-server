//! Murder Row Kystia combat events from TrinityCore a352b1fa
//! src/server/scripts/QuelThalas/MurderRow/boss_kystia_manaheart.cpp.
//! This ports the authored primary-actor schedule, not the whole encounter:
//! Nibbles, infusion/channel callbacks, shields, mirror destinations and
//! door/boundary behavior remain tracked source gaps.

use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx, instance_script};
use crate::maps::{InMap, InstanceManager, MapStore};

const SCRIPT: &str = "boss_kystia_manaheart";
const INSTANCE: &str = "instance_murder_row";
const EVENT_CHAOS_BARRAGE: u32 = 1;
const EVENT_MIRROR_IMAGES: u32 = 2;
const EVENT_FEL_NOVA: u32 = 3;
const SPELL_CHAOS_BARRAGE: u32 = 1230298;
const SPELL_MIRROR_IMAGES: u32 = 1264095;
pub(crate) const SPELL_FEL_NOVA_SELECTOR: u32 = 474240;
pub(crate) const SPELL_BLINK: u32 = 474183;
const SPELL_FEL_NOVA: u32 = 1223906;

pub fn is_creature_script(script_name: &str) -> bool {
    script_name == SCRIPT
}

pub fn creature_ai(
    world: &World,
    creature: Entity,
    script_name: &str,
) -> Option<Box<dyn CreatureAi>> {
    if !is_creature_script(script_name) {
        return None;
    }
    let map = world.get::<InMap>(creature)?;
    let store = world.get_resource::<MapStore>()?;
    if store.instance_scripts.get(&map.map_id).map(String::as_str) != Some(INSTANCE) {
        return None;
    }
    // Only an authenticated primary-actor binding grants encounter completion.
    let boss = instance_script::bound_boss(world, creature)?;
    let heroic = world
        .get_resource::<InstanceManager>()
        .and_then(|manager| manager.copy_difficulty(*map))
        .is_some_and(|difficulty| matches!(difficulty.difficulty_id, 2 | 8 | 23));
    Some(Box::new(KystiaManaheart { boss, heroic }))
}

/// TC spell_kystia_manaheart_fel_nova_selector::HandleHitTarget.
pub(crate) fn cast_blink_at_selected_target(
    world: &mut World,
    caster: Entity,
    target: Entity,
    _depth: u8,
) {
    Ctx { world, me: caster }.do_cast_at(Some(target), SPELL_BLINK, false);
}

/// TC spell_kystia_manaheart_blink::HandleCast (AfterCast).
pub(crate) fn cast_fel_nova_after_blink(
    world: &mut World,
    caster: Entity,
    _spell: &crate::spell_info::SpellInfo,
) {
    Ctx { world, me: caster }.do_cast_self(SPELL_FEL_NOVA);
}

struct KystiaManaheart {
    boss: BossAi,
    heroic: bool,
}

impl BossScript for KystiaManaheart {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event_id: u32) {
        match event_id {
            EVENT_CHAOS_BARRAGE => {
                // TC ignores cast-in-progress, not cast time/costs/school locks.
                // boss_update_ai already excludes an in-progress cast here.
                ctx.do_cast_at(ctx.victim(), SPELL_CHAOS_BARRAGE, false);
                self.boss.events.repeat(3_500);
            }
            EVENT_MIRROR_IMAGES => {
                ctx.talk(1);
                ctx.do_cast_at(Some(ctx.me), SPELL_MIRROR_IMAGES, false);
                self.boss.events.repeat(30_500);
            }
            EVENT_FEL_NOVA => {
                ctx.talk(2);
                ctx.do_cast(SPELL_FEL_NOVA_SELECTOR);
                self.boss.events.repeat(15_000);
            }
            _ => {}
        }
    }
}

impl CreatureAi for KystiaManaheart {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(0);
        self.boss.events.schedule(EVENT_CHAOS_BARRAGE, 1_000);
        self.boss.events.schedule(EVENT_MIRROR_IMAGES, 14_000);
        if self.heroic {
            self.boss.events.schedule(EVENT_FEL_NOVA, 12_000);
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        ctx.talk(5);
    }

    fn enter_evade_mode(&mut self, ctx: &mut Ctx) {
        ctx.talk(6);
        self.boss.reset(ctx);
    }

    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        self.boss.just_summoned(ctx, summon);
    }

    fn summoned_creature_despawn(&mut self, _ctx: &mut Ctx, summon: Entity) {
        self.boss.summoned_creature_despawn(summon);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}
