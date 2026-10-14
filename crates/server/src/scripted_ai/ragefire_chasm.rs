//! TC a352b1fa RagefireChasm/boss_dark_shaman_koranthal.cpp and instance/header.
//! Koranthal, Gordoth, Adarogg and Slagmaw.
use bevy::prelude::*;

pub(crate) mod intro_triggers;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx, DamageTaken, instance_script};

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
const GORDOTH_SCRIPT: &str = "boss_lava_guard_gordoth";
const GORDOTH_BOSS_ID: u32 = 3;
const GROUND_RUPTURE: u32 = 119999;
const SEISMIC_SLAM: u32 = 120024;
const ENRAGE: u32 = 50420;
const EVENT_GROUND_RUPTURE: u32 = 1;
const EVENT_SEISMIC_SLAM: u32 = 2;
const GORDOTH_JUMP_POSITION: [f32; 4] = [-363.5392, 203.36604, -22.005634, 0.30812347];
const GORDOTH_ACTION_JUMP: i32 = 1;
const GORDOTH_POINT_JUMP: u32 = 1;
const JAIL_BREAK: u32 = 120025;
const GROUND_SLAM: u32 = 120023;
const BLOODY_SUICIDE: u32 = 120022;
const DARK_SHAMAN_RESEARCHER: u32 = 61644;
const LAB_VIAL: u32 = 211784;
const LANDING_EFFECT_RADIUS: f32 = 25.0;
const RESEARCHER_MEAT_EXPLOSIONS: [u32; 7] =
    [111778, 111778, 111779, 111779, 111780, 111780, 111780];
const ADAROGG_SCRIPT: &str = "boss_adarogg";
const ADAROGG_BOSS_ID: u32 = 0;
const INFERNO_CHARGE_CAST: u32 = 119405;
/// Server-side: only present once `tdb_serverside_spell` is loaded.
const INFERNO_CHARGE_SUMMON: u32 = 119297;
const FIRE_BREATH: u32 = 119420;
const SAY_INFERNO_CHARGE: u8 = 0;
const EVENT_INFERNO_CHARGE: u32 = 1;
const EVENT_FIRE_BREATH: u32 = 2;
const ADAROGG_ACTION_KILL_HOUNDMASTERS: i32 = 1;
const ADJULES_CHOW_TIME: u32 = 120113;
const EAT_TROGG: u32 = 120064;
const SAY_HOUNDMASTER_INTRO_0: u8 = 0;
const SAY_HOUNDMASTER_INTRO_1: u8 = 1;
const HOUNDMASTER_1: &str = "npc_corrupted_houndmaster_1";
const HOUNDMASTER_2: &str = "npc_corrupted_houndmaster_2";
const HOUNDMASTER_RANGE: f32 = 70.0;
const PATH_INTRO: u32 = 6_140_800;
const PATH_INTRO2: u32 = 6_140_801;
const PATH_INTRO3: u32 = 6_140_802;
/// WoW-space `AdaroggIntroPoints`.
const ADAROGG_INTRO_POINTS: [[f32; 3]; 2] = [
    [-282.31488, -53.24906, -60.802902],
    [-281.1894, -54.73433, -60.34256],
];
/// `TaskScheduler` tasks of the intro, as events of a separate map.
const TASK_CHOW_1: u32 = 1;
const TASK_CHOW_2: u32 = 2;
const TASK_CHOW_3: u32 = 3;
const TASK_PATH_INTRO2: u32 = 4;
const TASK_PATH_INTRO3: u32 = 5;
const TASK_EAT_TROGG: u32 = 6;
const SLAGMAW_SCRIPT: &str = "boss_slagmaw";
const SLAGMAW_BOSS_ID: u32 = 2;
const LAVA_SPIT: u32 = 119434;
const MAGNAW_SUBMERGE: u32 = 120384;
/// Server-side North, East, South, West (`SlagmawTeleportSpells`).
const MAGNAW_TELEPORTS: [u32; 4] = [119424, 119425, 119426, 119428];
const EVENT_LAVA_SPIT: u32 = 1;
const EVENT_TELEPORT: u32 = 2;
const EVENT_EMERGE: u32 = 3;
const EVENT_BOUNDARY_CHECK: u32 = 4;
const SLAGMAW_BOUNDARY: f32 = 50.0;

pub(crate) fn is_creature_script(name: &str) -> bool {
    matches!(
        name,
        SCRIPT | GORDOTH_SCRIPT | ADAROGG_SCRIPT | SLAGMAW_SCRIPT
    )
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
    let ai: Box<dyn CreatureAi> = match name {
        SCRIPT => Box::new(Koranthal {
            boss: BossAi::new(BOSS_ID),
        }),
        GORDOTH_SCRIPT => Box::new(Gordoth {
            boss: BossAi::new(GORDOTH_BOSS_ID),
            enrage_triggered: false,
        }),
        ADAROGG_SCRIPT => Box::new(Adarogg {
            boss: BossAi::new(ADAROGG_BOSS_ID),
            scheduler: super::EventMap::default(),
            eat_counter: 0,
        }),
        SLAGMAW_SCRIPT => Box::new(Slagmaw {
            boss: BossAi::new(SLAGMAW_BOSS_ID),
            lava_spit_counter: 0,
            last_teleport: MAGNAW_TELEPORTS[3],
        }),
        _ => return None,
    };
    Some(ai)
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
        despawn_at_evade(ctx, BOSS_ID);
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

/// Both bosses use the source _DespawnAtEvade default30s. Generic evade has
/// already run _EnterEvadeMode/Reset; a temporary summon is unsummoned instead.
fn despawn_at_evade(ctx: &mut Ctx, boss_id: u32) {
    let map = ctx.map().expect("Ragefire boss instance map");
    instance_script::send_disengage_unit(ctx.world, map, ctx.me);
    if ctx.world.get::<crate::zones::TempSummon>(ctx.me).is_some() {
        crate::zones::unsummon(ctx.world, ctx.me);
        return;
    }
    super::smart_ai::lifetimes::despawn_creature(ctx.world, ctx.me, EVADE_RESPAWN_SECONDS);
    instance_script::set_boss_state(
        ctx.world,
        map,
        boss_id,
        instance_script::EncounterState::Fail,
    );
}

struct Gordoth {
    boss: BossAi,
    enrage_triggered: bool,
}

impl Gordoth {
    fn destroy_landing_vials(ctx: &mut Ctx) {
        let map = ctx.map().expect("Gordoth instance map");
        let origin = ctx.position().expect("Gordoth position");
        let origin = Vec3::new(origin.x, origin.y, origin.z);
        let vials: Vec<_> = ctx
            .world
            .query::<(
                Entity,
                &crate::maps::InMap,
                &shared::protocol::GameObjectInfo,
                &shared::components::Position,
            )>()
            .iter(ctx.world)
            .filter(|(_, copy, info, position)| {
                **copy == map
                    && info.entry == LAB_VIAL
                    && Vec3::new(position.x, position.y, position.z).distance(origin)
                        <= LANDING_EFFECT_RADIUS
            })
            .map(|(entity, _, _, _)| entity)
            .collect();
        for vial in vials {
            crate::game_objects::set_go_state(
                ctx.world,
                vial,
                crate::game_objects::GO_STATE_DESTROYED,
            );
        }
    }

    fn explode_landing_researchers(ctx: &mut Ctx) {
        let map = ctx.map().expect("Gordoth instance map");
        let origin = ctx.position().expect("Gordoth position");
        let origin = Vec3::new(origin.x, origin.y, origin.z);
        let researchers: Vec<_> = ctx
            .world
            .query::<(
                Entity,
                &crate::maps::InMap,
                &shared::components::Npc,
                &shared::components::Position,
            )>()
            .iter(ctx.world)
            .filter(|(_, copy, npc, position)| {
                **copy == map
                    && npc.template_id == DARK_SHAMAN_RESEARCHER
                    && Vec3::new(position.x, position.y, position.z).distance(origin)
                        <= LANDING_EFFECT_RADIUS
            })
            .map(|(entity, _, _, _)| entity)
            .collect();
        for researcher in researchers {
            for spell in RESEARCHER_MEAT_EXPLOSIONS {
                if let Err(error) = crate::spell_cast::creature_cast(
                    ctx.world,
                    researcher,
                    spell,
                    Some(researcher),
                    true,
                ) {
                    warn!("Gordoth researcher gore {spell} failed: {error:?}");
                }
            }
            if let Err(error) = crate::spell_cast::creature_cast(
                ctx.world,
                researcher,
                BLOODY_SUICIDE,
                Some(researcher),
                false,
            ) {
                warn!("Gordoth researcher Bloody Suicide failed: {error:?}");
            }
        }
    }
}

impl CreatureAi for Gordoth {
    fn initialize_ai(&mut self, ctx: &mut Ctx) {
        let map = ctx.map().expect("Gordoth instance map");
        // A copy without the imported 1446 binding has no boss state: nothing started.
        let state = instance_script::get_boss_state(ctx.world, map, GORDOTH_BOSS_ID);
        if state.is_some_and(|state| state != instance_script::EncounterState::NotStarted) {
            let position = GORDOTH_JUMP_POSITION;
            ctx.world.entity_mut(ctx.me).insert((
                shared::components::Position {
                    x: position[0],
                    y: position[2],
                    z: -position[1],
                },
                shared::components::Rotation {
                    x: 0.0,
                    y: position[3] + std::f32::consts::FRAC_PI_2,
                    z: 0.0,
                },
            ));
        }
    }

    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.enrage_triggered = false;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(EVENT_GROUND_RUPTURE, 7_300);
        self.boss.events.schedule(EVENT_SEISMIC_SLAM, 13_300);
    }

    fn do_action(&mut self, ctx: &mut Ctx, action: i32) {
        if action != GORDOTH_ACTION_JUMP {
            return;
        }
        // Source animkit state and the unpumped +30ms cage task remain gaps.
        ctx.do_cast_self(JAIL_BREAK);
        super::jump_movement::move_jump(
            ctx.world,
            ctx.me,
            GORDOTH_POINT_JUMP,
            shared::components::Position {
                x: GORDOTH_JUMP_POSITION[0],
                y: GORDOTH_JUMP_POSITION[2],
                z: -GORDOTH_JUMP_POSITION[1],
            },
            50.0,
            4.0,
        );
    }

    fn movement_inform(&mut self, ctx: &mut Ctx, kind: super::MotionType, point: u32) {
        if kind != super::MotionType::Effect || point != GORDOTH_POINT_JUMP {
            return;
        }
        ctx.do_cast(GROUND_SLAM);
        let facing = GORDOTH_JUMP_POSITION[3] + std::f32::consts::FRAC_PI_2;
        ctx.world.entity_mut(ctx.me).insert((
            crate::creature_ai::HomePosition {
                position: shared::components::Position {
                    x: GORDOTH_JUMP_POSITION[0],
                    y: GORDOTH_JUMP_POSITION[2],
                    z: -GORDOTH_JUMP_POSITION[1],
                },
                orientation: facing,
            },
            shared::components::Rotation {
                x: 0.0,
                y: facing,
                z: 0.0,
            },
        ));
        Self::destroy_landing_vials(ctx);
        Self::explode_landing_researchers(ctx);
    }

    fn damage_taken(&mut self, ctx: &mut Ctx, _attacker: Option<Entity>, damage: DamageTaken) {
        if !self.enrage_triggered && damage.health_below_pct_damaged(31.0) {
            self.enrage_triggered = true;
            ctx.do_cast_self(ENRAGE);
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn enter_evade_mode(&mut self, ctx: &mut Ctx) {
        despawn_at_evade(ctx, GORDOTH_BOSS_ID);
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

impl BossScript for Gordoth {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            EVENT_GROUND_RUPTURE => {
                let target = ctx.select_target_random(0.0, false);
                ctx.do_cast_at(target, GROUND_RUPTURE, false);
                self.boss.events.repeat(12_100);
            }
            EVENT_SEISMIC_SLAM => {
                if let Err(error) = crate::spell_cast::creature_cast_ignoring_cooldowns(
                    ctx.world,
                    ctx.me,
                    SEISMIC_SLAM,
                    None,
                ) {
                    warn!("Gordoth source Seismic Slam failed: {error:?}");
                }
                self.boss.events.repeat(36_300);
            }
            _ => {}
        }
    }
}

/// TC boss_adarogg. Its unvalidated `scheduler` (Reset clears the BossAI validator)
/// runs the intro before UpdateVictim; engaging cancels it.
struct Adarogg {
    boss: BossAi,
    scheduler: super::EventMap,
    eat_counter: u8,
}

impl Adarogg {
    fn kill_houndmasters(&mut self, ctx: &mut Ctx) {
        let Some(first) =
            ctx.find_nearest_creature_with_string_id(HOUNDMASTER_RANGE, HOUNDMASTER_1)
        else {
            return;
        };
        let Some(second) =
            ctx.find_nearest_creature_with_string_id(HOUNDMASTER_RANGE, HOUNDMASTER_2)
        else {
            return;
        };
        super::creature_text::talk(ctx.world, first, SAY_HOUNDMASTER_INTRO_0);
        super::creature_text::talk(ctx.world, second, SAY_HOUNDMASTER_INTRO_1);
        self.scheduler.schedule(TASK_CHOW_1, 2_000);
    }

    fn chow_toward(ctx: &mut Ctx, point: [f32; 3]) {
        ctx.do_cast(ADJULES_CHOW_TIME);
        let me = ctx.me;
        ctx.move_point(
            me,
            0,
            shared::components::Position {
                x: point[0],
                y: point[2],
                z: -point[1],
            },
        );
    }

    fn move_path(ctx: &mut Ctx, path: u32) {
        // Ground non-exact paths require the prepared map389 navigation bake.
        super::waypoint_path::move_path(ctx.world, ctx.me, path);
    }

    fn run_task(&mut self, ctx: &mut Ctx, task: u32) {
        match task {
            TASK_CHOW_1 => {
                Self::chow_toward(ctx, ADAROGG_INTRO_POINTS[0]);
                self.scheduler.schedule(TASK_CHOW_2, 1_000);
            }
            TASK_CHOW_2 => {
                Self::chow_toward(ctx, ADAROGG_INTRO_POINTS[1]);
                self.scheduler.schedule(TASK_CHOW_3, 1_200);
            }
            TASK_CHOW_3 => {
                ctx.do_cast(ADJULES_CHOW_TIME);
                Self::move_path(ctx, PATH_INTRO);
            }
            TASK_PATH_INTRO2 => Self::move_path(ctx, PATH_INTRO2),
            TASK_PATH_INTRO3 => Self::move_path(ctx, PATH_INTRO3),
            TASK_EAT_TROGG => {
                if self.eat_counter >= 2 {
                    return;
                }
                ctx.do_cast(EAT_TROGG);
                self.eat_counter += 1;
                self.scheduler.schedule_range(TASK_EAT_TROGG, 2_000, 3_000);
            }
            _ => {}
        }
    }
}

impl CreatureAi for Adarogg {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.eat_counter = 0;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.scheduler.reset();
        self.boss.events.schedule(EVENT_INFERNO_CHARGE, 10_000);
        self.boss.events.schedule(EVENT_FIRE_BREATH, 20_000);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn enter_evade_mode(&mut self, ctx: &mut Ctx) {
        despawn_at_evade(ctx, ADAROGG_BOSS_ID);
    }

    fn do_action(&mut self, ctx: &mut Ctx, action: i32) {
        if action == ADAROGG_ACTION_KILL_HOUNDMASTERS {
            self.kill_houndmasters(ctx);
        }
    }

    fn waypoint_path_ended(&mut self, _ctx: &mut Ctx, _node_id: u32, path_id: u32) {
        let (task, delay) = match path_id {
            PATH_INTRO => (TASK_PATH_INTRO2, 6_148),
            PATH_INTRO2 => (TASK_PATH_INTRO3, 6_996),
            PATH_INTRO3 => (TASK_PATH_INTRO2, 6_148),
            _ => return,
        };
        self.scheduler.schedule(task, delay);
        self.eat_counter = 0;
        self.scheduler.schedule(TASK_EAT_TROGG, 1_000);
    }

    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        self.boss.just_summoned(ctx, summon);
    }

    fn summoned_creature_despawn(&mut self, _ctx: &mut Ctx, summon: Entity) {
        self.boss.summoned_creature_despawn(summon);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.scheduler.update(u64::from(diff_ms));
        while let Some(task) = self.scheduler.execute_event() {
            self.run_task(ctx, task);
        }
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Adarogg {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            EVENT_INFERNO_CHARGE => {
                if let Some(target) = ctx.select_target_random_non_tank() {
                    // Talk's target only fills text tokens; 61408 has no creature_text rows.
                    ctx.talk(SAY_INFERNO_CHARGE);
                    ctx.do_cast_at(Some(target), INFERNO_CHARGE_SUMMON, false);
                    ctx.do_cast_at(Some(target), INFERNO_CHARGE_CAST, false);
                }
                self.boss.events.repeat_range(15_000, 20_000);
            }
            EVENT_FIRE_BREATH => {
                ctx.do_cast_victim(FIRE_BREATH);
                self.boss.events.repeat_range(15_000, 20_000);
            }
            _ => {}
        }
    }
}

/// TC boss_slagmaw. Its UpdateAI runs at most one event per update.
struct Slagmaw {
    boss: BossAi,
    lava_spit_counter: u8,
    last_teleport: u32,
}

impl Slagmaw {
    /// `GetNextTeleportSpell`: a random teleport other than the previous one.
    fn next_teleport(&mut self) -> u32 {
        let choices: Vec<u32> = MAGNAW_TELEPORTS
            .into_iter()
            .filter(|&spell| spell != self.last_teleport)
            .collect();
        let pick = rand::Rng::gen_range(&mut rand::thread_rng(), 0..choices.len());
        self.last_teleport = choices[pick];
        self.last_teleport
    }

    fn lava_spit(&mut self, ctx: &mut Ctx) {
        if self.lava_spit_counter < 5 {
            if let Some(target) = ctx.select_target_random(0.0, false) {
                ctx.do_cast_at(Some(target), LAVA_SPIT, false);
                self.lava_spit_counter += 1;
            }
            self.boss.events.repeat(1_000);
        } else if self.lava_spit_counter == 5 {
            // HandleSubmergePhase.
            ctx.do_cast_self(MAGNAW_SUBMERGE);
            self.lava_spit_counter = 0;
            self.boss.events.schedule(EVENT_TELEPORT, 3_000);
        }
    }

    /// `me->GetVictim()->GetDistance(me) > 50.0f`: beyond both combat reaches.
    fn victim_beyond_boundary(ctx: &Ctx) -> bool {
        let Some(victim) = ctx.victim() else {
            return false;
        };
        match super::model_reach::distance(ctx.world, victim, ctx.me) {
            Some(distance) => distance > SLAGMAW_BOUNDARY,
            None => {
                error!(
                    "Slagmaw {}: no source combat reach for its boundary check; run scripts/encounter_model_geometry.py",
                    ctx.me
                );
                false
            }
        }
    }
}

impl CreatureAi for Slagmaw {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.lava_spit_counter = 0;
        self.last_teleport = MAGNAW_TELEPORTS[3];
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule(EVENT_LAVA_SPIT, 1_000);
        self.boss.events.schedule(EVENT_BOUNDARY_CHECK, 2_500);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn enter_evade_mode(&mut self, ctx: &mut Ctx) {
        despawn_at_evade(ctx, SLAGMAW_BOSS_ID);
    }

    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        self.boss.just_summoned(ctx, summon);
    }

    fn summoned_creature_despawn(&mut self, _ctx: &mut Ctx, summon: Entity) {
        self.boss.summoned_creature_despawn(summon);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if !ctx.update_victim() {
            return;
        }
        self.boss.events.update(u64::from(diff_ms));
        if ctx.is_casting() {
            return;
        }
        match self.boss.events.execute_event() {
            Some(EVENT_LAVA_SPIT) => self.lava_spit(ctx),
            Some(EVENT_TELEPORT) => {
                let spell = self.next_teleport();
                ctx.do_cast_self(spell);
                self.boss.events.schedule(EVENT_EMERGE, 1_000);
            }
            Some(EVENT_EMERGE) => {
                crate::auras::remove_auras(
                    ctx.world,
                    ctx.me,
                    crate::auras::RemoveReason::Cancelled,
                    |aura| aura.spell_id == MAGNAW_SUBMERGE,
                );
                self.boss.events.schedule(EVENT_LAVA_SPIT, 1_000);
            }
            Some(EVENT_BOUNDARY_CHECK) => {
                if Self::victim_beyond_boundary(ctx) {
                    crate::creature_aggro::enter_evade_mode(ctx.world, ctx.me);
                }
                self.boss.events.schedule(EVENT_BOUNDARY_CHECK, 2_500);
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
