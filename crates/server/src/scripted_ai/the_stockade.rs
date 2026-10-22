//! The Stockade (map 34) scripts, ported from TrinityCore
//! src/server/scripts/EasternKingdoms/TheStockade/ (a352b1fa): `the_stockade.h`,
//! `instance_the_stockade.cpp`, `boss_hogger.cpp`, `boss_lord_overheat.cpp`,
//! `boss_randolph_moloch.cpp`. Each creature script runs only in a map whose
//! instance script is `instance_the_stockade` (`RegisterStormwindStockadesAI` →
//! `GetInstanceAI`).
//!
//! One deviation: TrinityCore's `boss_hogger::Reset` and `boss_randolph_moloch::Reset`
//! override `BossAI::Reset` without calling `_Reset`, so after a wipe their encounter
//! stays `IN_PROGRESS` and their old events stay scheduled next to the ones the next
//! pull schedules. Here both also run `_Reset`, as `boss_lord_overheat` (which keeps
//! `BossAI::Reset`) does, so every Stockade boss resets on a wipe as on Retail.

use bevy::prelude::*;
use shared::components::{Npc, Position};

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx, DamageTaken, EventMap, MotionType, ReactState};
use crate::terrain_height::wow_to_bevy;

/// `StormwindStockadeScriptName`.
pub const INSTANCE_THE_STOCKADE: &str = "instance_the_stockade";

/// `SSDataTypes`: boss ids.
pub const DATA_RANDOLPH_MOLOCH: u32 = 0;
pub const DATA_LORD_OVERHEAT: u32 = 1;
pub const DATA_HOGGER: u32 = 2;

/// `SSCreatureIds` the scripts summon (the bosses' own ids are in the tests).
pub const NPC_WARDEN_THELWATER: u32 = 46409;
pub const NPC_MORTIMER_MOLOCH: u32 = 46482;

const CREATURE_SCRIPTS: [&str; 5] = [
    "boss_hogger",
    "npc_warden_thelwater",
    "boss_lord_overheat",
    "boss_randolph_moloch",
    "npc_mortimer_moloch",
];

pub fn is_creature_script(script_name: &str) -> bool {
    CREATURE_SCRIPTS.contains(&script_name)
}

/// `GetInstanceAI<AI>(creature, "instance_the_stockade")`.
pub fn creature_ai(
    world: &World,
    creature: Entity,
    script_name: &str,
) -> Option<Box<dyn CreatureAi>> {
    let map = world.get::<crate::maps::InMap>(creature)?;
    let instance = world
        .get_resource::<crate::maps::MapStore>()
        .and_then(|store| store.instance_scripts.get(&map.map_id))?;
    if instance != INSTANCE_THE_STOCKADE {
        return None;
    }
    Some(match script_name {
        "boss_hogger" => Box::new(BossHogger::new()),
        "npc_warden_thelwater" => Box::new(NpcWardenThelwater::default()),
        "boss_lord_overheat" => Box::new(BossLordOverheat::new()),
        "boss_randolph_moloch" => Box::new(BossRandolphMoloch::new()),
        "npc_mortimer_moloch" => Box::new(NpcMortimerMoloch::default()),
        _ => return None,
    })
}

/// `Points::POINT_FINISH` (both boss_hogger.cpp and boss_randolph_moloch.cpp).
const POINT_FINISH: u32 = 1;

fn bevy_position(wow: [f32; 4]) -> Position {
    let (x, y, z) = wow_to_bevy(wow[0], wow[1], wow[2]);
    Position { x, y, z }
}

fn entry(ctx: &Ctx, unit: Entity) -> Option<u32> {
    ctx.world.get::<Npc>(unit).map(|npc| npc.template_id)
}

// --- boss_hogger.cpp ---

mod hogger {
    pub const SAY_PULL: u8 = 0;
    pub const SAY_ENRAGE: u8 = 1;
    pub const SAY_DEATH: u8 = 2;
    pub const SAY_WARDEN_1: u8 = 0;
    pub const SAY_WARDEN_2: u8 = 1;
    pub const SAY_WARDEN_3: u8 = 2;

    pub const SPELL_VICIOUS_SLICE: u32 = 86604;
    pub const SPELL_MADDENING_CALL: u32 = 86620;
    pub const SPELL_ENRAGE: u32 = 86736;

    pub const EVENT_VICIOUS_SLICE: u32 = 1;
    pub const EVENT_MADDENING_CALL: u32 = 2;
    pub const EVENT_SAY_WARDEN_1: u32 = 3;
    pub const EVENT_SAY_WARDEN_2: u32 = 4;
    pub const EVENT_SAY_WARDEN_3: u32 = 5;

    pub const WARDEN_THELWATER_MOVE_POINT: [f32; 4] = [152.019, 106.198, -35.1896, 1.082104];
    pub const WARDEN_THELWATER_SPAWN_POSITION: [f32; 4] = [138.369, 78.2932, -33.85627, 1.082104];
}

/// Hogger - 46254.
pub struct BossHogger {
    boss: BossAi,
    has_enraged: bool,
}

impl BossHogger {
    fn new() -> Self {
        Self {
            boss: BossAi::new(DATA_HOGGER),
            has_enraged: false,
        }
    }
}

impl BossScript for BossHogger {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event_id: u32) {
        match event_id {
            hogger::EVENT_VICIOUS_SLICE => {
                ctx.do_cast_victim(hogger::SPELL_VICIOUS_SLICE);
                self.boss.events.repeat_range(10_000, 14_000);
            }
            hogger::EVENT_MADDENING_CALL => {
                ctx.do_cast(hogger::SPELL_MADDENING_CALL);
                self.boss.events.repeat_range(15_000, 20_000);
            }
            _ => {}
        }
    }
}

impl CreatureAi for BossHogger {
    fn scheduled_damage(&self) -> Vec<super::ScheduledDamage> {
        use super::{ScheduledDamage, ScheduledDamageTarget};
        [
            (
                hogger::EVENT_VICIOUS_SLICE,
                hogger::SPELL_VICIOUS_SLICE,
                ScheduledDamageTarget::Victim,
            ),
            (
                hogger::EVENT_MADDENING_CALL,
                hogger::SPELL_MADDENING_CALL,
                ScheduledDamageTarget::SourceArea,
            ),
        ]
        .into_iter()
        .filter_map(|(event, spell_id, target)| {
            self.boss
                .events
                .remaining_ms(event)
                .map(|request_in_ms| ScheduledDamage {
                    spell_id,
                    request_in_ms,
                    target,
                })
        })
        .collect()
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.has_enraged = false;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(hogger::SAY_PULL);
        self.boss
            .events
            .schedule_range(hogger::EVENT_VICIOUS_SLICE, 3_000, 4_000);
        self.boss
            .events
            .schedule_range(hogger::EVENT_MADDENING_CALL, 1_000, 2_000);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        ctx.talk(hogger::SAY_DEATH);
        ctx.summon_creature(
            NPC_WARDEN_THELWATER,
            hogger::WARDEN_THELWATER_SPAWN_POSITION,
        );
    }

    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        self.boss.just_summoned(ctx, summon);
        if entry(ctx, summon) == Some(NPC_WARDEN_THELWATER) {
            ctx.move_point(
                summon,
                POINT_FINISH,
                bevy_position(hogger::WARDEN_THELWATER_MOVE_POINT),
            );
        }
    }

    fn summoned_creature_despawn(&mut self, _ctx: &mut Ctx, summon: Entity) {
        self.boss.summoned_creature_despawn(summon);
    }

    fn damage_taken(&mut self, ctx: &mut Ctx, _attacker: Option<Entity>, damage: DamageTaken) {
        if damage.health_below_pct_damaged(30.0) && !self.has_enraged {
            self.has_enraged = true;
            ctx.talk(hogger::SAY_ENRAGE);
            ctx.do_cast_self(hogger::SPELL_ENRAGE);
        }
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

/// Warden Thelwater - 46409. Its `TaskScheduler` tasks are events of an `EventMap`.
#[derive(Default)]
pub struct NpcWardenThelwater {
    scheduler: EventMap,
}

impl CreatureAi for NpcWardenThelwater {
    fn movement_inform(&mut self, _ctx: &mut Ctx, kind: MotionType, id: u32) {
        if kind != MotionType::Point {
            return;
        }
        if id == POINT_FINISH {
            self.scheduler.schedule(hogger::EVENT_SAY_WARDEN_1, 1_000);
            self.scheduler.schedule(hogger::EVENT_SAY_WARDEN_2, 5_000);
            self.scheduler.schedule(hogger::EVENT_SAY_WARDEN_3, 8_000);
        }
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.scheduler.update(u64::from(diff_ms));
        while let Some(task) = self.scheduler.execute_event() {
            match task {
                hogger::EVENT_SAY_WARDEN_1 => ctx.talk(hogger::SAY_WARDEN_1),
                hogger::EVENT_SAY_WARDEN_2 => ctx.talk(hogger::SAY_WARDEN_2),
                hogger::EVENT_SAY_WARDEN_3 => ctx.talk(hogger::SAY_WARDEN_3),
                _ => {}
            }
        }
    }
}

// --- boss_lord_overheat.cpp ---

mod overheat {
    pub const SPELL_FIREBALL: u32 = 12466;
    pub const SPELL_OVERHEAT: u32 = 86633;
    pub const SPELL_RAIN_OF_FIRE: u32 = 86636;

    pub const EVENT_OVERHEAT: u32 = 1;
    pub const EVENT_RAIN_OF_FIRE: u32 = 2;
    pub const EVENT_FIREBALL: u32 = 3;

    pub const SAY_PULL: u8 = 0;
    pub const SAY_DEATH: u8 = 1;
}

/// Lord Overheat - 46264.
pub struct BossLordOverheat {
    boss: BossAi,
}

impl BossLordOverheat {
    fn new() -> Self {
        Self {
            boss: BossAi::new(DATA_LORD_OVERHEAT),
        }
    }
}

impl BossScript for BossLordOverheat {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event_id: u32) {
        match event_id {
            overheat::EVENT_FIREBALL => {
                ctx.do_cast_victim(overheat::SPELL_FIREBALL);
                self.boss.events.repeat(2_000);
            }
            overheat::EVENT_OVERHEAT => {
                if let Some(target) = ctx.select_target_random(100.0, true) {
                    ctx.do_cast_at(Some(target), overheat::SPELL_OVERHEAT, false);
                }
                self.boss.events.repeat_range(9_000, 10_000);
            }
            overheat::EVENT_RAIN_OF_FIRE => {
                ctx.do_cast_aoe(overheat::SPELL_RAIN_OF_FIRE);
                self.boss.events.repeat_range(15_000, 20_000);
            }
            _ => {}
        }
    }
}

impl CreatureAi for BossLordOverheat {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(overheat::SAY_PULL);
        self.boss.events.schedule(overheat::EVENT_FIREBALL, 2_000);
        self.boss
            .events
            .schedule_range(overheat::EVENT_OVERHEAT, 9_000, 11_000);
        self.boss
            .events
            .schedule_range(overheat::EVENT_RAIN_OF_FIRE, 10_000, 13_000);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        ctx.talk(overheat::SAY_DEATH);
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

// --- boss_randolph_moloch.cpp ---

mod moloch {
    pub const SPELL_WILDLY_STABBING: u32 = 86726;
    pub const SPELL_SWEEP: u32 = 86729;
    pub const SPELL_VANISH: u32 = 55964;
    pub const SPELL_SHADOWSTEP: u32 = 55966;

    pub const EVENT_WILDLY_STABBING: u32 = 1;
    pub const EVENT_SWEEP: u32 = 2;
    pub const EVENT_VANISH: u32 = 3;
    pub const EVENT_JUST_VANISHED: u32 = 4;
    pub const EVENT_MORTIMER_MOLOCH_EMOTE: u32 = 6;
    pub const EVENT_MORTIMER_MOLOCH_DEATH: u32 = 7;

    pub const SAY_PULL: u8 = 0;
    pub const SAY_VANISH: u8 = 1;
    pub const SAY_DEATH: u8 = 2;
    pub const MORTIMER_MOLOCH_DEATH: u8 = 0;
    pub const MORTIMER_MOLOCH_EMOTE: u8 = 1;

    pub const MORTIMER_MOLOCH_POS: [f32; 4] = [145.5811, 0.7059, -25.606, 6.2];
}

/// Randolph Moloch - 46383.
pub struct BossRandolphMoloch {
    boss: BossAi,
    first_vanish: bool,
    second_vanish: bool,
}

impl BossRandolphMoloch {
    fn new() -> Self {
        Self {
            boss: BossAi::new(DATA_RANDOLPH_MOLOCH),
            first_vanish: false,
            second_vanish: false,
        }
    }
}

impl BossScript for BossRandolphMoloch {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event_id: u32) {
        match event_id {
            moloch::EVENT_WILDLY_STABBING => {
                ctx.do_cast_victim(moloch::SPELL_WILDLY_STABBING);
                self.boss.events.repeat_range(8_000, 12_000);
            }
            moloch::EVENT_SWEEP => {
                ctx.do_cast_victim(moloch::SPELL_SWEEP);
                self.boss
                    .events
                    .schedule_range(moloch::EVENT_SWEEP, 6_000, 7_000);
            }
            moloch::EVENT_VANISH => {
                ctx.talk(moloch::SAY_VANISH);
                ctx.remove_all_auras();
                ctx.do_cast_self(moloch::SPELL_VANISH);
                ctx.set_react_state(ReactState::Passive);
                self.boss
                    .events
                    .schedule(moloch::EVENT_JUST_VANISHED, 2_000);
            }
            moloch::EVENT_JUST_VANISHED => {
                if let Some(target) = ctx.select_target_random(100.0, true) {
                    ctx.do_cast_at(Some(target), moloch::SPELL_SHADOWSTEP, true);
                }
                ctx.set_react_state(ReactState::Aggressive);
            }
            _ => {}
        }
    }
}

impl CreatureAi for BossRandolphMoloch {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.first_vanish = false;
        self.second_vanish = false;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(moloch::SAY_PULL);
        self.boss
            .events
            .schedule_range(moloch::EVENT_WILDLY_STABBING, 4_000, 5_000);
        self.boss
            .events
            .schedule_range(moloch::EVENT_SWEEP, 2_000, 3_000);
    }

    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        self.boss.just_summoned(ctx, summon);
        if entry(ctx, summon) == Some(NPC_MORTIMER_MOLOCH)
            && let Some(position) = ctx.position()
        {
            ctx.set_walk(summon, true);
            ctx.move_point(summon, POINT_FINISH, position);
        }
    }

    fn summoned_creature_despawn(&mut self, _ctx: &mut Ctx, summon: Entity) {
        self.boss.summoned_creature_despawn(summon);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        ctx.talk(moloch::SAY_DEATH);
        ctx.summon_creature(NPC_MORTIMER_MOLOCH, moloch::MORTIMER_MOLOCH_POS);
    }

    fn damage_taken(&mut self, _ctx: &mut Ctx, _attacker: Option<Entity>, damage: DamageTaken) {
        if damage.health_below_pct_damaged(71.0)
            && damage.health_above_pct(59.0)
            && !self.first_vanish
        {
            self.first_vanish = true;
            self.boss.events.schedule(moloch::EVENT_VANISH, 1_000);
        }
        if damage.health_below_pct_damaged(41.0)
            && damage.health_above_pct(29.0)
            && !self.second_vanish
        {
            self.second_vanish = true;
            self.boss.events.schedule(moloch::EVENT_VANISH, 1_000);
        }
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

/// Mortimer Moloch - 46482.
#[derive(Default)]
pub struct NpcMortimerMoloch {
    scheduler: EventMap,
}

impl CreatureAi for NpcMortimerMoloch {
    fn reset(&mut self, ctx: &mut Ctx) {
        ctx.set_react_state(ReactState::Passive);
    }

    fn movement_inform(&mut self, _ctx: &mut Ctx, kind: MotionType, id: u32) {
        if kind != MotionType::Point {
            return;
        }
        if id == POINT_FINISH {
            self.scheduler
                .schedule(moloch::EVENT_MORTIMER_MOLOCH_EMOTE, 4_000);
            self.scheduler
                .schedule(moloch::EVENT_MORTIMER_MOLOCH_DEATH, 7_000);
        }
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.scheduler.update(u64::from(diff_ms));
        while let Some(task) = self.scheduler.execute_event() {
            match task {
                moloch::EVENT_MORTIMER_MOLOCH_EMOTE => ctx.talk(moloch::MORTIMER_MOLOCH_EMOTE),
                moloch::EVENT_MORTIMER_MOLOCH_DEATH => {
                    ctx.talk(moloch::MORTIMER_MOLOCH_DEATH);
                    ctx.kill_self();
                }
                _ => {}
            }
        }
    }
}
