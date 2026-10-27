//! Source-qualified Fiendish Cruelty from actual Destruction critical impacts.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraSource};
use combat::random as rand;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;
use shared::components::ActiveSpec;

const FIENDISH: u32 = 1245633;
const BUFF: u32 = 1245664;
const QUALIFYING: &[u32] = &[29722, 116858, 17962, 434506];

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(owns_proc),
    SpellScriptHook::AfterPolicyDirectDamage(attempt_fiendish),
];

#[derive(Component)]
struct FiendishProgress {
    passive: u32,
    attempts: u32,
}

fn owns_proc(spell: u32) -> bool {
    spell == FIENDISH
}

fn attempt_fiendish(world: &mut World, ctx: &EffectCtx<'_>, _: Entity, critical: bool) {
    let destruction = world
        .get::<ActiveSpec>(ctx.caster)
        .is_some_and(|spec| spec.0 == 267);
    if !destruction || !critical || !QUALIFYING.contains(&ctx.spell.id) {
        return;
    }
    let instance = world.get::<ActiveAuras>(ctx.caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.caster == ctx.caster && aura.spell_id == FIENDISH)
            .map(|aura| aura.instance_id)
    });
    let Some(instance) = instance else {
        return;
    };
    // SimC76f8118 actions433–439/3860/3936/4293/4446/5209 and
    // init1323–1326: eligible actual critical impacts attempt native chance
    // through the source accumulated distribution, shared across actions.
    let chance = world
        .resource::<SpellStore>()
        .get(FIENDISH)
        .expect("selected native Fiendish")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Fiendish chance")
        .base_points
        / 100.0;
    if roll_fiendish(world, ctx.caster, instance, f64::from(chance)) {
        auras::apply_aura(
            world,
            &AuraApplication {
                caster: ctx.caster,
                target: ctx.caster,
                spell_id: BUFF,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
}

fn roll_fiendish(world: &mut World, caster: Entity, passive: u32, chance: f64) -> bool {
    if world
        .get::<FiendishProgress>(caster)
        .is_none_or(|state| state.passive != passive)
    {
        world.entity_mut(caster).insert(FiendishProgress {
            passive,
            attempts: 0,
        });
    }
    let constant = rand::accumulated_proc_constant(chance);
    let mut state = world.get_mut::<FiendishProgress>(caster).unwrap();
    state.attempts = state.attempts.saturating_add(1);
    let success = rand::random::<f64>() < constant * f64::from(state.attempts);
    if success {
        state.attempts = 0;
    }
    success
}
