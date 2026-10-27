//! Native Trickster Cloud Cover window and its earned Fazed applications.
use std::collections::HashMap;
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraSource};
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;

const TALENT: u32 = 441429;
const WINDOW: u32 = 441640;
const CLOUD: u32 = 441587;
const FAZED: u32 = 441224;

#[derive(Component, Default)]
struct CloudCoverTargets(HashMap<Entity, f64>);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterNativeOnCast(open_window),
    SpellScriptHook::LandedDamagePolicy(apply_fazed),
];

fn open_window(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if depth != 0 || !matches!(spell, 13750 | 121471) || !auras::has_aura(world, caster, TALENT) {
        return;
    }
    let store = world.resource::<SpellStore>();
    let cloud_duration = store.get(CLOUD).and_then(|spell| spell.misc.as_ref())
        .and_then(|misc| misc.duration).expect("native Cloud Cover duration").duration_ms;
    let mut window = store.get(WINDOW).expect("native Cloud Cover aura").clone();
    let extra_index = if spell == 13750 { 0 } else { 1 };
    let extra = spell_cast::scripts::script_effect_value(world, caster, TALENT, extra_index) as i32;
    window.misc.as_mut().and_then(|misc| misc.duration.as_mut())
        .expect("native Cloud Cover aura duration").duration_ms = cloud_duration + extra;
    // SimC76f8118 sc_rogue.cpp3579/5553/11091–11098: the cooldown
    // earns the native cloud duration plus its native exit-persistence time.
    auras::apply_aura_with_spell(world, &AuraApplication {
        caster, target: caster, spell_id: WINDOW,
        effect_mask: auras::aura_effect_mask(&window), source: AuraSource::Cast,
    }, &window);
}

fn apply_fazed(world: &mut World, caster: Entity, target: Entity, spell: Option<u32>, amount: f32, _: bool) {
    if amount <= 0.0 || !auras::has_aura(world, caster, WINDOW) {
        return;
    }
    if let Some(id) = spell {
        let rogue_spell = world.resource::<SpellStore>().get(id)
            .and_then(|spell| spell.class_options.as_ref())
            .is_some_and(|class| class.spell_class_set == 8);
        if !rogue_spell { return; }
    }
    let now = spell_cast::now(world);
    if world.get::<CloudCoverTargets>(caster).is_none() {
        world.entity_mut(caster).insert(CloudCoverTargets::default());
    }
    let mut targets = world.get_mut::<CloudCoverTargets>(caster).unwrap();
    if targets.0.get(&target).is_some_and(|ready| *ready > now) { return; }
    targets.0.insert(target, now + 0.001);
    drop(targets);
    let store = world.resource::<SpellStore>();
    let duration = store.get(CLOUD).and_then(|spell| spell.misc.as_ref())
        .and_then(|misc| misc.duration).expect("native Cloud Cover duration").duration_ms;
    let mut fazed = store.get(FAZED).expect("native Fazed").clone();
    fazed.misc.as_mut().and_then(|misc| misc.duration.as_mut()).expect("native Fazed duration").duration_ms = duration;
    // sc_rogue.cpp8434–8465/8720–8729: each eligible landed attack
    // earns owned Fazed with the cloud's duration and per-target1ms cooldown.
    auras::apply_aura_with_spell(world, &AuraApplication {
        caster, target, spell_id: FAZED, effect_mask: auras::aura_effect_mask(&fazed), source: AuraSource::Cast,
    }, &fazed);
}
