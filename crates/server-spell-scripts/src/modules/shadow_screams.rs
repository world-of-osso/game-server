//! SimC76f8118 sc_priest_shadow.cpp1324–1327/2203–2204: every Madness execute,
//! including background Maddening Tentacles, extends the native Screams buff.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraSource};
use combat::spell_cast::script_registry::{Policy, Relationship, SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(ordinary_hook),
    SpellScriptHook::TriggeredResourceProducer(apply_screams),
    SpellScriptHook::Policies(&[Policy {
        name: "SIMC_SHADOW_SCREAMS_REAPPLICATION_EXTEND",
        spec: 258,
        talent: TALENT,
        relationship: Relationship::ReapplicationExtend { payload: BUFF },
    }]),
];
const MADNESS: u32 = 335467;
const TALENT: u32 = 375767;
const BUFF: u32 = 393919;

fn ordinary_hook(spell: u32) -> Option<combat::spell_cast::scripts::AfterCastHook> {
    (spell == MADNESS).then_some(apply_screams)
}

fn apply_screams(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != MADNESS || !auras::has_aura(world, caster, TALENT) {
        return;
    }
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: BUFF,
            effect_mask: 3,
            source: AuraSource::Cast,
        },
    );
}
