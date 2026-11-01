//! SimC76f8118 unique_gear_dragonflight.cpp7404–7467: equipped Tome of Unstable Power.
use super::combat;
use bevy::prelude::*;
use combat::auras::procs::{self, AuraEvent};
use combat::auras::{self, AuraInstance};
use combat::random::Rng;
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::{self, scripts::soft_cap_multiplier};
use combat::spell_info::SpellStore;

const DRIVER: u32 = 432777;
const BARRAGE: u32 = 434021;
const CHANNEL: u32 = 434070;
/// SimC hardcodes the Barrage's reduced_aoe_targets; native data has none.
const REDUCED_TARGETS: f32 = 8.0;

struct PendingBarrage {
    at: f64,
    target: Entity,
    amount: f32,
    item_level: Option<u16>,
}

#[derive(Component, Default)]
struct PendingBarrages(Vec<PendingBarrage>);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::AfterNativeCastUpdate(update),
    SpellScriptHook::TargetContextDamageMultiplier(reduce_area),
];

fn register(scripts: &mut auras::scripts::AuraScripts) {
    let driver = scripts.0.entry(DRIVER).or_default();
    driver.check_proc = Some(check_proc);
    driver.on_proc = Some(schedule_barrage);
}

fn check_proc(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.spell_id != Some(BARRAGE)
        && procs::select_proc_trigger_target(world, aura.caster, BARRAGE, event.target).is_ok()
}

fn schedule_barrage(
    world: &mut World,
    caster: Entity,
    aura: &AuraInstance,
    target: Option<Entity>,
) {
    let target = target.expect("qualified Tome proc recipient");
    let amount = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Tome driver requires its item-scaled damage budget")
        .amount;
    // SimC fires one ground pulse after the native image channel's duration.
    let channel_ms = world
        .resource::<SpellStore>()
        .get(CHANNEL)
        .and_then(|spell| spell.misc.as_ref())
        .and_then(|misc| misc.duration)
        .expect("native Tome channel duration")
        .duration_ms;
    let at = spell_cast::now(world) + f64::from(channel_ms) / 1000.0;
    world
        .entity_mut(caster)
        .entry::<PendingBarrages>()
        .or_default()
        .into_mut()
        .0
        .push(PendingBarrage {
            at,
            target,
            amount,
            item_level: aura.cast_item_level,
        });
}

fn update(world: &mut World) {
    let now = spell_cast::now(world);
    let casters: Vec<_> = world
        .query_filtered::<Entity, With<PendingBarrages>>()
        .iter(world)
        .collect();
    for caster in casters {
        let due = {
            let mut pending = world.get_mut::<PendingBarrages>(caster).unwrap();
            let (due, later) = std::mem::take(&mut pending.0)
                .into_iter()
                .partition(|barrage| barrage.at <= now);
            pending.0 = later;
            due
        };
        for barrage in due {
            fire_barrage(world, caster, barrage);
        }
    }
}

fn fire_barrage(world: &mut World, caster: Entity, barrage: PendingBarrage) {
    let Some(position) = spell_cast::targets::position(world, barrage.target) else {
        return;
    };
    // SimC snapshot_state: 0.9 + 0.1 × uniform{0,1,2}; the five-image case is unmodeled there.
    let images = combat::random::thread_rng().gen_range(0..3u8);
    let multiplier = 0.9 + 0.1 * f32::from(images);
    let mut spell = spell_cast::load_triggered_spell(world, BARRAGE, 1).expect("native Barrage");
    // Effect1 is a dummy back-reference to the driver; only the damage executes.
    spell.effects.retain(|effect| effect.index == 0);
    let damage = spell
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("native Barrage damage effect");
    damage.base_points = barrage.amount * multiplier;
    spell.cast_effect0_amount = Some(damage.base_points);
    spell.cast_item_level = barrage.item_level;
    spell.cast_destination = Some(position.to_array());
    if !spell_cast::launch_triggered_spell(world, caster, &spell, Some(barrage.target), 1) {
        warn!(?caster, "Tome Unstable Barrage was refused");
    }
}

fn reduce_area(world: &World, ctx: &EffectCtx<'_>, _: Entity, periodic: bool) -> f32 {
    if periodic || ctx.spell.id != BARRAGE {
        return 1.0;
    }
    let count = spell_cast::resolving_targets(world, ctx)
        .expect("native Barrage retains resolved targets")
        .len();
    soft_cap_multiplier(count, REDUCED_TARGETS)
}
