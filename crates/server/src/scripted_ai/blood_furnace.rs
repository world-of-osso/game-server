//! Blood Furnace bosses, TC a352b1fa boss_the_maker.cpp and boss_kelidan_the_breaker.cpp.
use bevy::prelude::*;
use shared::components::UnitFlags;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::event_map::EventMap;
use super::{CreatureAi, Ctx, Hook, ReactState};
use crate::auras::{
    AuraInstance, RemoveReason,
    scripts::{AuraScript, AuraScripts},
};

const INSTANCE: &str = "instance_blood_furnace";
const MAKER: &str = "boss_the_maker";
const KELIDAN: &str = "boss_kelidan_the_breaker";
const CHANNELER: &str = "npc_shadowmoon_channeler";
const NPC_KELIDAN: u32 = 17377;
/// TC DATA_THE_MAKER: installed DB2 encounter1922, completion bit0.
const DATA_THE_MAKER: u32 = 0;
/// TC DATA_KELIDAN is ordinal2; installed DB2 encounter1923 completes bit1.
const DATA_KELIDAN: u32 = 1;
const EXPLODING_BEAKER: u32 = 30925;
const EXPLODING_BEAKER_H: u32 = 40059;

pub(super) const SPAWN_GROUP_CHANNELER_1: u32 = 360;
pub(super) const SPAWN_GROUP_CHANNELER_2: u32 = 361;
pub(super) const SPAWN_GROUP_CHANNELER_3: u32 = 362;
pub(super) const SPAWN_GROUP_CHANNELER_4: u32 = 363;
pub(super) const SPAWN_GROUP_CHANNELER_5: u32 = 364;
const CHANNELER_SPAWN_GROUPS: [u32; 5] = [
    SPAWN_GROUP_CHANNELER_1,
    SPAWN_GROUP_CHANNELER_2,
    SPAWN_GROUP_CHANNELER_3,
    SPAWN_GROUP_CHANNELER_4,
    SPAWN_GROUP_CHANNELER_5,
];

const SAY_WAKE: u8 = 0;
const SAY_NOVA: u8 = 1;
const SAY_SLAY: u8 = 2;
const SAY_DEATH: u8 = 3;
const SAY_CHANNELER_AGGRO_1: u8 = 0;
const SAY_CHANNELER_AGGRO_2: u8 = 1;

const EVOCATION: u32 = 30935;
const SHADOW_BOLT_VOLLEY: u32 = 17228;
const SHADOW_BOLT_VOLLEY_H: u32 = 40070;
const CORRUPTION: u32 = 30938;
const BURNING_NOVA: u32 = 30940;
const STAR_BEAM: u32 = 30888;
const SHADOW_BOLT: u32 = 12739;
const SHADOW_BOLT_H: u32 = 15472;
const MARK_OF_SHADOW: u32 = 30937;
const VORTEX: u32 = 37370;
const FIRE_NOVA: u32 = 33775;
const FIRE_NOVA_H: u32 = 37371;

const EVENT_EVOCATION: u32 = 1;
const EVENT_SHADOW_BOLT_VOLLEY: u32 = 2;
const EVENT_CORRUPTION: u32 = 3;
const EVENT_BURNING_NOVA: u32 = 4;
const TASK_STAR_BEAM: u32 = 1;
const TASK_SHADOW_BOLT: u32 = 2;
const TASK_MARK_OF_SHADOW: u32 = 3;

/// `FindNearestCreatureWithOptions(100.0f, ...)` partner search radius.
const STAR_BEAM_PARTNER_RANGE: f32 = 100.0;
const STAR_BEAM_DELAY_MS: u64 = 10_000;
const STAR_BEAM_RETRY_MS: u64 = 1_000;
/// `roll_chance(15)` for the second channeler aggro line.
const CHANNELER_AGGRO_2_CHANCE: f64 = 0.15;

const ACTION_CHANNELER_DIES: i32 = 0;
const ACTION_RESET_ENCOUNTER: i32 = 1;
const MAX_CHANNELERS: u8 = 5;

pub(super) fn is_creature_script(name: &str) -> bool {
    matches!(name, MAKER | KELIDAN | CHANNELER)
}

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = world.get::<crate::maps::InMap>(actor)?;
    let instance = world
        .get_resource::<crate::maps::MapStore>()?
        .instance_scripts
        .get(&map.map_id)?;
    if instance != INSTANCE {
        return None;
    }
    match world.get::<super::CreatureScriptName>(actor)?.0.as_str() {
        MAKER => Some(Box::new(TheMaker {
            boss: BossAi::new(DATA_THE_MAKER),
        })),
        KELIDAN => Some(Box::new(Kelidan {
            boss: BossAi::new(DATA_KELIDAN),
            dead_channelers: 0,
        })),
        CHANNELER => Some(Box::new(Channeler::default())),
        _ => None,
    }
}

/// `MAKER_DOMINATION_CHARM_UNIMPLEMENTED`: Domination30923 needs player charm
/// (aura6) and is not scheduled.
struct TheMaker {
    boss: BossAi,
}

impl CreatureAi for TheMaker {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(0);
        #[cfg(test)]
        if let Some(seed) = ctx
            .world
            .get::<super::tests::cast_admission::GroundCombatSeed>(ctx.me)
        {
            self.boss.events.seed_rng(seed.0);
        }
        self.boss
            .events
            .schedule_range(EXPLODING_BEAKER, 5_000, 15_000);
    }

    fn killed_unit(&mut self, ctx: &mut Ctx, victim: Entity) {
        if ctx
            .world
            .get::<shared::components::Player>(victim)
            .is_some()
        {
            ctx.talk(1);
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        ctx.talk(2);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for TheMaker {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        if event == EXPLODING_BEAKER {
            // MAKER_NORMAL_BEAKER_30925_ABSENT_70448: Normal resolves no SpellInfo.
            ctx.do_cast_victim(ctx.dungeon_mode(EXPLODING_BEAKER, EXPLODING_BEAKER_H));
            self.boss.events.repeat_range(10_000, 20_000);
        }
    }
}

/// 17377 - Keli'dan the Breaker: dormant until his five channelers die.
struct Kelidan {
    boss: BossAi,
    dead_channelers: u8,
}

impl Kelidan {
    /// `SetImmuneToNPC`, `SetUninteractible` and the matching react state.
    fn set_dormant(ctx: &mut Ctx, dormant: bool) {
        let mut flags = *ctx
            .world
            .get::<UnitFlags>(ctx.me)
            .expect("creature unit flags");
        let bits = UnitFlags::IMMUNE_TO_NPC | UnitFlags::NOT_SELECTABLE;
        if dormant {
            flags.0 |= bits;
        } else {
            flags.0 &= !bits;
        }
        ctx.world.entity_mut(ctx.me).insert(flags);
        ctx.set_react_state(if dormant {
            ReactState::Passive
        } else {
            ReactState::Aggressive
        });
    }

    /// `SpawnGroupSpawn(group, true)` for every channeler group.
    fn spawn_channelers(ctx: &mut Ctx) {
        let Some(map) = ctx.map() else {
            return;
        };
        let difficulty = crate::zones::map_difficulty_id(ctx.world, map);
        let replicate = ctx
            .world
            .get::<lightyear::prelude::Replicate>(ctx.me)
            .is_some();
        for group in CHANNELER_SPAWN_GROUPS {
            crate::zones::spawn_group_spawn(ctx.world, map, difficulty, replicate, group);
        }
    }
}

impl CreatureAi for Kelidan {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_appeared(&mut self, ctx: &mut Ctx) {
        Self::set_dormant(ctx, true);
        ctx.do_cast_self(EVOCATION);
        Self::spawn_channelers(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(SAY_WAKE);
        crate::spell_cast::cancel_cast(ctx.world, ctx.me);
        let events = &mut self.boss.events;
        events.schedule_range(EVENT_SHADOW_BOLT_VOLLEY, 0, 2_000);
        events.schedule_range(EVENT_CORRUPTION, 5_000, 10_000);
        events.schedule_range(EVENT_BURNING_NOVA, 10_000, 20_000);
    }

    fn do_action(&mut self, ctx: &mut Ctx, action: i32) {
        match action {
            ACTION_CHANNELER_DIES => {
                self.dead_channelers += 1;
                if self.dead_channelers == MAX_CHANNELERS {
                    Self::set_dormant(ctx, false);
                    ctx.do_zone_in_combat(ctx.me);
                    self.dead_channelers = 0;
                }
            }
            ACTION_RESET_ENCOUNTER => {
                self.dead_channelers = 0;
                Self::spawn_channelers(ctx);
            }
            _ => {}
        }
    }

    fn spell_hit(&mut self, ctx: &mut Ctx, _caster: Entity, spell_id: u32) {
        if spell_id == BURNING_NOVA {
            ctx.talk(SAY_NOVA);
        }
    }

    fn just_reached_home(&mut self, ctx: &mut Ctx) {
        Self::set_dormant(ctx, true);
        self.boss.events.schedule(EVENT_EVOCATION, 5_000);
        self.do_action(ctx, ACTION_RESET_ENCOUNTER);
    }

    fn killed_unit(&mut self, ctx: &mut Ctx, _victim: Entity) {
        ctx.talk(SAY_SLAY);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        ctx.talk(SAY_DEATH);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if !ctx.update_victim() {
            self.run_idle_evocation(ctx, diff_ms);
            return;
        }
        self.boss.events.update(u64::from(diff_ms));
        if ctx.is_casting() {
            return;
        }
        while let Some(event) = self.boss.events.execute_event() {
            self.execute_event(ctx, event);
            if ctx.is_casting() {
                return;
            }
        }
    }
}

impl Kelidan {
    /// Out of combat only the reset's EVENT_EVOCATION runs; other due events drop.
    fn run_idle_evocation(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.boss.events.update(u64::from(diff_ms));
        while let Some(event) = self.boss.events.execute_event() {
            if event == EVENT_EVOCATION {
                ctx.do_cast_self(EVOCATION);
            }
        }
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let events = &mut self.boss.events;
        match event {
            EVENT_SHADOW_BOLT_VOLLEY => {
                // KELIDAN_HEROIC_VOLLEY_40070_ABSENT_70448: Heroic resolves no SpellInfo.
                ctx.do_cast_self(ctx.dungeon_mode(SHADOW_BOLT_VOLLEY, SHADOW_BOLT_VOLLEY_H));
                events.repeat_range(15_000, 25_000);
            }
            EVENT_CORRUPTION => {
                ctx.do_cast_self(CORRUPTION);
                events.repeat_range(10_000, 15_000);
            }
            EVENT_BURNING_NOVA => {
                // The source banish-immunity toggle brackets this instant self cast;
                // the stun aura already applies here, so it has no observable effect.
                ctx.do_cast_self(BURNING_NOVA);
                events.repeat_range(25_000, 35_000);
            }
            _ => {}
        }
    }
}

/// 17653 - Shadowmoon Channeler: beams its partner, notifies Keli'dan.
#[derive(Default)]
struct Channeler {
    tasks: EventMap,
}

impl Channeler {
    /// `DoCastChanneledSpell`: Star Beam at the partner after10s, retried each second.
    fn schedule_star_beam(&mut self) {
        self.tasks.schedule(TASK_STAR_BEAM, STAR_BEAM_DELAY_MS);
    }

    /// TC `GetStringIdOfChanneler`.
    fn partner_string_id(ctx: &Ctx) -> Option<&'static str> {
        let ids = ctx.world.get::<crate::zones::CreatureStringIds>(ctx.me)?;
        [
            (
                "KelidansShadowmoonChanneler1",
                "KelidansShadowmoonChanneler4",
            ),
            (
                "KelidansShadowmoonChanneler2",
                "KelidansShadowmoonChanneler3",
            ),
            (
                "KelidansShadowmoonChanneler3",
                "KelidansShadowmoonChanneler5",
            ),
            (
                "KelidansShadowmoonChanneler4",
                "KelidansShadowmoonChanneler2",
            ),
            (
                "KelidansShadowmoonChanneler5",
                "KelidansShadowmoonChanneler1",
            ),
        ]
        .into_iter()
        .find(|(own, _)| ids.has(own))
        .map(|(_, partner)| partner)
    }

    fn cast_star_beam(&mut self, ctx: &mut Ctx) {
        let partner = Self::partner_string_id(ctx)
            .and_then(|id| ctx.find_nearest_creature_with_string_id(STAR_BEAM_PARTNER_RANGE, id))
            .filter(|&partner| {
                ctx.world.get::<crate::creature_ai::CreatureState>(partner)
                    != Some(&crate::creature_ai::CreatureState::Returning)
            });
        match partner {
            Some(partner) => {
                ctx.do_cast_at(Some(partner), STAR_BEAM, false);
            }
            None => self
                .tasks
                .repeat_from_due_range(STAR_BEAM_RETRY_MS, STAR_BEAM_RETRY_MS),
        }
    }

    /// `SelectTarget(Random, 0, 0.0f, false, true, -SPELL_MARK_OF_SHADOW)`.
    fn unmarked_threat_target(ctx: &Ctx) -> Option<Entity> {
        let threat = ctx.world.get::<shared::components::ThreatTable>(ctx.me)?;
        let candidates: Vec<Entity> = threat
            .entries()
            .iter()
            .filter_map(|entry| Entity::try_from_bits(entry.entity))
            .filter(|&unit| {
                crate::spell_cast::targets::is_alive(ctx.world, unit)
                    && !crate::auras::has_aura(ctx.world, unit, MARK_OF_SHADOW)
            })
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let pick = rand::Rng::gen_range(&mut rand::thread_rng(), 0..candidates.len());
        Some(candidates[pick])
    }

    /// The copy's Keli'dan (`instance->GetCreature(DATA_KELIDAN)`).
    fn kelidan(ctx: &mut Ctx) -> Option<Entity> {
        let map = ctx.map()?;
        ctx.world
            .query::<(Entity, &crate::maps::InMap, &shared::components::Npc)>()
            .iter(ctx.world)
            .find(|(_, copy, npc)| **copy == map && npc.template_id == NPC_KELIDAN)
            .map(|(actor, ..)| actor)
    }

    fn notify_kelidan(ctx: &mut Ctx, action: i32) {
        if let Some(kelidan) = Self::kelidan(ctx) {
            super::dispatch(ctx.world, kelidan, Hook::DoAction(action));
        }
    }
}

impl CreatureAi for Channeler {
    fn initialize_ai(&mut self, ctx: &mut Ctx) {
        ctx.world
            .entity_mut(ctx.me)
            .insert(crate::combat::CorpseDelay(3.0));
        super::dispatch(ctx.world, ctx.me, Hook::Reset);
    }

    fn reset(&mut self, _ctx: &mut Ctx) {
        self.tasks.reset();
    }

    fn just_appeared(&mut self, _ctx: &mut Ctx) {
        self.schedule_star_beam();
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.tasks.reset();
        crate::spell_cast::cancel_cast(ctx.world, ctx.me);
        ctx.talk(SAY_CHANNELER_AGGRO_1);
        if rand::Rng::gen_bool(&mut rand::thread_rng(), CHANNELER_AGGRO_2_CHANCE) {
            ctx.talk(SAY_CHANNELER_AGGRO_2);
        }
        self.tasks.schedule_range(TASK_SHADOW_BOLT, 0, 5_000);
        self.tasks
            .schedule_range(TASK_MARK_OF_SHADOW, 5_000, 15_000);
    }

    fn just_reached_home(&mut self, ctx: &mut Ctx) {
        self.schedule_star_beam();
        Self::notify_kelidan(ctx, ACTION_RESET_ENCOUNTER);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        Self::notify_kelidan(ctx, ACTION_CHANNELER_DIES);
    }

    /// `TaskScheduler::Update` with the not-casting validator, then `UpdateVictim`.
    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.tasks.update(u64::from(diff_ms));
        while !ctx.is_casting()
            && let Some(task) = self.tasks.execute_event()
        {
            match task {
                TASK_STAR_BEAM => self.cast_star_beam(ctx),
                TASK_SHADOW_BOLT => {
                    ctx.do_cast_victim(ctx.dungeon_mode(SHADOW_BOLT, SHADOW_BOLT_H));
                    self.tasks.repeat_from_due_range(10_000, 15_000);
                }
                TASK_MARK_OF_SHADOW => {
                    if let Some(target) = Self::unmarked_threat_target(ctx) {
                        ctx.do_cast_at(Some(target), MARK_OF_SHADOW, false);
                    }
                    self.tasks.repeat_from_due_range(5_000, 15_000);
                }
                _ => {}
            }
        }
        ctx.update_victim();
    }
}

pub(super) fn register_aura_scripts(mut scripts: ResMut<AuraScripts>) {
    scripts.0.insert(
        BURNING_NOVA,
        AuraScript {
            on_apply: Some(burning_nova_vortex),
            on_remove: Some(burning_nova_fire_nova),
            ..Default::default()
        },
    );
}

fn is_heroic(world: &World, unit: Entity) -> bool {
    world
        .get::<crate::maps::InMap>(unit)
        .is_some_and(|&map| crate::zones::map_difficulty_id(world, map) == 2)
}

/// TC spell_kelidan_the_breaker_burning_nova AfterApply: Heroic triggered Vortex.
fn burning_nova_vortex(world: &mut World, target: Entity, _aura: &AuraInstance) {
    if is_heroic(world, target)
        && let Err(error) =
            crate::spell_cast::creature_cast(world, target, VORTEX, Some(target), true)
    {
        warn!("Keli'dan Vortex{VORTEX} failed: {error:?}");
    }
}

/// AfterRemove: an ordinary Fire Nova, the Heroic one in a Heroic copy.
fn burning_nova_fire_nova(
    world: &mut World,
    target: Entity,
    _aura: &AuraInstance,
    _reason: RemoveReason,
) {
    let nova = if is_heroic(world, target) {
        FIRE_NOVA_H
    } else {
        FIRE_NOVA
    };
    if let Err(error) = crate::spell_cast::creature_cast(world, target, nova, Some(target), false) {
        debug!("Keli'dan Fire Nova{nova} failed: {error:?}");
    }
}
