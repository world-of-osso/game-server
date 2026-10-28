//! Vengeance Immolation Aura initial hit and Fallout fragments.
use super::combat;
use bevy::prelude::*;
use combat::auras::AuraInstance;
use combat::soul_fragments::{self, FragmentKind};
use combat::spell_cast::{
    self,
    script_registry::{HitHook, SpellScriptHook},
    targets::Relation,
};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterAuraApplication(initial_damage),
    SpellScriptHook::EffectHit(effect_hit),
];

fn initial_damage(world: &mut World, actor: Entity, aura: &AuraInstance) {
    if aura.spell_id != 258920
        || aura.caster != actor
        || !is_vengeance(world, actor)
        || is_precombat(world, actor)
    {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp8927–8954: each functional aura start emits the initial hit.
    if !spell_cast::try_cast_triggered(world, actor, 258921, None, 1) {
        error!(?actor, "Vengeance Immolation initial damage refused");
    }
}
fn is_vengeance(world: &World, actor: Entity) -> bool {
    world
        .get::<ActiveSpec>(actor)
        .is_some_and(|spec| spec.0 == 581)
}
#[cfg(feature = "test-support")]
fn is_precombat(world: &World, actor: Entity) -> bool {
    world
        .get::<combat::simc_apl_native::FixturePrecombat>(actor)
        .is_some()
}
#[cfg(not(feature = "test-support"))]
fn is_precombat(_: &World, _: Entity) -> bool {
    false
}

fn effect_hit(spell: u32, index: u8) -> Option<HitHook> {
    match (spell, index) {
        (258921, 0) => Some(initial_hit),
        (258922, 0) => Some(extend_charred_flesh),
        _ => None,
    }
}
fn initial_hit(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    fallout(world, caster);
    extend_charred_flesh(world, caster, target, depth);
}
fn extend_charred_flesh(world: &mut World, caster: Entity, target: Entity, _: u8) {
    if !is_vengeance(world, caster)
        || !world
            .get::<combat::class_progression::TraitLoadout>(caster)
            .is_some_and(|traits| {
                traits
                    .active_talent_spells()
                    .any(|talent| talent.spell_id == 336639)
            })
    {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp4739–4750: each landed Immolation
    // impact extends the caster's actual Fiery Brand and Sigil of Flame.
    let milliseconds = spell_cast::scripts::script_effect_value(world, caster, 336639, 0);
    assert!(
        milliseconds >= 0.0 && milliseconds.fract() == 0.0,
        "invalid Charred Flesh duration"
    );
    combat::auras::extend_owned_duration(
        world,
        caster,
        target,
        &[207771, 204598],
        milliseconds as u32,
    );
}
fn fallout(world: &mut World, caster: Entity) {
    if !is_vengeance(world, caster)
        || !world
            .get::<combat::class_progression::TraitLoadout>(caster)
            .is_some_and(|traits| {
                traits
                    .active_talent_spells()
                    .any(|talent| talent.spell_id == 227174)
            })
    {
        return;
    }
    let Some(center) = spell_cast::targets::position(world, caster) else {
        error!(?caster, "Fallout caster position missing");
        return;
    };
    let radius = world
        .resource::<combat::spell_info::SpellStore>()
        .get(258921)
        .and_then(|spell| spell.effects.iter().find(|effect| effect.index == 0))
        .and_then(|effect| effect.radius[1])
        .expect("native Immolation radius")
        .radius_max;
    let targets =
        spell_cast::targets::area_units(world, caster, center, radius, Relation::Enemy, 0).len();
    // SimC76f8118 sc_demon_hunter.cpp4718–4736: one target is guaranteed;
    // each impact in a multi-target hit has a 60% chance.
    if targets > 1 && combat::random::random::<f64>() >= 0.60 {
        return;
    }
    if let Err(reason) = soul_fragments::spawn(world, caster, FragmentKind::Lesser, false) {
        error!(%reason, "Fallout fragment spawn failed");
    }
}
