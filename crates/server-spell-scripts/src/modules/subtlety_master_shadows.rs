//! SimC76f8118 sc_rogue.cpp5572–5575/6114–6117/6146–6149/
//! 8532–8537/11196–11204: in-combat stance actions earn native Energy ticks.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraSource};
use combat::spell_cast::{
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
    scripts::script_effect_value,
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{CombatStatus, PowerType};

const TALENT: u32 = 196976;
const BUFF: u32 = 196980;
const STANCE_ACTIONS: &[u32] = &[1784, 1856, 185313];
pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterForegroundSpecial(earn_buff)];

fn earn_buff(world: &mut World, caster: Entity, source: &SpellInfo) {
    if !STANCE_ACTIONS.contains(&source.id)
        || !has_talent(world, caster, TALENT)
        || !world
            .get::<CombatStatus>(caster)
            .is_some_and(|status| status.0)
    {
        return;
    }
    let fresh = !auras::has_aura(world, caster, BUFF);
    let mut spell = world
        .resource::<SpellStore>()
        .get(BUFF)
        .expect("native Master of Shadows buff")
        .clone();
    // The pinned duration-only refresh preserves the existing tick phase.
    spell.cast_dont_reset_periodic_timer = true;
    auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: BUFF,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
        &spell,
    );
    if fresh {
        let immediate = script_effect_value(world, caster, BUFF, 1);
        if immediate > 0.0 {
            let ctx = EffectCtx {
                caster,
                spell: &spell,
                depth: 1,
                dest: None,
                chain_multiplier: 1.0,
            };
            effects::energize_unit(world, &ctx, caster, PowerType::Energy, immediate, false);
        }
    }
}
