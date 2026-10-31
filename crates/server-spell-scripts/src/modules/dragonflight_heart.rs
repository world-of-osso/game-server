//! SimC76f8118 unique_gear_dragonflight.cpp7575–7685: equipped Heart proc.
use super::combat;
use bevy::prelude::*;
use combat::auras::procs::{self, AuraEvent};
use combat::auras::{self, AuraInstance, RemoveReason};
use combat::random::Rng;
use combat::spell_cast::effects::{self, EffectCtx};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::{self, TriggeredSpellOverrides};
use combat::spell_info::SpellStore;
use shared::components::{Health, Npc};

const DRIVER: u32 = 432699;
const SICKNESS: u32 = 433522;
const JUMPS: u32 = 433768;
const DUNGEON_BOSS_FLAG: u32 = 0x1000_0000;
const WORLD_BOSS_FLAG: u32 = 0x4;

#[derive(Component, Default)]
struct HeartJumps(u32);

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut auras::scripts::AuraScripts) {
    let driver = scripts.0.entry(DRIVER).or_default();
    driver.check_proc = Some(check_proc);
    driver.on_proc = Some(proc_heart);
    let sickness = scripts.0.entry(SICKNESS).or_default();
    sickness.on_apply = Some(start_sickness);
    sickness.on_refresh = Some(cap_sickness_stacks);
    sickness.on_remove = Some(end_sickness);
    sickness.after_periodic_damage = Some(tick_sickness);
}

fn check_proc(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.spell_id != Some(SICKNESS)
        && procs::select_proc_trigger_target(world, aura.caster, SICKNESS, event.target).is_ok()
}

fn proc_heart(world: &mut World, caster: Entity, aura: &AuraInstance, target: Option<Entity>) {
    let amount = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Heart driver requires its item-scaled damage budget")
        .amount;
    apply_sickness(world, caster, target, aura.cast_item_level, amount);
}

fn apply_sickness(
    world: &mut World,
    caster: Entity,
    target: Option<Entity>,
    level: Option<u16>,
    amount: f32,
) {
    if !spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        SICKNESS,
        target,
        1,
        TriggeredSpellOverrides {
            effect0_amount: Some(amount),
            item_level: level,
            ..Default::default()
        },
    ) {
        warn!(?caster, ?target, "Heart Crystal Sickness cast was refused");
    }
}

fn start_sickness(world: &mut World, target: Entity, aura: &AuraInstance) {
    cap_sickness_stacks(world, target, aura);
    let max = world
        .resource::<SpellStore>()
        .get(JUMPS)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("Heart jump counter requires native stack data")
        .cumulative_aura;
    let mut caster = world.entity_mut(aura.caster);
    let mut count = caster.entry::<HeartJumps>().or_default().into_mut();
    count.0 += 1;
    if count.0 >= max {
        count.0 = 0;
    }
}

fn cap_sickness_stacks(world: &mut World, target: Entity, aura: &AuraInstance) {
    // Native refresh restores the data cap; SimC always subtracts one.
    if !auras::limit_owned_stacks(
        world,
        aura.caster,
        target,
        SICKNESS,
        aura.max_stacks.saturating_sub(1),
    ) {
        warn!(?target, "Heart Crystal Sickness aura was not found");
    }
}

fn end_sickness(world: &mut World, _target: Entity, aura: &AuraInstance, reason: RemoveReason) {
    if matches!(reason, RemoveReason::Expired | RemoveReason::Death)
        && let Some(mut count) = world.get_mut::<HeartJumps>(aura.caster)
    {
        count.0 = 0;
    }
}

fn tick_sickness(world: &mut World, target: Entity, aura: &AuraInstance, effect: u8) {
    if effect != 0 {
        return;
    }
    if !auras::set_owned_stacks(
        world,
        aura.caster,
        target,
        SICKNESS,
        aura.stacks.saturating_add(1),
    ) {
        warn!(
            ?target,
            "Heart Crystal Sickness disappeared before its stack update"
        );
        return;
    }
    let Some(next) = choose_jump_target(world, aura.caster, target) else {
        return;
    };
    execute_and_jump(world, target, next, aura);
}

fn execute_and_jump(world: &mut World, target: Entity, next: Entity, aura: &AuraInstance) {
    let health = world
        .get::<Health>(target)
        .expect("sickness target health")
        .current;
    let amount = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("sickness periodic damage")
        .amount;
    auras::remove_instance(world, target, aura.instance_id, RemoveReason::Consumed);
    let spell = world
        .resource::<SpellStore>()
        .get(SICKNESS)
        .expect("sickness spell")
        .clone();
    let ctx = EffectCtx {
        caster: aura.caster,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    effects::deal_copied_damage(world, &ctx, target, health);
    apply_sickness(world, aura.caster, Some(next), aura.cast_item_level, amount);
}

fn choose_jump_target(world: &World, caster: Entity, target: Entity) -> Option<Entity> {
    if world
        .get::<HeartJumps>(caster)
        .is_none_or(|count| count.0 == 0)
        || is_boss(world, target)
    {
        return None;
    }
    let threshold = combat::spell_cast::scripts::script_effect_value(world, caster, DRIVER, 1);
    let health = world.get::<Health>(target)?;
    if health.current > health.max * (threshold / 100.0) {
        return None;
    }
    let targets: Vec<_> = combat::combat_refs::active_opponents(world, caster)
        .filter(|candidate| {
            *candidate != target && combat::spell_cast::targets::is_alive(world, *candidate)
        })
        .collect();
    (!targets.is_empty()).then(|| targets[combat::random::thread_rng().gen_range(0..targets.len())])
}

fn is_boss(world: &World, target: Entity) -> bool {
    let Some(npc) = world.get::<Npc>(target) else {
        return false;
    };
    world
        .resource::<combat::world_data::WorldData>()
        .templates
        .get(&npc.template_id)
        .is_some_and(|template| {
            template.flags_extra & DUNGEON_BOSS_FLAG != 0
                || template.type_flags & WORLD_BOSS_FLAG != 0
        })
}
