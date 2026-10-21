//! TrinityCore `BossAI` and `SummonList` (game/AI/ScriptedAI/ScriptedCreature.cpp:545-672):
//! the event map, the summons and the instance boss state a boss script shares.
//!
//! Boss frames: `_JustEngagedWith` also sends `EncounterEngageUnit` and `_Reset` /
//! `_JustDied` `EncounterDisengageUnit` (Retail `INSTANCE_ENCOUNTER_ENGAGE_UNIT`;
//! TrinityCore leaves `SendEncounterUnit(ENCOUNTER_FRAME_ENGAGE, me)` to each
//! script, and the Stockade scripts do not call it).

use bevy::prelude::*;

use super::instance_script::{self, EncounterState};
use super::{Ctx, EventMap, Hook};

/// `SummonList`: the creatures a boss summoned.
#[derive(Debug, Default)]
pub struct SummonList(Vec<Entity>);

impl SummonList {
    /// `SummonList::Summon`.
    pub fn summon(&mut self, summon: Entity) {
        self.0.push(summon);
    }

    /// `SummonList::Despawn`.
    pub fn despawn(&mut self, summon: Entity) {
        self.0.retain(|&entity| entity != summon);
    }

    /// `SummonList::DespawnAll`: every summon still in the world goes.
    pub fn despawn_all(&mut self, world: &mut World) {
        for summon in std::mem::take(&mut self.0) {
            crate::zones::unsummon(world, summon);
        }
    }
}

/// `BossAI`'s state: `_bossId`, `events`, `summons`.
#[derive(Debug)]
pub struct BossAi {
    pub boss_id: u32,
    pub events: EventMap,
    pub summons: SummonList,
}

/// A boss script: its `BossAI` and `ExecuteEvent`.
pub trait BossScript {
    fn boss_ai(&mut self) -> &mut BossAi;
    fn execute_event(&mut self, ctx: &mut Ctx, event_id: u32);
}

impl BossAi {
    pub fn new(boss_id: u32) -> Self {
        Self {
            boss_id,
            events: EventMap::default(),
            summons: SummonList::default(),
        }
    }

    /// `BossAI::_Reset`.
    pub fn reset(&mut self, ctx: &mut Ctx) {
        if !ctx.is_alive() {
            return;
        }
        self.events.reset();
        #[cfg(test)]
        self.seed_fixture_events(ctx);
        self.summons.despawn_all(ctx.world);
        let Some(map) = ctx.map() else {
            return;
        };
        let state = instance_script::get_boss_state(ctx.world, map, self.boss_id);
        if state.is_some_and(|state| state != EncounterState::Done) {
            instance_script::set_boss_state(
                ctx.world,
                map,
                self.boss_id,
                EncounterState::NotStarted,
            );
        }
        if state == Some(EncounterState::InProgress) {
            instance_script::send_disengage_unit(ctx.world, map, ctx.me);
        }
    }

    #[cfg(test)]
    fn seed_fixture_events(&mut self, ctx: &Ctx) {
        if let Some(seed) = ctx.world.get_resource::<super::tests::EncounterSeed>() {
            self.events.seed_rng_if_unseeded(seed.0);
        }
    }

    /// `BossAI::_JustEngagedWith` (`CheckRequiredBosses` is `true`: the Stockade
    /// requires no earlier boss).
    pub fn just_engaged_with(&mut self, ctx: &mut Ctx) {
        // Some scripts have no Reset override; seed before their first schedules too.
        #[cfg(test)]
        self.seed_fixture_events(ctx);
        let Some(map) = ctx.map() else {
            return;
        };
        if instance_script::get_boss_state(ctx.world, map, self.boss_id).is_none() {
            return;
        }
        instance_script::set_boss_state(ctx.world, map, self.boss_id, EncounterState::InProgress);
        instance_script::send_engage_unit(ctx.world, map, ctx.me);
    }

    /// `BossAI::_JustDied`.
    pub fn just_died(&mut self, ctx: &mut Ctx) {
        self.events.reset();
        self.summons.despawn_all(ctx.world);
        if let Some(map) = ctx.map()
            && instance_script::get_boss_state(ctx.world, map, self.boss_id).is_some()
        {
            instance_script::set_boss_state(ctx.world, map, self.boss_id, EncounterState::Done);
            instance_script::send_disengage_unit(ctx.world, map, ctx.me);
        }
    }

    /// `BossAI::JustSummoned`.
    pub fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        self.summons.summon(summon);
        if ctx.is_engaged() {
            ctx.do_zone_in_combat(summon);
        }
    }

    /// `BossAI::SummonedCreatureDespawn`.
    pub fn summoned_creature_despawn(&mut self, summon: Entity) {
        self.summons.despawn(summon);
    }
}

/// `BossAI::UpdateAI`: events tick only while the boss has a victim, and wait
/// while it casts. Alert schedules live separately in the boss-mod catalog.
pub fn boss_update_ai<S: BossScript>(script: &mut S, ctx: &mut Ctx, diff_ms: u32) {
    if !ctx.update_victim() {
        return;
    }
    script.boss_ai().events.update(u64::from(diff_ms));
    if ctx.is_casting() {
        return;
    }
    while let Some(event_id) = script.boss_ai().events.execute_event() {
        script.execute_event(ctx, event_id);
        if ctx.is_casting() {
            return;
        }
    }
}

/// The hook a summon's despawn raises on its summoner.
pub(crate) fn notify_summoner_despawn(world: &mut World, summoner: Entity, summon: Entity) {
    super::dispatch(world, summoner, Hook::SummonedCreatureDespawn(summon));
}
