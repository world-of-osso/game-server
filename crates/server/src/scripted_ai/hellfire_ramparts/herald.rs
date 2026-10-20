//! TC a352b1fa Herald startup through first West and real fall/DESCEND3.
//! Ground parents and source paired-death credit; Liquid Fire children/reset/native remain gaps.
use bevy::prelude::*;
use shared::components::{Npc, Position, Rotation};

use super::super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::super::waypoint_path::GravityDisabled;
use super::super::{
    CreatureAi, Ctx, DamageTaken, EventMap, MotionType, ReactState, instance_script,
};
use crate::maps::{InMap, MapStore};
use crate::zones::CreatureOrigin;

pub(super) const SCRIPT: &str = "boss_vazruden_the_herald";

/// TC166 _isUnkillable=true, preserved through UpdateEntry and descent ingress.
/// Unlike Creature::SetUnkillable this DamageTaken guard also includes self-loss.
/// DESCEND4 removes it two source seconds after actual fall completion.
#[derive(Component)]
pub(crate) struct HeraldUnkillable;
const MAP: u32 = 543;
const HERALD: u32 = 17_307;
const SENTRY: u32 = 17_517;
const ACTION_START_ENCOUNTER: i32 = 1_730_700;
const INTRO_1: u32 = 1;
const INTRO_2: u32 = 2;
const DESCEND_1: u32 = 3;
const DESCEND_2: u32 = 4;
const DESCEND_3: u32 = 5;
const DESCEND_4: u32 = 6;
const FLY_TO_LOC: u32 = 7;
const FIREBALL_VICTIM: u32 = 8;
const FIREBALL_RANDOM: u32 = 9;
const FIREBALL_GROUND: u32 = 10;
const CONE_OF_FIRE: u32 = 11;
const BELLOWING_ROAR: u32 = 12;
const NAZAN: u32 = 17_536;
const SUMMON_VAZRUDEN: u32 = 30_717;
const FLY_WEST: u32 = 30_698;
const LOGIC_TIMER: u32 = 30_694;
const FACE_HIGHEST_THREAT: u32 = 30_700;
const DRAGON_HOVER: u32 = 18_430;
const WEST: Position = Position {
    x: -1423.4910,
    y: 103.7056,
    z: -1710.4010,
};
const EAST: Position = Position {
    x: -1375.3560,
    y: 99.14612,
    z: -1744.6040,
};
const NORTH: Position = Position {
    x: -1444.9900,
    y: 98.20342,
    z: -1752.9600,
};
const DESCEND: Position = Position {
    x: -1429.1720,
    y: 84.34760,
    z: -1749.1290,
};
const CENTER: Position = Position {
    x: -1410.5204,
    y: 89.05998,
    z: -1740.5852,
};

pub(super) fn is_ramparts(world: &World, map: InMap) -> bool {
    map.map_id == MAP
        && world.get_resource::<MapStore>().is_some_and(|maps| {
            maps.instance_scripts.get(&MAP).map(String::as_str) == Some("instance_ramparts")
        })
}

/// InstanceScript::OnUnitDeath: count entries, not spawn-group depletion.
pub(crate) fn on_unit_death(world: &mut World, unit: Entity) {
    let Some(map) = world.get::<InMap>(unit).copied() else {
        return;
    };
    let sentry = world
        .get::<Npc>(unit)
        .is_some_and(|npc| npc.template_id == SENTRY);
    if !sentry || !is_ramparts(world, map) {
        return;
    }
    let Some(script) = instance_script::instance_script(world, map) else {
        return;
    };
    script.ramparts_sentry_deaths += 1;
    if script.ramparts_sentry_deaths != 2 {
        return;
    }
    // Source resets even when the stored Herald is absent.
    script.ramparts_sentry_deaths = 0;
    if let Some(herald) = super::find_source_actor(world, map, HERALD) {
        super::super::dispatch(
            world,
            herald,
            super::super::Hook::DoAction(ACTION_START_ENCOUNTER),
        );
    }
}

#[derive(Default, PartialEq, Eq)]
enum IntroPhase {
    #[default]
    Waiting,
    MovingToCenter,
    AtCenter,
    Finished,
}

pub(super) struct HeraldIntro {
    phase: IntroPhase,
    special_events: EventMap,
    special_tick: std::time::Duration,
    boss: BossAi,
    last_fly_spell: u32,
    started_descending: bool,
    is_other_dead: bool,
}

impl Default for HeraldIntro {
    fn default() -> Self {
        Self {
            phase: IntroPhase::Waiting,
            special_events: EventMap::default(),
            special_tick: std::time::Duration::ZERO,
            boss: BossAi::new(1),
            last_fly_spell: FLY_WEST,
            started_descending: false,
            is_other_dead: false,
        }
    }
}

impl HeraldIntro {
    fn advance_special_clock(&mut self, ctx: &Ctx) {
        let now = ctx.world.resource::<Time>().elapsed();
        self.special_events
            .update((now - self.special_tick).as_millis() as u64);
        self.special_tick = now;
    }

    fn start_intro(&mut self, ctx: &mut Ctx) {
        ctx.talk(0);
        ctx.set_react_state(ReactState::Passive);
        ctx.world
            .entity_mut(ctx.me)
            .remove::<crate::creature_ai::WaypointState>();
        super::super::movement::clear_point_movement(ctx.world, ctx.me);
        ctx.move_point(ctx.me, 0, CENTER);
    }

    fn continue_intro(&mut self, ctx: &mut Ctx) {
        if self.phase != IntroPhase::AtCenter {
            return;
        }
        // NPC_ONESHOT_EMOTE_UNIMPLEMENTED: LAND emits nothing; no pose/social substitute.
        if !morph_to_nazan(ctx) {
            return;
        }
        ctx.set_react_state(ReactState::Passive);
        ctx.do_cast_self(SUMMON_VAZRUDEN);
        ctx.world.entity_mut(ctx.me).insert(GravityDisabled);
        ctx.do_cast_self(FLY_WEST);
        ctx.do_cast_self(LOGIC_TIMER);
        ctx.do_zone_in_combat(ctx.me);
        self.phase = IntroPhase::Finished;
    }

    // TC318–327: special events do not wait for the current non-melee cast.
    fn start_descent(&mut self, ctx: &mut Ctx) {
        ctx.talk(0);
        ctx.set_react_state(ReactState::Passive);
        crate::auras::remove_auras(
            ctx.world,
            ctx.me,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == DRAGON_HOVER,
        );
        super::super::movement::clear_point_movement(ctx.world, ctx.me);
        ctx.move_point(ctx.me, 4, DESCEND);
        crate::spell_cast::cancel_cast(ctx.world, ctx.me);
        self.boss.events.reset();
        // Point4's real completion owns the next descent event.
    }

    // TC336–346: special time runs without a combat victim or cast admission.
    fn release_ground_phase(&mut self, ctx: &mut Ctx) {
        ctx.set_react_state(ReactState::Aggressive);
        ctx.do_cast_self(LOGIC_TIMER);
        ctx.world.entity_mut(ctx.me).remove::<HeraldUnkillable>();
        #[cfg(test)]
        if let Some(seed) = ctx
            .world
            .get::<super::super::tests::cast_admission::GroundCombatSeed>(ctx.me)
        {
            self.boss.events.seed_rng(seed.0);
        }
        self.boss.events.schedule_range(FIREBALL_GROUND, 0, 5_000);
        self.boss.events.schedule_range(CONE_OF_FIRE, 0, 10_000);
        let map = ctx.map().expect("registered Ramparts actor map");
        if crate::zones::map_difficulty_id(ctx.world, map) == 2 {
            self.boss.events.schedule(BELLOWING_ROAR, 2_000);
        }
    }

    fn choose_next_flight(&mut self, ctx: &mut Ctx) {
        use rand::seq::SliceRandom;
        crate::auras::remove_auras(
            ctx.world,
            ctx.me,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == DRAGON_HOVER,
        );
        ctx.set_react_state(ReactState::Passive);
        let available: Vec<_> = [30_696, FLY_WEST, 30_699]
            .into_iter()
            .filter(|spell| *spell != self.last_fly_spell)
            .collect();
        self.last_fly_spell = *available.choose(&mut rand::thread_rng()).unwrap();
        ctx.do_cast_self(self.last_fly_spell);
        self.boss.events.repeat_range(20_000, 30_000);
    }
}

impl CreatureAi for HeraldIntro {
    fn just_appeared(&mut self, ctx: &mut Ctx) {
        ctx.world
            .entity_mut(ctx.me)
            .insert((GravityDisabled, HeraldUnkillable));
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.do_action(ctx, ACTION_START_ENCOUNTER);
        // TC173–183: all source schedules remain real, including those beyond this slice.
        self.boss.events.schedule(FIREBALL_VICTIM, 0);
        self.boss
            .events
            .schedule_range(FIREBALL_RANDOM, 10_000, 15_000);
        self.boss.events.schedule_range(FLY_TO_LOC, 20_000, 30_000);
    }

    fn do_action(&mut self, _ctx: &mut Ctx, action: i32) {
        if action == super::ACTION_OTHER_IS_DEAD {
            self.is_other_dead = true;
        }
        if action == ACTION_START_ENCOUNTER && self.phase == IntroPhase::Waiting {
            self.phase = IntroPhase::MovingToCenter;
            self.special_events.schedule(INTRO_1, 0);
        }
        // TC195–201: actual30693 hit dispatches action0 once.
        if action == 0 && !self.started_descending {
            self.started_descending = true;
            self.special_events.schedule(DESCEND_1, 0);
        }
    }

    fn enter_evade_mode(&mut self, ctx: &mut Ctx) {
        super::evade_encounter(ctx);
    }

    // TC281–288: the first real death only marks the survivor; the second completes.
    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        if self.is_other_dead {
            self.boss.just_died(ctx);
        }
        super::notify_partner_death(ctx, 17_537);
    }

    // TC217–220: Finished is !_isPerformingIntro; action0 owns the once latch.
    // TC214–215's preceding min1 adjustment runs at the precommit boundary.
    fn damage_taken(&mut self, ctx: &mut Ctx, _attacker: Option<Entity>, damage: DamageTaken) {
        if self.phase == IntroPhase::Finished && damage.health_below_pct_damaged(30.0) {
            self.do_action(ctx, 0);
        }
    }

    fn movement_inform(&mut self, ctx: &mut Ctx, kind: MotionType, id: u32) {
        if !matches!(kind, MotionType::Point | MotionType::Effect) {
            return;
        }
        match id {
            0 if self.phase == IntroPhase::MovingToCenter => {
                self.phase = IntroPhase::AtCenter;
                // Point movement precedes UpdateAI in this tick. Anchor to arrival,
                // not the previous AI tick; never shorten the source1s by one frame.
                self.advance_special_clock(ctx);
                self.special_events.schedule(INTRO_2, 1_000);
            }
            4 if self.started_descending => {
                self.advance_special_clock(ctx);
                self.special_events.schedule(DESCEND_2, 0);
            }
            5 if self.started_descending => {
                self.advance_special_clock(ctx);
                self.special_events.schedule(DESCEND_3, 0);
            }
            1..=3 if self.phase == IntroPhase::Finished => {
                ctx.do_cast_self(FACE_HIGHEST_THREAT);
                ctx.do_cast_self(DRAGON_HOVER);
                ctx.set_react_state(ReactState::Aggressive);
            }
            _ => {}
        }
    }

    fn spell_hit(&mut self, ctx: &mut Ctx, _caster: Entity, spell: u32) {
        match spell {
            30_696 => ctx.move_point(ctx.me, 1, NORTH),
            FLY_WEST => ctx.move_point(ctx.me, 2, WEST),
            30_699 => ctx.move_point(ctx.me, 3, EAST),
            FACE_HIGHEST_THREAT => face_highest_threat(ctx),
            _ => {}
        }
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.advance_special_clock(ctx);
        while let Some(event) = self.special_events.execute_event() {
            match event {
                INTRO_1 => self.start_intro(ctx),
                INTRO_2 => self.continue_intro(ctx),
                DESCEND_1 => self.start_descent(ctx),
                DESCEND_2 => super::super::fall_movement::move_fall(ctx.world, ctx.me, 5),
                DESCEND_3 => {
                    ctx.world.entity_mut(ctx.me).remove::<GravityDisabled>();
                    ctx.do_cast_self(20_538);
                    self.special_events.schedule(DESCEND_4, 2_000);
                }
                DESCEND_4 => self.release_ground_phase(ctx),
                _ => unreachable!("unknown Herald intro event{event}"),
            }
        }
        boss_update_ai(self, ctx, diff_ms);
        #[cfg(test)]
        super::super::tests::cast_admission::record_ai_tick(
            ctx.world,
            ctx.me,
            self.boss.events.elapsed_ms(),
            diff_ms,
        );
    }
}

impl BossScript for HeraldIntro {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            FLY_TO_LOC => self.choose_next_flight(ctx),
            FIREBALL_VICTIM => {
                if ctx.react_state() != ReactState::Passive {
                    ctx.do_cast_victim(difficulty_spell(ctx, 30_691, 32_491));
                }
                self.boss.events.repeat(3_200);
            }
            FIREBALL_RANDOM => {
                if ctx.react_state() != ReactState::Passive {
                    if let Some(target) = ctx.select_target_random(0.0, false) {
                        ctx.do_cast_at(Some(target), difficulty_spell(ctx, 33_793, 33_794), false);
                    }
                }
                self.boss.events.repeat_range(10_000, 15_000);
            }
            FIREBALL_GROUND => {
                if let Some(target) = ctx.select_target_random(0.0, false) {
                    ctx.do_cast_at(Some(target), difficulty_spell(ctx, 34_653, 36_920), false);
                }
                self.boss.events.repeat_range(5_000, 15_000);
            }
            CONE_OF_FIRE => {
                ctx.do_cast_victim(difficulty_spell(ctx, 30_926, 36_921));
                self.boss.events.repeat_range(10_000, 15_000);
            }
            BELLOWING_ROAR => {
                ctx.do_cast_self(39_427);
                self.boss.events.repeat_range(40_000, 50_000);
            }
            // Fireball/Cone77 hit hooks and the source GO AI live in liquid_fire.
            _ => unreachable!("unknown Herald combat event{event}"),
        }
    }
}

fn morph_to_nazan(ctx: &mut Ctx) -> bool {
    let map = ctx.map().expect("registered Ramparts actor map");
    let Some(origin) = ctx.world.get::<CreatureOrigin>(ctx.me).copied() else {
        error!(actor=?ctx.me, "Herald entry change requires ordinary source origin");
        return false;
    };
    let position = ctx.position().expect("ordinary Herald position");
    let data = ctx.world.resource::<crate::world_data::WorldData>();
    let Some(source) = data.spawns.iter().find(|spawn| spawn.guid == origin.guid) else {
        error!(actor=?ctx.me, guid=origin.guid, "Herald entry change requires original source placement");
        return false;
    };
    let placement = crate::world_data::CreatureSpawn {
        template_entry: NAZAN,
        position_x: position.x,
        position_y: -position.z,
        position_z: position.y,
        ..source.clone()
    };
    let difficulty_id = crate::zones::map_difficulty_id(ctx.world, map);
    let projection = crate::zones::read_entry_projection(
        data,
        data.template_for(NAZAN, difficulty_id),
        &mut crate::zones::CreatureSpawnContext {
            map,
            difficulty_id,
            ground: None,
            temporary: false,
        },
        &placement,
        Some(source),
        &mut rand::thread_rng(),
    );
    let Some(projection) = projection else {
        return false;
    };
    // TC305–310: gravity is enabled before entry projection and the normal30717 cast.
    ctx.world.entity_mut(ctx.me).remove::<GravityDisabled>();
    projection.apply(ctx.world.entity_mut(ctx.me), false)
}

pub(super) fn difficulty_spell(ctx: &Ctx, normal: u32, heroic: u32) -> u32 {
    let map = ctx.map().expect("registered Ramparts actor map");
    if crate::zones::map_difficulty_id(ctx.world, map) == 2 {
        heroic
    } else {
        normal
    }
}

fn face_highest_threat(ctx: &mut Ctx) {
    let Some(target) = ctx.select_target_max_threat() else {
        return;
    };
    let Some(origin) = ctx.position() else {
        return;
    };
    let Some(target) = ctx.world.get::<Position>(target).copied() else {
        return;
    };
    let Some(yaw) = crate::creature_ai::facing_along(target.x - origin.x, target.z - origin.z)
    else {
        return;
    };
    ctx.world
        .get_mut::<Rotation>(ctx.me)
        .expect("ordinary Herald rotation")
        .y = yaw;
    if let Some(queue) = ctx
        .world
        .get_resource::<lightyear::prelude::ReplicationDirtyQueue>()
    {
        queue.mark(ctx.me);
    }
}
