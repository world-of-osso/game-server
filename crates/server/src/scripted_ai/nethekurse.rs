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
const ACTION_START_INTRO: i32 = 0;
const EVENT_TAUNT_PEONS: u32 = 1;
const PEON_SHADOW_SEAR: u32 = 30735;
const PEON_DEATH_COIL: u32 = 30741;
const PEON_TARGET_FISSURES: u32 = 30745;
const ACTION_PEON_ENGAGED: i32 = 1;
const ACTION_PEON_KILLED: i32 = 2;
const EVENT_INTERNAL_COOLDOWN: u32 = 2;
const EVENT_CLEAR_EMOTE: u32 = 3;
/// TC SharedDefines.h EMOTE_STATE_APPLAUD.
const EMOTE_STATE_APPLAUD: u32 = 253;
const PEON_REACTION_MS: u64 = 5_000;

pub(super) fn is_creature_script(name: &str) -> bool {
    matches!(
        name,
        SCRIPT | "npc_fel_orc_convert" | "npc_lesser_shadow_fissure" | "npc_wild_shadow_fissure"
    )
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
    match world.get::<super::CreatureScriptName>(actor)?.0.as_str() {
        "npc_fel_orc_convert" => return Some(Box::new(FelOrcConvert::default())),
        "npc_lesser_shadow_fissure" => return Some(Box::new(Fissure { wild: false })),
        "npc_wild_shadow_fissure" => return Some(Box::new(Fissure { wild: true })),
        _ => {}
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
        crate::creature_ai::pause_waypoint(ctx.world, ctx.me, PEON_REACTION_MS as u32);
        self.peon_event_on_cooldown = true;
        self.boss
            .events
            .schedule(EVENT_INTERNAL_COOLDOWN, PEON_REACTION_MS);
    }

    fn taunt_peons(&mut self, ctx: &mut Ctx) {
        let group = rand::Rng::gen_range(&mut rand::thread_rng(), 0..3);
        let spell = [PEON_SHADOW_SEAR, PEON_DEATH_COIL, PEON_TARGET_FISSURES][group];
        ctx.talk(group as u8);
        ctx.do_cast_self(spell);
        self.boss.events.cancel(EVENT_TAUNT_PEONS);
        if self.dead_peons != 4 {
            self.boss
                .events
                .schedule_range(EVENT_TAUNT_PEONS, 25_000, 35_000);
        }
    }

    fn update_out_of_combat(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.boss.events.update(u64::from(diff_ms));
        while let Some(event) = self.boss.events.execute_event() {
            match event {
                EVENT_TAUNT_PEONS => self.taunt_peons(ctx),
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
            ACTION_START_INTRO if !ctx.is_engaged() && self.dead_peons != 4 => {
                self.boss.events.schedule(EVENT_TAUNT_PEONS, 0);
            }
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

    fn spell_hit(&mut self, ctx: &mut Ctx, _caster: Entity, spell: u32) {
        let group = match spell {
            PEON_SHADOW_SEAR => 1,
            PEON_DEATH_COIL => 2,
            PEON_TARGET_FISSURES => 3,
            _ => return,
        };
        if rand::Rng::gen_bool(&mut rand::thread_rng(), 0.30) {
            ctx.talk(group);
            // NETHEKURSE_PEON_ROAR_9110_AUDIO_UNPORTED: explicit sound on Coil reaction.
        }
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

/// TC DoCastSelf30735 SPELLVALUE_MAX_TARGETS2, without modifying native SpellInfo.
pub(crate) fn filter_shadow_sear(
    world: &World,
    _spell: &crate::spell_info::SpellInfo,
    explicit: Option<Entity>,
    targets: &mut Vec<Entity>,
) {
    if !explicit
        .and_then(|actor| world.get::<shared::components::Npc>(actor))
        .is_some_and(|npc| npc.template_id == 16807)
    {
        return;
    }
    // TC caps the area-entry selector, not the separate TARGET_UNIT_CASTER slot.
    let caster = explicit.expect("qualified self cast");
    let includes_caster = targets.contains(&caster);
    targets.retain(|&target| target != caster);
    use rand::seq::SliceRandom;
    targets.shuffle(&mut rand::thread_rng());
    targets.truncate(2);
    if includes_caster {
        targets.push(caster);
    }
}

struct Fissure {
    wild: bool,
}

impl CreatureAi for Fissure {
    fn initialize_ai(&mut self, ctx: &mut Ctx) {
        ctx.set_react_state(super::ReactState::Passive);
    }

    fn just_appeared(&mut self, ctx: &mut Ctx) {
        let consumption = if self.wild {
            32250
        } else {
            ctx.dungeon_mode(30497, 35952)
        };
        // Lesser30497/35952 are named absent70448 gaps, never substituted.
        ctx.do_cast_self(consumption);
    }
}

/// TC30745 FilterTargets: uniformly retain one actual source-selected target.
pub(crate) fn filter_target_fissures(
    _world: &World,
    _spell: &crate::spell_info::SpellInfo,
    _explicit: Option<Entity>,
    targets: &mut Vec<Entity>,
) {
    use rand::seq::SliceRandom;
    targets.shuffle(&mut rand::thread_rng());
    targets.truncate(1);
}

/// TC30745 OnEffectHitTarget(SCRIPT_EFFECT): normal CastSpell30744 at that hit.
pub(crate) fn cast_target_fissure(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    let result = crate::spell_cast::creature_cast(world, caster, 30744, Some(target), false);
    #[cfg(test)]
    super::tests::cast_admission::record(world, caster, 30744, &result);
    if let Err(reason) = result {
        warn!(
            ?caster,
            ?target,
            ?reason,
            "Target Fissures30745 child30744 refused"
        );
    }
}
