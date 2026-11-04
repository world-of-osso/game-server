//! SimC76f8118 sc_druid.cpp5355–5423: Lunar Beam heals once per ground pulse.
use bevy::prelude::*;

use super::combat;
use combat::auras::{
    self, AuraApplication, AuraInstance, AuraSource, procs::AuraEvent, scripts::AuraScripts,
};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{
    self, effects::EffectCtx, queued_triggers, script_registry::SpellScriptHook, targets,
};
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterForegroundSpecial(schedule_heals),
    SpellScriptHook::TargetFilter(limit_pulse_targets),
    SpellScriptHook::LateAuraRegistration(register_boundless_moonlight),
];

const BOUNDLESS_MOONLIGHT: u32 = 424058;
const BOUNDLESS_TICKER: u32 = 425217;
const BOUNDLESS_HEAL: u32 = 425206;

#[derive(Component)]
struct BoundlessMoonlight {
    center: Vec3,
    radius: f32,
    pending: f32,
}

fn register_boundless_moonlight(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(BOUNDLESS_TICKER).or_default();
    script.on_damage_proc = Some(bank_boundless_moonlight);
    script.on_periodic = Some(heal_boundless_moonlight);
    script.on_remove = Some(remove_boundless_moonlight);
}

// INFERRED_WCL dtmp7Lg2abzyZVBG fight 3 pull 1: 425206 heals on 1s
// intervals after Lunar Beam; 425217 effect 0 supplies its 12% rate.
// SimC76f8118 sc_druid.cpp12842–12862 accumulates eligible damage per tick.
fn start_boundless_moonlight(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == BOUNDLESS_MOONLIGHT)
    }) {
        return;
    }
    let Some(center) = spell.cast_destination.map(Vec3::from_array).or_else(|| {
        spell
            .cast_primary_target
            .and_then(|target| targets::position(world, target))
    }) else {
        return;
    };
    let radius = {
        let pulse = world
            .resource::<SpellStore>()
            .get(414613)
            .expect("Lunar Beam pulse");
        targets::radius(&pulse.effects[0], 1)
    };
    world.entity_mut(caster).insert(BoundlessMoonlight {
        center,
        radius,
        pending: 0.0,
    });
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: BOUNDLESS_TICKER,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
}

fn bank_boundless_moonlight(
    world: &mut World,
    caster: Entity,
    aura: &AuraInstance,
    event: &AuraEvent,
) {
    if event.actor != caster || event.damage <= 0.0 {
        return;
    }
    let Some(target) = event.target.and_then(|unit| targets::position(world, unit)) else {
        return;
    };
    let percent = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .map_or(0.0, |effect| effect.amount);
    if let Some(mut state) = world.get_mut::<BoundlessMoonlight>(caster) {
        if target.distance(state.center) <= state.radius {
            state.pending += event.damage * percent / 100.0;
        }
    }
}

fn heal_boundless_moonlight(world: &mut World, caster: Entity, _: &AuraInstance, index: u8) {
    if index != 0 {
        return;
    }
    let Some(mut state) = world.get_mut::<BoundlessMoonlight>(caster) else {
        return;
    };
    let amount = std::mem::take(&mut state.pending);
    if amount > 0.0 {
        spell_cast::cast_triggered_with_effect0_amount(
            world,
            caster,
            BOUNDLESS_HEAL,
            Some(caster),
            1,
            amount,
        );
    }
}

fn remove_boundless_moonlight(
    world: &mut World,
    caster: Entity,
    _: &AuraInstance,
    _: auras::RemoveReason,
) {
    world.entity_mut(caster).remove::<BoundlessMoonlight>();
}

// SimC76f8118 sc_druid.cpp5381–5385: pulse cap is Lunar Beam effect 3.
fn limit_pulse_targets(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: u8,
    _: Option<Entity>,
    targets: &mut Vec<Entity>,
) {
    if spell.id == 414613 && effect == 0 {
        let limit =
            spell_cast::scripts::script_effect_value(world, caster, 204066, 2).max(0.0) as usize;
        targets.truncate(limit);
    }
}

fn schedule_heals(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != 204066 {
        return;
    }
    start_boundless_moonlight(world, caster, spell);
    let duration = spell_cast::duration_ms(world, caster, spell).max(0) as u32;
    let Some(origin) = targets::position(world, caster) else {
        return;
    };
    let ctx = EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    for delay in (1000..=duration).step_by(1000) {
        queued_triggers::schedule_destination_trigger(
            world,
            &ctx,
            queued_triggers::DestinationTrigger {
                target: caster,
                spell_id: 414613,
                delay_ms: delay,
                destination: origin.to_array(),
            },
        );
        queued_triggers::schedule_full_trigger(world, &ctx, caster, 204069, delay);
    }
}
