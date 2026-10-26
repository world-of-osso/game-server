//! Source-owned Wither-tick Devil Fruit RPPM and Malevolence extension.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraSource, procs};
use combat::spell_cast::{
    effects::EffectCtx,
    script_registry::{Policy, Relationship, SpellScriptHook},
};
use combat::spell_info::SpellStore;

const WITHER: u32 = 445474;
const DEVIL_FRUIT: u32 = 1266805;
const MALEVOLENCE: u32 = 442726;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(owns_proc),
    SpellScriptHook::PeriodicPrimaryDamage(attempt_devil_fruit),
    // SimC76f8118 sc_warlock_init.cpp968–971: both the ordinary button
    // and the earned proc extend Malevolence rather than replacing it.
    SpellScriptHook::Policies(&[
        Policy {
            name: "SIMC_AFFLICTION_MALEVOLENCE_EXTEND",
            spec: 265,
            talent: 430014,
            relationship: Relationship::ReapplicationExtend {
                payload: MALEVOLENCE,
            },
        },
        Policy {
            name: "SIMC_DESTRUCTION_MALEVOLENCE_EXTEND",
            spec: 267,
            talent: 430014,
            relationship: Relationship::ReapplicationExtend {
                payload: MALEVOLENCE,
            },
        },
    ]),
];

fn owns_proc(spell: u32) -> bool {
    spell == DEVIL_FRUIT
}

fn attempt_devil_fruit(world: &mut World, ctx: &EffectCtx<'_>, _: Entity, _: f32, _: bool) {
    if ctx.spell.id != WITHER {
        return;
    }
    let instance = world.get::<ActiveAuras>(ctx.caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.caster == ctx.caster && aura.spell_id == DEVIL_FRUIT)
            .map(|aura| aura.instance_id)
    });
    // SimC76f8118 sc_warlock_actions.cpp1154–1159/init1475: each actual
    // owned Wither tick attempts the selected native RPPM, not a button cast.
    if let Some(instance) = instance
        && procs::try_native_policy_proc(world, ctx.caster, instance)
    {
        apply_earned_malevolence(world, ctx.caster);
    }
}

fn apply_earned_malevolence(world: &mut World, caster: Entity) {
    let store = world.resource::<SpellStore>();
    let duration_seconds = store
        .get(DEVIL_FRUIT)
        .expect("selected native Devil Fruit")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Devil Fruit duration")
        .base_points;
    let mut buff = store
        .get(MALEVOLENCE)
        .expect("native Malevolence buff")
        .clone();
    // Source actions1158 overrides only the proc's lifetime. Native buff
    // effects and ordinary hard-cast duration remain unchanged in the store.
    buff.misc
        .as_mut()
        .and_then(|misc| misc.duration.as_mut())
        .expect("native Malevolence duration")
        .duration_ms = (duration_seconds * 1000.0).round() as i32;
    auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: MALEVOLENCE,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
        &buff,
    );
}
