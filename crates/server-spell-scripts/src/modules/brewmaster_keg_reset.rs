//! SimC76f8118 sc_monk.cpp3412/4517–4522: restore Keg Smash on these triggers.
use super::combat;
use bevy::prelude::*;
use combat::auras::AuraInstance;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{
    effects::EffectCtx, restore_charges, script_registry::SpellScriptHook, scripts::AfterCastHook,
};
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ActualDamageMultiplier(primary_damage),
    SpellScriptHook::AfterAuraApplication(empty_barrel),
    SpellScriptHook::AfterResourceValues(cast_hook),
];

fn primary_damage(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) -> f32 {
    if ctx.spell.id != 121253 || ctx.spell.cast_primary_target != Some(target) {
        return 1.0;
    }
    // SimC76f8118 sc_monk.cpp2530–2545: the selected talent adds a
    // primary bonus; batch6::keg_smash_plan already applies the spell's own.
    let stormstout = world.get::<TraitLoadout>(ctx.caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == 383707)
    });
    let talent = if stormstout {
        world
            .resource::<SpellStore>()
            .get(383707)
            .expect("Stormstout talent")
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("Stormstout primary damage effect")
            .base_points
    } else {
        0.0
    };
    1.0 + talent / 100.0
}

fn restore_keg(world: &mut World, caster: Entity) {
    let category = world
        .resource::<SpellStore>()
        .get(121253)
        .expect("Keg Smash spell exists")
        .categories
        .as_ref()
        .expect("Keg Smash category exists")
        .charge_category;
    assert_ne!(category, 0, "Keg Smash must have a charge category");
    restore_charges(world, caster, category, 1);
}

fn empty_barrel(world: &mut World, target: Entity, aura: &AuraInstance) {
    if aura.spell_id != 1265307 || target != aura.caster {
        return;
    }
    if world.get::<TraitLoadout>(target).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == 1265138)
    }) {
        restore_keg(world, target);
    }
}

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    (spell == 325153).then_some(exploding_keg)
}

fn exploding_keg(world: &mut World, caster: Entity, _: &SpellInfo) {
    restore_keg(world, caster);
}
