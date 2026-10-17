//! TC a352b1fa Maraudon bosses; TaskScheduler deadlines and aura effect0 hooks.
use bevy::prelude::*;
use shared::components::{Npc, UnitFlags};

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::{CreatureAi, Ctx, DamageTaken, ReactState};
use crate::auras::{
    AuraInstance, RemoveReason,
    handlers::DUMMY,
    scripts::{AuraScript, AuraScripts},
};

const THERADRAS_SCRIPT: &str = "boss_princess_theradras";
const DUST_FIELD: u32 = 21909;
const BOULDER: u32 = 21832;
const THRASH: u32 = 3391;
const REPULSIVE_GAZE: u32 = 21869;
const ZAETARS_SPIRIT: u32 = 12238;
const NOXXION_SCRIPT: &str = "boss_noxxion";
const CELEBRAS_SCRIPT: &str = "boss_celebras_the_cursed";
const WRATH: u32 = 21807;
const ENTANGLING_ROOTS: u32 = 12747;
const CORRUPT_FORCES: u32 = 21968;
const CELEBRAS_REDEEMED: u32 = 13716;
const LANDSLIDE_SCRIPT: &str = "boss_landslide";
const KNOCK_AWAY: u32 = 18670;
const TRAMPLE: u32 = 5568;
const LANDSLIDE: u32 = 21808;
const TOXIC_VOLLEY: u32 = 21687;
const UPPERCUT: u32 = 22916;
const SUMMON_DUMMY: u32 = 21708;
const SUMMON_SPAWNS: u32 = 21707;

pub(super) fn is_creature_script(name: &str) -> bool {
    matches!(
        name,
        NOXXION_SCRIPT | CELEBRAS_SCRIPT | LANDSLIDE_SCRIPT | THERADRAS_SCRIPT
    )
}

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = world.get::<crate::maps::InMap>(actor)?;
    let instance = world
        .get_resource::<crate::maps::MapStore>()?
        .instance_scripts
        .get(&map.map_id)?;
    if instance != "instance_maraudon" {
        return None;
    }
    match world.get::<super::CreatureScriptName>(actor)?.0.as_str() {
        NOXXION_SCRIPT => Some(Box::new(Noxxion {
            boss: BossAi::new(0),
        })),
        CELEBRAS_SCRIPT => Some(Box::new(Celebras {
            boss: BossAi::new(3),
        })),
        LANDSLIDE_SCRIPT => Some(Box::new(Landslide {
            boss: BossAi::new(4),
            landslide: false,
        })),
        THERADRAS_SCRIPT => Some(Box::new(Theradras {
            boss: BossAi::new(7),
        })),
        _ => None,
    }
}

struct Theradras {
    boss: BossAi,
}

impl CreatureAi for Theradras {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule_range(DUST_FIELD, 20_000, 25_000);
        self.boss.events.schedule_range(BOULDER, 20_000, 30_000);
        self.boss.events.schedule_range(THRASH, 10_000, 20_000);
        self.boss
            .events
            .schedule_range(REPULSIVE_GAZE, 25_000, 35_000);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        // The pinned source supplies fixed WoW coordinates, not the corpse position.
        ctx.summon_creature(ZAETARS_SPIRIT, [28.1887, 62.3964, -123.161, 4.31096]);
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

impl BossScript for Theradras {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let repeat = match event {
            DUST_FIELD => {
                ctx.do_cast_self(DUST_FIELD);
                (20_000, 25_000)
            }
            BOULDER => {
                if let Some(target) = ctx.select_target_random(0.0, false) {
                    ctx.do_cast_at(Some(target), BOULDER, false);
                }
                (15_000, 25_000)
            }
            THRASH => {
                ctx.do_cast_self(THRASH);
                (10_000, 20_000)
            }
            REPULSIVE_GAZE => {
                ctx.do_cast_self(REPULSIVE_GAZE);
                (30_000, 40_000)
            }
            _ => return,
        };
        self.boss.events.repeat_from_due_range(repeat.0, repeat.1);
    }
}

struct Noxxion {
    boss: BossAi,
}

impl CreatureAi for Noxxion {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule_range(TOXIC_VOLLEY, 5_000, 10_000);
        self.boss.events.schedule_range(UPPERCUT, 15_000, 20_000);
        self.boss
            .events
            .schedule_range(SUMMON_DUMMY, 30_000, 40_000);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }
    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        self.boss.just_summoned(ctx, summon);
    }
    fn summoned_creature_despawn(&mut self, _ctx: &mut Ctx, summon: Entity) {
        self.boss.summoned_creature_despawn(summon);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        // BossAI's scheduler validator defers dispatch while casting; its clock
        // still advances. Passive UpdateVictim keeps this engaged scheduler alive.
        boss_update_ai(self, ctx, diff_ms);
    }
}

impl BossScript for Noxxion {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let (min, max) = match event {
            TOXIC_VOLLEY => {
                ctx.do_cast_self(TOXIC_VOLLEY);
                (15_000, 20_000)
            }
            UPPERCUT => {
                ctx.do_cast_victim(UPPERCUT);
                (20_000, 30_000)
            }
            SUMMON_DUMMY => {
                ctx.do_cast_self(SUMMON_DUMMY);
                (50_000, 60_000)
            }
            _ => return,
        };
        self.boss.events.repeat_from_due_range(min, max);
    }
}

struct Celebras {
    boss: BossAi,
}

impl CreatureAi for Celebras {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule_range(WRATH, 0, 6_000);
        self.boss.events.schedule_range(ENTANGLING_ROOTS, 0, 5_000);
        self.boss.events.schedule(CORRUPT_FORCES, 30_000);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
        let position = *ctx
            .world
            .get::<shared::components::Position>(ctx.me)
            .expect("boss position");
        ctx.summon_creature(
            CELEBRAS_REDEEMED,
            [position.x, -position.z, position.y, 0.0],
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

impl BossScript for Celebras {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let repeat = match event {
            WRATH => {
                ctx.do_cast_victim(WRATH);
                (4_000, 8_000)
            }
            ENTANGLING_ROOTS => {
                ctx.do_cast_victim(ENTANGLING_ROOTS);
                (20_000, 20_000)
            }
            CORRUPT_FORCES => {
                ctx.do_cast_self(CORRUPT_FORCES);
                (20_000, 20_000)
            }
            _ => return,
        };
        self.boss.events.repeat_from_due_range(repeat.0, repeat.1);
    }
}

#[cfg(test)]
pub(super) fn seeded_landslide(seed: u64) -> Box<dyn CreatureAi> {
    let mut boss = BossAi::new(4);
    boss.events = super::EventMap::seeded(seed);
    Box::new(Landslide {
        boss,
        landslide: false,
    })
}

struct Landslide {
    boss: BossAi,
    landslide: bool,
}

impl CreatureAi for Landslide {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
        self.landslide = false;
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        self.boss.events.schedule_range(KNOCK_AWAY, 5_000, 10_000);
        self.boss.events.schedule_range(TRAMPLE, 10_000, 15_000);
    }

    fn damage_taken(&mut self, _ctx: &mut Ctx, _attacker: Option<Entity>, damage: DamageTaken) {
        if !self.landslide && damage.health_below_pct_damaged(50.0) {
            self.landslide = true;
            self.boss.events.schedule(LANDSLIDE, 0);
        }
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
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

impl BossScript for Landslide {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, ctx: &mut Ctx, event: u32) {
        let repeat = match event {
            KNOCK_AWAY => {
                ctx.do_cast_victim(KNOCK_AWAY);
                (15_000, 25_000)
            }
            TRAMPLE => {
                ctx.do_cast_self(TRAMPLE);
                (10_000, 20_000)
            }
            LANDSLIDE => {
                ctx.do_cast_self(LANDSLIDE);
                (30_000, 40_000)
            }
            _ => return,
        };
        self.boss.events.repeat_from_due_range(repeat.0, repeat.1);
    }
}

pub(super) fn register_aura_scripts(mut scripts: ResMut<AuraScripts>) {
    scripts.0.insert(
        SUMMON_DUMMY,
        AuraScript {
            on_apply: Some(apply_spawn_phase),
            on_remove: Some(remove_spawn_phase),
            ..Default::default()
        },
    );
}

fn is_creature_dummy_zero(world: &World, target: Entity, aura: &AuraInstance) -> bool {
    world.get::<Npc>(target).is_some()
        && aura
            .effects
            .iter()
            .any(|effect| effect.index == 0 && effect.aura_type == DUMMY)
}

fn apply_spawn_phase(world: &mut World, target: Entity, aura: &AuraInstance) {
    if !is_creature_dummy_zero(world, target, aura) {
        return;
    }
    world.entity_mut(target).insert(ReactState::Passive);
    world
        .get_mut::<UnitFlags>(target)
        .expect("creature unit flags")
        .0 |= UnitFlags::NOT_SELECTABLE;
    if let Err(error) =
        crate::spell_cast::creature_cast(world, target, SUMMON_SPAWNS, Some(target), true)
    {
        warn!("Noxxion source21707 triggered summon failed: {error:?}");
    }
}

fn remove_spawn_phase(
    world: &mut World,
    target: Entity,
    aura: &AuraInstance,
    _reason: RemoveReason,
) {
    if !is_creature_dummy_zero(world, target, aura) {
        return;
    }
    world.entity_mut(target).insert(ReactState::Aggressive);
    world
        .get_mut::<UnitFlags>(target)
        .expect("creature unit flags")
        .0 &= !UnitFlags::NOT_SELECTABLE;
}
