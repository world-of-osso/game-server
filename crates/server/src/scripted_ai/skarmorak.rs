//! TC a352b1fa TheStoneVault/boss_skarmorak.cpp:82–216,244–261.
//! Normal/projected205 parent/energy/Shell channel partial; native70448 effects.
//! Shards/fragments/fortified target controllers remain sourced-but-unported.
use bevy::prelude::*;
use shared::components::PowerType;

use super::boss_ai::BossAi;
use super::{CreatureAi, Ctx};
use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::auras::{ActiveAuras, AuraInstance, RemoveReason};
use crate::spell_cast::powers::{change_power, power_value};

pub(super) const SCRIPT: &str = "boss_skarmorak";
const SMASH: u32 = 422233;
const CRASH: u32 = 423538;
const ENERGY_CONTROLLER: u32 = 439749;
const SHELL_ENERGIZE: u32 = 429976;
const SHELL: u32 = 423200;
const SHELL_SELECTOR: u32 = 426181;
const SHELL_ABSORB: u32 = 423228;
const SHATTERED_SHELL: u32 = 423246;
const ACTION_FORTIFIED_SHELL: i32 = 1;
const SHELL_INDICATOR: u32 = 445409;
const DISCHARGE: u32 = 423324;
const DISCHARGE_STACKS: u32 = 423327;
const CHECK_ENERGY: u32 = u32::MAX;

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = *world.get::<crate::maps::InMap>(actor)?;
    let npc = world.get::<shared::components::Npc>(actor)?;
    if map.map_id != 2652 || npc.template_id != 210156 {
        return None;
    }
    if !matches!(crate::zones::map_difficulty_id(world, map), 1 | 205) {
        return None;
    }
    Some(Box::new(Skarmorak {
        boss: super::instance_script::bound_boss(world, actor)?,
    }))
}

struct Skarmorak {
    boss: BossAi,
}

impl Skarmorak {
    fn shell_ended(&mut self, ctx: &mut Ctx) {
        for spell in [SHELL_INDICATOR, DISCHARGE, DISCHARGE_STACKS] {
            remove_owned_aura(ctx.world, ctx.me, spell);
        }
        self.boss.events.schedule(SMASH, 10_000);
        self.boss.events.schedule(CRASH, 16_100);
        self.boss.events.schedule(CHECK_ENERGY, 500);
    }

    fn check_energy(&mut self, ctx: &mut Ctx) {
        let current = power_value(ctx.world, ctx.me, PowerType::Energy)
            .expect("Skarmorak sourceclass4 requires native Energy ingress")
            .0;
        if current <= 0.0 {
            ctx.talk(2);
            for spell in [SHELL_ENERGIZE, SHELL, SHELL_SELECTOR] {
                cast_source_shell(ctx, spell);
            }
            for event in [SMASH, CRASH, CHECK_ENERGY] {
                self.boss.events.cancel(event);
            }
            self.boss.events.schedule(DISCHARGE, 3_000);
        }
        // TC literally Repeat(5s) even after CancelEvent(CHECK_ENERGY).
        self.boss.events.schedule(CHECK_ENERGY, 5_000);
    }

    fn execute_event(&mut self, ctx: &mut Ctx, spell: u32) {
        match spell {
            SMASH => {
                ctx.do_cast_victim(SMASH);
                self.boss.events.schedule(SMASH, 17_000);
            }
            CRASH => {
                ctx.do_cast_self(CRASH);
                self.boss.events.schedule(CRASH, 19_400);
            }
            CHECK_ENERGY => self.check_energy(ctx),
            DISCHARGE => {
                ctx.talk(3);
                ctx.do_cast_self(DISCHARGE);
            }
            _ => {}
        }
    }
}

impl CreatureAi for Skarmorak {
    fn encounter_schedule(&self) -> Option<&super::EventMap> {
        Some(&self.boss.events)
    }
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
        ctx.talk(0);
        ctx.do_cast_self(ENERGY_CONTROLLER);
        self.boss.events.schedule(SMASH, 4_600);
        self.boss.events.schedule(CRASH, 10_500);
        self.boss.events.schedule(CHECK_ENERGY, 500);
    }

    fn do_action(&mut self, ctx: &mut Ctx, action: i32) {
        if action != ACTION_FORTIFIED_SHELL {
            return;
        }
        // TC DoAction explicitly uses TRIGGERED_FULL_MASK for Shattered Shell.
        ctx.do_cast_at(Some(ctx.me), SHATTERED_SHELL, true);
        self.shell_ended(ctx);
    }

    fn channel_finished(&mut self, ctx: &mut Ctx, spell: u32) {
        if spell == DISCHARGE {
            remove_owned_aura(ctx.world, ctx.me, SHELL_ABSORB);
            self.shell_ended(ctx);
        }
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
        ctx.talk(5);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if !ctx.update_victim() {
            return;
        }
        self.boss.events.update(u64::from(diff_ms));
        if ctx.is_casting() {
            return;
        }
        // Source UpdateAI uses switch(ExecuteEvent()), not the generic while loop.
        if let Some(spell) = self.boss.events.execute_event() {
            self.execute_event(ctx, spell);
        }
    }
}

fn cast_source_shell(ctx: &mut Ctx, spell: u32) {
    // Exact TC IGNORE_CAST_IN_PROGRESS, not FULL_MASK or cooldown/resource bypass.
    if let Err(reason) = crate::spell_cast::creature_cast_ignoring_in_progress(
        ctx.world,
        ctx.me,
        spell,
        Some(ctx.me),
    ) {
        warn!("Skarmorak source Shell request{spell} refused: {reason:?}");
    }
}

fn remove_owned_aura(world: &mut World, actor: Entity, spell: u32) {
    let instance = world.get::<ActiveAuras>(actor).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == spell && aura.caster == actor)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        crate::auras::remove_instance(world, actor, instance, RemoveReason::Cancelled);
    }
}

#[derive(Component)]
struct ShellTicks {
    instance: u32,
    tick: u32,
}

pub(super) fn register_aura_scripts(mut scripts: ResMut<AuraScripts>) {
    scripts.0.insert(
        SHELL_ABSORB,
        AuraScript {
            on_remove: Some(shell_absorb_removed),
            ..Default::default()
        },
    );
    scripts.0.insert(
        SHELL_ENERGIZE,
        AuraScript {
            on_periodic: Some(shell_energize_tick),
            ..Default::default()
        },
    );
}

fn shell_absorb_removed(
    world: &mut World,
    unit: Entity,
    _aura: &AuraInstance,
    reason: RemoveReason,
) {
    // TC Unit.cpp1933–1936 depleted absorb and4034–4039 dispel both use
    // AURA_REMOVE_BY_ENEMY_SPELL; cleanup/expiry/death must not break Shell.
    if !matches!(reason, RemoveReason::Consumed | RemoveReason::Dispelled) {
        return;
    }
    let Some(map) = world.get::<crate::maps::InMap>(unit).copied() else {
        return;
    };
    let actor = world
        .query::<(Entity, &crate::maps::InMap, &shared::components::Npc)>()
        .iter(world)
        .find_map(|(actor, in_map, npc)| {
            (*in_map == map && npc.template_id == 210156).then_some(actor)
        });
    if let Some(actor) = actor {
        super::dispatch(world, actor, super::Hook::DoAction(ACTION_FORTIFIED_SHELL));
    }
}

fn shell_energize_tick(world: &mut World, unit: Entity, aura: &AuraInstance, index: u8) {
    if index != 0 {
        return;
    }
    let tick = match world.get_mut::<ShellTicks>(unit) {
        Some(mut state) if state.instance == aura.instance_id => {
            state.tick += 1;
            state.tick
        }
        _ => {
            world.entity_mut(unit).insert(ShellTicks {
                instance: aura.instance_id,
                tick: 1,
            });
            1
        }
    };
    let cycle = [16.0, 17.0, 17.0];
    change_power(
        world,
        unit,
        PowerType::Energy,
        cycle[tick as usize % cycle.len()],
    );
}
