//! Frantic Momentum is earned from Feral finishers, not arbitrary damage.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::random::Rng;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::{ActiveSpec, PowerType};

const TALENT: u32 = 391875;
const BUFF: u32 = 391876;
const DRUID_CLASS_SET: u32 = 7;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(owns_proc),
    SpellScriptHook::PolicyContextRequired(needs_context),
    SpellScriptHook::AfterElementalLaunch(after_launch),
];

#[derive(Component, Default)]
struct FranticMomentumReady(f64);

fn owns_proc(spell: u32) -> bool {
    spell == TALENT
}

fn finisher(world: &World, actor: Entity, spell: &SpellInfo) -> bool {
    world
        .get::<ActiveSpec>(actor)
        .is_some_and(|spec| spec.0 == 103)
        && spell
            .class_options
            .as_ref()
            .is_some_and(|class| class.spell_class_set == DRUID_CLASS_SET)
        && spell
            .power_costs
            .iter()
            .any(|cost| cost.power_type == PowerType::ComboPoints as i32)
        && world.get::<ActiveAuras>(actor).is_some_and(|auras| {
            auras
                .iter()
                .any(|a| a.spell_id == TALENT && a.caster == actor)
        })
}

fn needs_context(world: &World, actor: Entity, spell: &SpellInfo) -> bool {
    finisher(world, actor, spell)
}

// SimC76f8118 sc_druid.cpp3818–3844/11272–11276: each finisher
// rolls the native effect1 chance per effective point and the native ICD.
fn after_launch(world: &mut World, actor: Entity, spell: &SpellInfo, _: bool) {
    if !finisher(world, actor, spell) {
        return;
    }
    let Some(points) = spell.cast_policy_context.as_ref().and_then(|context| {
        context
            .resources
            .iter()
            .find(|(power, _)| *power == PowerType::ComboPoints)
            .map(|(_, points)| *points)
    }) else {
        return;
    };
    if points <= 0.0 {
        return;
    }
    let now = spell_cast::now(world);
    if world
        .get::<FranticMomentumReady>(actor)
        .is_some_and(|state| now < state.0)
    {
        return;
    }
    let talent = world
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == TALENT && a.caster == actor)
        .unwrap();
    let chance = talent
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Frantic Momentum chance")
        .amount;
    if combat::random::thread_rng().gen_range(0.0..100.0) >= chance * points {
        return;
    }
    let cooldown = world
        .resource::<combat::spell_info::SpellStore>()
        .get(TALENT)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("native Frantic Momentum proc options")
        .proc_category_recovery_ms as f64
        / 1000.0;
    world
        .entity_mut(actor)
        .insert(FranticMomentumReady(now + cooldown));
    spell_cast::cast_triggered(world, actor, BUFF, Some(actor), 1);
}
