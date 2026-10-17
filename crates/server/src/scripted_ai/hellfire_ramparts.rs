//! Ramparts bosses, TC a352b1fa. EventMap repeats anchor to dispatch.
mod omor;
use bevy::prelude::*;
use shared::components::ThreatTable;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx, DamageTaken};

const SCRIPT: &str = "boss_watchkeeper_gargolmar";
const TAUNT: u32 = 1;
const MORTAL_WOUND: u32 = 30641;
const SURGE: u32 = 34645;
const REQUEST_HEAL: u32 = 30642;
const RETALIATION: u32 = 22857;

pub(super) fn is_creature_script(name: &str) -> bool {
    matches!(name, SCRIPT | "boss_omor_the_unscarred")
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
    if world.get::<super::CreatureScriptName>(actor)?.0 == "boss_omor_the_unscarred" {
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
