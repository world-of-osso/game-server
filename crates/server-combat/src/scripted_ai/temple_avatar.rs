//! INFERRED_WCL first-wave Avatar actors: first-hit times/positions in
//! M4cdBNJkTjavYgQV/11 pull12, ax1vNFnfPdXrzRQ2/16 pull9 and
//! ZgBCXQvcnhaAYT1r/5 pull10. First hit is an upper bound on spawn time,
//! not a captured spawn; retire these proxies when authored spawns are known.
use bevy::prelude::*;
use shared::components::{Health, Npc, Player};

use super::boss_ai::{BossAi, SummonList};
use super::instance_script::{self, EncounterState};
use super::{CreatureAi, Ctx, EventMap};

const AVATAR: u32 = 133392;
const GUARDIAN: u32 = 268344;
const DEFILER: u32 = 268427;
const HEXXER: u32 = 268491;
const TAINTED_STRIKE: u32 = 1300803;
const LATENT_HEX: u32 = 1302153;
const FLAME_SHOCK: u32 = 1302158;

pub(super) fn is_encounter_add(world: &World, actor: Entity) -> bool {
    world
        .get::<crate::maps::InMap>(actor)
        .is_some_and(|map| map.map_id == 1877)
        && world
            .get::<Npc>(actor)
            .is_some_and(|npc| matches!(npc.template_id, GUARDIAN | DEFILER | HEXXER))
}

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    if map.map_id != 1877 || crate::zones::map_difficulty_id(world, map) != 8 {
        return None;
    }
    let entry = world.get::<Npc>(actor)?.template_id;
    if entry == AVATAR {
        return Some(Box::new(Avatar {
            boss: super::instance_script::bound_boss(world, actor)?,
            summons: SummonList::default(),
            started: false,
        }));
    }
    matches!(entry, GUARDIAN | DEFILER | HEXXER).then(|| {
        Box::new(Add {
            entry,
            events: EventMap::default(),
        }) as Box<dyn CreatureAi>
    })
}

fn living_party_present(world: &mut World, map: crate::maps::InMap) -> bool {
    world
        .query::<(
            &crate::maps::InMap,
            &Health,
            Option<&Player>,
            Option<&crate::followers::Follower>,
        )>()
        .iter(world)
        .any(|(copy, health, player, follower)| {
            *copy == map && health.current > 0.0 && (player.is_some() || follower.is_some())
        })
}

struct Avatar {
    boss: BossAi,
    summons: SummonList,
    started: bool,
}

impl CreatureAi for Avatar {
    fn just_summoned(&mut self, _ctx: &mut Ctx, summon: Entity) {
        self.summons.summon(summon);
    }
    fn summoned_creature_despawn(&mut self, _ctx: &mut Ctx, summon: Entity) {
        self.summons.despawn(summon);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        let Some(map) = ctx.map() else { return };
        if instance_script::get_boss_state(ctx.world, map, self.boss.boss_id)
            != Some(EncounterState::InProgress)
        {
            if self.started {
                self.summons.despawn_all(ctx.world);
                self.started = false;
                self.boss.events.reset();
            }
            return;
        }
        // INFERRED_WIPE_POLICY: reset immediately when no party member in this copy is alive.
        // The retail encounter's exact wipe delay is not available in the WCL samples.
        if !living_party_present(ctx.world, map) {
            instance_script::set_boss_state(
                ctx.world,
                map,
                self.boss.boss_id,
                EncounterState::Fail,
            );
            self.summons.despawn_all(ctx.world);
            self.boss.events.reset();
            self.started = false;
            super::heal_objective::initialize(ctx.world, ctx.me);
            instance_script::set_boss_state(
                ctx.world,
                map,
                self.boss.boss_id,
                EncounterState::NotStarted,
            );
            return;
        }
        if !self.started {
            self.started = true;
            // Median first-hit times; positions are the repeated first-hit samples.
            self.boss.events.schedule(GUARDIAN, 4_118);
            self.boss.events.schedule(DEFILER, 15_889);
            self.boss.events.schedule(HEXXER, 18_724);
        }
        self.boss.events.update(u64::from(diff_ms));
        while let Some(entry) = self.boss.events.execute_event() {
            let position = match entry {
                GUARDIAN => [4147.04, 3676.64, -43.14379, 0.0],
                DEFILER => [4156.81, 3650.39, -43.25317, 0.0],
                HEXXER => [4173.58, 3654.79, -43.26488, 0.0],
                _ => continue,
            };
            if let Some(summon) = ctx.summon_creature(entry, position) {
                ctx.do_zone_in_combat(summon);
            }
        }
    }
}

struct Add {
    entry: u32,
    events: EventMap,
}
impl CreatureAi for Add {
    fn reset(&mut self, _ctx: &mut Ctx) {
        self.events.reset();
    }
    fn just_engaged_with(&mut self, _ctx: &mut Ctx, _who: Entity) {
        match self.entry {
            GUARDIAN => self.events.schedule(TAINTED_STRIKE, 2_802),
            HEXXER => {
                self.events.schedule(LATENT_HEX, 2_487);
                self.events.schedule(FLAME_SHOCK, 8_537);
            }
            _ => {}
        }
    }
    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if !ctx.update_victim() {
            return;
        }
        self.events.update(u64::from(diff_ms));
        if ctx.is_casting() {
            return;
        }
        while let Some(spell) = self.events.execute_event() {
            match spell {
                TAINTED_STRIKE | FLAME_SHOCK => ctx.do_cast_victim(spell),
                LATENT_HEX => ctx.do_cast_self(spell),
                _ => {}
            }
            if ctx.is_casting() {
                return;
            }
        }
    }
}
