//! TC a352b1fa RagefireChasm/boss_dark_shaman_koranthal.cpp and instance/header.
//! Koranthal only; other Ragefire boss/intro scripts are not yet ported.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx, instance_script};

pub(crate) const SCRIPT: &str = "boss_dark_shaman_koranthal";
const INSTANCE: &str = "instance_ragefire_chasm";
const BOSS_ID: u32 = 1;
const SHADOW_VORTEX: u32 = 119928;
const TWISTED_ELEMENTS: u32 = 119300;
const SHADOW_STORM: u32 = 119971;
pub(crate) const SHADOW_STORM_DUMMY: u32 = 119973;
const SAY_AGGRO: u8 = 0;
const SAY_SHADOW_STORM: u8 = 1;
const SAY_DEATH: u8 = 2;
const EVENT_TWISTED_ELEMENTS: u32 = 1;
const EVENT_SHADOW_STORM: u32 = 2;
const EVADE_RESPAWN_SECONDS: u32 = 30;

pub(crate) fn is_creature_script(name: &str) -> bool {
    name == SCRIPT
}

pub(crate) fn creature_ai(world: &World, actor: Entity, name: &str) -> Option<Box<dyn CreatureAi>> {
    if !is_creature_script(name) {
        return None;
    }
    let map = world.get::<crate::maps::InMap>(actor)?;
    let maps = world.get_resource::<crate::maps::MapStore>()?;
    if maps.instance_scripts.get(&map.map_id).map(String::as_str) != Some(INSTANCE) {
        return None;
    }
    Some(Box::new(Koranthal {
        boss: BossAi::new(BOSS_ID),
    }))
}

struct Koranthal {
    boss: BossAi,
}

impl CreatureAi for Koranthal {
    fn just_appeared(&mut self, ctx: &mut Ctx) {
        ctx.do_cast_at(Some(ctx.me), SHADOW_VORTEX, false);
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(SAY_AGGRO);
        crate::auras::remove_auras(
            ctx.world,
            ctx.me,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == SHADOW_VORTEX,
        );
        self.boss.events.schedule(EVENT_TWISTED_ELEMENTS, 6_000);
        self.boss.events.schedule(EVENT_SHADOW_STORM, 20_500);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        ctx.talk(SAY_DEATH);
    }

    fn enter_evade_mode(&mut self, ctx: &mut Ctx) {
        let map = ctx.map().expect("Koranthal instance map");
        instance_script::send_disengage_unit(ctx.world, map, ctx.me);
        // Generic enter_evade_mode has already run _EnterEvadeMode/Reset.
        if ctx.world.get::<crate::zones::TempSummon>(ctx.me).is_some() {
            crate::zones::unsummon(ctx.world, ctx.me);
            return;
        }
        super::smart_ai::lifetimes::despawn_creature(ctx.world, ctx.me, EVADE_RESPAWN_SECONDS);
        instance_script::set_boss_state(
            ctx.world,
            map,
            BOSS_ID,
            instance_script::EncounterState::Fail,
        );
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

impl BossScript for Koranthal {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            EVENT_TWISTED_ELEMENTS => {
                ctx.do_cast_at(ctx.victim(), TWISTED_ELEMENTS, false);
                self.boss.events.repeat(7_300);
            }
            EVENT_SHADOW_STORM => {
                ctx.talk(SAY_SHADOW_STORM);
                ctx.do_cast(SHADOW_STORM);
                self.boss.events.reschedule(EVENT_TWISTED_ELEMENTS, 15_700);
                self.boss.events.repeat(47_200);
            }
            _ => {}
        }
    }
}

/// `spell_dark_shaman_koranthal_shadow_storm` effect0/DUMMY OnEffectHitTarget.
pub(crate) fn cast_shadow_storm_effect(
    world: &mut World,
    caster: Entity,
    target: Entity,
    _depth: u8,
) {
    let spell =
        crate::spell_cast::scripts::script_effect_value(world, caster, SHADOW_STORM_DUMMY, 0)
            as u32;
    if let Err(error) = crate::spell_cast::creature_cast(world, caster, spell, Some(target), true) {
        warn!("Koranthal Shadow Storm child {spell} failed: {error:?}");
    }
}
