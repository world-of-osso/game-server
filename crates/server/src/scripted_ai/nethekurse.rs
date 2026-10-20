//! Grand Warlock Nethekurse, pinned TC a352b1fa boss_nethekurse.cpp.
use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::event_map::EventMap;
use super::{CreatureAi, Ctx, DamageTaken, Hook};

const SCRIPT: &str = "boss_grand_warlock_nethekurse";
/// TC DATA_NETHEKURSE ordinal0 corresponds to installed1936/map540 bit1.
const ENCOUNTER_BIT: u32 = 1;
const SHADOW_FISSURE: u32 = 30496;
const DEATH_COIL: u32 = 30500;
const SHADOW_CLEAVE: u32 = 30495;
const SHADOW_SLAM: u32 = 35953;
const DARK_SPIN: u32 = 30502;
const ACTION_PEON_ENGAGED: i32 = 1;
const ACTION_PEON_KILLED: i32 = 2;
const EVENT_INTERNAL_COOLDOWN: u32 = 2;
const EVENT_CLEAR_EMOTE: u32 = 3;
/// TC SharedDefines.h EMOTE_STATE_APPLAUD.
const EMOTE_STATE_APPLAUD: u32 = 253;
const PEON_REACTION_MS: u64 = 5_000;

pub(super) fn is_creature_script(name: &str) -> bool {
    matches!(name, SCRIPT | "npc_fel_orc_convert")
}

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = world.get::<crate::maps::InMap>(actor)?;
    let instance = world
        .get_resource::<crate::maps::MapStore>()?
        .instance_scripts
        .get(&map.map_id)?;
    if instance != "instance_shattered_halls" {
        return None;
    }
    if world.get::<super::CreatureScriptName>(actor)?.0 == "npc_fel_orc_convert" {
        return Some(Box::new(FelOrcConvert::default()));
    }
    Some(Box::new(Nethekurse {
        boss: BossAi::new(ENCOUNTER_BIT),
        dead_peons: 0,
        dark_spin_started: false,
        peon_event_on_cooldown: false,
    }))
}

struct Nethekurse {
    boss: BossAi,
    /// TC Reset deliberately preserves the count of already killed converts.
    dead_peons: u32,
    dark_spin_started: bool,
    peon_event_on_cooldown: bool,
}

impl Nethekurse {
    fn react_to_peon(&mut self, ctx: &mut Ctx, defeated: bool) {
        ctx.talk(if defeated { 7 } else { 6 });
        let orientation = if defeated { 4.5727625 } else { 4.537856 };
        if let Some(mut rotation) = ctx.world.get_mut::<shared::components::Rotation>(ctx.me) {
            rotation.y = orientation + std::f32::consts::FRAC_PI_2;
        }
        if defeated {
            ctx.world
                .get_mut::<shared::components::UnitPose>(ctx.me)
                .expect("creature pose")
                .emote_state = EMOTE_STATE_APPLAUD;
            self.boss
                .events
                .schedule(EVENT_CLEAR_EMOTE, PEON_REACTION_MS);
        }
        if let Some(queue) = ctx
            .world
            .get_resource::<lightyear::prelude::ReplicationDirtyQueue>()
        {
            queue.mark(ctx.me);
        }
        // NETHEKURSE_WAYPOINT_PAUSE_UNIMPLEMENTED: TC also pauses patrol5s.
        self.peon_event_on_cooldown = true;
        self.boss
            .events
            .schedule(EVENT_INTERNAL_COOLDOWN, PEON_REACTION_MS);
    }

    fn update_out_of_combat(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.boss.events.update(u64::from(diff_ms));
        while let Some(event) = self.boss.events.execute_event() {
            match event {
                EVENT_INTERNAL_COOLDOWN => self.peon_event_on_cooldown = false,
                EVENT_CLEAR_EMOTE => {
                    ctx.world
                        .get_mut::<shared::components::UnitPose>(ctx.me)
                        .expect("creature pose")
                        .emote_state = 0;
                    if let Some(queue) = ctx
                        .world
                        .get_resource::<lightyear::prelude::ReplicationDirtyQueue>()
                    {
                        queue.mark(ctx.me);
                    }
                }
                _ => {}
            }
        }
    }
}

impl CreatureAi for Nethekurse {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.dark_spin_started = false;
        self.peon_event_on_cooldown = false;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.reset();
        #[cfg(test)]
        if let Some(seed) = ctx
            .world
            .get::<super::tests::cast_admission::GroundCombatSeed>(ctx.me)
        {
            self.boss.events.seed_rng(seed.0);
        }
        self.boss.events.schedule(SHADOW_FISSURE, 8_000);
        self.boss.events.schedule_range(DEATH_COIL, 10_000, 15_000);
        let cleave = ctx.dungeon_mode(SHADOW_CLEAVE, SHADOW_SLAM);
        self.boss.events.schedule_range(cleave, 15_000, 20_000);
        let group = match self.dead_peons {
            0 => 3,
            1..=3 => 4,
            4 => 5,
            _ => return,
        };
        ctx.talk(group);
    }

    fn do_action(&mut self, ctx: &mut Ctx, action: i32) {
        if !ctx.is_alive() {
            return;
        }
        match action {
            ACTION_PEON_ENGAGED if !ctx.is_engaged() && !self.peon_event_on_cooldown => {
                self.react_to_peon(ctx, false);
            }
            ACTION_PEON_KILLED => {
                self.dead_peons += 1;
                if !ctx.is_engaged() && self.dead_peons == 4 {
                    ctx.do_zone_in_combat(ctx.me);
                }
                if !ctx.is_engaged() && !self.peon_event_on_cooldown && self.dead_peons != 4 {
                    self.react_to_peon(ctx, true);
                }
            }
            _ => {}
        }
    }

    fn damage_taken(&mut self, _ctx: &mut Ctx, _attacker: Option<Entity>, damage: DamageTaken) {
        if !self.dark_spin_started && damage.health_below_pct_damaged(25.0) {
            self.dark_spin_started = true;
            self.boss.events.schedule(DARK_SPIN, 0);
        }
    }

    fn killed_unit(&mut self, ctx: &mut Ctx, _victim: Entity) {
        ctx.talk(8);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        ctx.talk(9);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if ctx.update_victim() {
            boss_update_ai(self, ctx, diff_ms);
        } else {
            self.update_out_of_combat(ctx, diff_ms);
        }
    }
}

impl BossScript for Nethekurse {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            SHADOW_FISSURE => {
                if let Some(target) = ctx.select_target_random(60.0, true) {
                    ctx.do_cast_at(Some(target), SHADOW_FISSURE, false);
                }
                self.boss.events.repeat(8_000);
            }
            DEATH_COIL => {
                if let Some(target) = ctx.select_target_random(0.0, false) {
                    ctx.do_cast_at(Some(target), DEATH_COIL, false);
                }
                self.boss.events.repeat_range(10_000, 20_000);
            }
            SHADOW_CLEAVE | SHADOW_SLAM => {
                ctx.do_cast_victim(event);
                self.boss.events.repeat_range(15_000, 20_000);
            }
            DARK_SPIN => ctx.do_cast_self(DARK_SPIN),
            _ => {}
        }
    }
}

const HEMORRHAGE: u32 = 30478;
const EVENT_CONVERT: &str = "FelOrcConvertNethekurseEvent";

#[derive(Default)]
struct FelOrcConvert {
    tasks: EventMap,
}

impl FelOrcConvert {
    fn notify_nethekurse(ctx: &mut Ctx, action: i32) {
        if !ctx
            .world
            .get::<crate::zones::CreatureStringIds>(ctx.me)
            .is_some_and(|ids| ids.has(EVENT_CONVERT))
        {
            return;
        }
        let Some(map) = ctx.map() else { return };
        let boss = ctx
            .world
            .query::<(Entity, &crate::maps::InMap, &shared::components::Npc)>()
            .iter(ctx.world)
            .find_map(|(actor, copy, npc)| {
                (*copy == map && npc.template_id == 16807).then_some(actor)
            });
        if let Some(boss) = boss {
            super::dispatch(ctx.world, boss, Hook::DoAction(action));
        }
    }
}

impl CreatureAi for FelOrcConvert {
    fn reset(&mut self, _ctx: &mut Ctx) {
        self.tasks.reset();
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        if rand::Rng::gen_bool(&mut rand::thread_rng(), 0.15) {
            ctx.talk(0);
        }
        self.tasks.schedule_range(HEMORRHAGE, 5_000, 10_000);
        Self::notify_nethekurse(ctx, ACTION_PEON_ENGAGED);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        Self::notify_nethekurse(ctx, ACTION_PEON_KILLED);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if !ctx.update_victim() {
            return;
        }
        self.tasks.update(u64::from(diff_ms));
        while !ctx.is_casting()
            && let Some(event) = self.tasks.execute_event()
        {
            if event == HEMORRHAGE {
                ctx.do_cast_victim(HEMORRHAGE);
                self.tasks.repeat_from_due_range(5_000, 10_000);
            }
        }
    }
}
