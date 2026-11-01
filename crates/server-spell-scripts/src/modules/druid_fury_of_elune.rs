//! SimC76f8118 sc_druid.cpp7349–7437: Fury of Elune pulses its native area child.
use bevy::prelude::*;
use shared::components::Position;

use super::combat;
use combat::spell_cast::{
    self,
    area_trigger::SpellAreaTrigger,
    effects::EffectCtx,
    script_registry::{ScriptReach, SpellScriptHook},
    targets,
};
use combat::spell_info::{SpellInfo, SpellStore};

const FURY: u32 = 202770;
const DAMAGE: u32 = 211545;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::AfterNativeCastUpdate(update_fields),
    SpellScriptHook::TargetContextDamageMultiplier(reduce_secondary_targets),
];

#[derive(Component)]
struct FuryPulse(f64);

/// Native 202770 effect1 places the field its radius from the target toward the
/// caster (TARGET_DEST_DEST_TARGET_TOWARDS_CASTER); each 211545 pulse hits its
/// area radius around the field.
fn declared_reach(world: &World, _caster: Entity, spell: &SpellInfo) -> ScriptReach {
    if spell.id != FURY {
        return ScriptReach::None;
    }
    let field = spell
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("native Fury of Elune field effect");
    let pulse = world
        .resource::<SpellStore>()
        .get(DAMAGE)
        .expect("native Fury of Elune damage child");
    ScriptReach::target(targets::radius(field, 0) + targets::radius(&pulse.effects[0], 1))
}

fn pulse_period(world: &World, caster: Entity) -> f64 {
    let period = world
        .resource::<SpellStore>()
        .get(FURY)
        .and_then(|spell| spell.effects.iter().find(|effect| effect.index == 2))
        .expect("Fury of Elune native astral-power period")
        .aura_period_ms;
    assert!(period > 0, "Fury of Elune must have a pulse period");
    let haste = world
        .get::<combat::stats::CharacterStats>(caster)
        .expect("Fury of Elune caster stats")
        .haste_pct;
    f64::from(period) / 1000.0 / (1.0 + f64::from(haste) / 100.0)
}

fn update_fields(world: &mut World) {
    let now = spell_cast::now(world);
    let fields: Vec<_> = world
        .query::<(Entity, &SpellAreaTrigger, &Position, Option<&FuryPulse>)>()
        .iter(world)
        .filter(|(_, field, _, _)| {
            field.spell_id == FURY && field.expires_at.is_none_or(|end| now < end)
        })
        .map(|(entity, field, position, pulse)| {
            (
                entity,
                field.caster,
                field.primary_target,
                *position,
                pulse.map(|p| p.0),
            )
        })
        .collect();
    for (entity, caster, primary, position, next) in fields {
        let period = pulse_period(world, caster);
        let Some(next) = next else {
            world.entity_mut(entity).insert(FuryPulse(now + period));
            continue;
        };
        if now < next {
            continue;
        }
        let mut spell =
            spell_cast::load_triggered_spell(world, DAMAGE, 1).expect("Fury of Elune damage child");
        spell.cast_destination = Some([position.x, position.y, position.z]);
        spell_cast::launch_triggered_spell_with_primary(world, caster, &spell, None, primary, 1);
        world.entity_mut(entity).insert(FuryPulse(next + period));
    }
}

/// SimC leaves the primary at full damage and soft-caps secondary hits at one target.
fn reduce_secondary_targets(world: &World, ctx: &EffectCtx, target: Entity, _: bool) -> f32 {
    if ctx.spell.id != DAMAGE {
        return 1.0;
    }
    // Native child effect 1 also targets the caster; it is not an AoE damage recipient.
    let targets: Vec<_> = spell_cast::resolving_targets(world, ctx)
        .expect("Fury pulse targets")
        .iter()
        .copied()
        .filter(|unit| *unit != ctx.caster)
        .collect();
    let full = ctx
        .spell
        .cast_primary_target
        .filter(|primary| targets.contains(primary))
        .or_else(|| targets.first().copied());
    if Some(target) == full || targets.len() <= 1 {
        1.0
    } else {
        (1.0 / targets.len().min(20) as f32).sqrt()
    }
}
