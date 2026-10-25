//! Source-backed cast procs using native chance, amount and buff records.
use bevy::prelude::*;
use shared::components::{ActiveSpec, PowerType};

use crate::spell_cast::{
    self,
    effects::{self, EffectCtx},
};
use crate::spell_info::SpellStore;

struct CastGrant {
    spec: u32,
    parents: &'static [u32],
    talent: u32,
    chance_effect: u8,
    amount_effect: u8,
    power: PowerType,
    buff: u32,
}

const GRANTS: &[CastGrant] = &[
    // SimC76f8118 sc_paladin.cpp281–285/317–323: both forms of Wrath
    // roll Walk Into Light, grant effect2 Holy Power and Blessing of An'she.
    // sc_paladin.cpp3792–3793 selects native445206 for Ret.
    CastGrant {
        spec: 70,
        parents: &[31884, 454351],
        talent: 1263782,
        chance_effect: 0,
        amount_effect: 1,
        power: PowerType::HolyPower,
        buff: 445206,
    },
];

pub(super) fn on_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    for grant in GRANTS.iter().filter(|grant| grant.parents.contains(&spell)) {
        let matching_spec = world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == grant.spec);
        if matching_spec && crate::auras::has_aura(world, caster, grant.talent) {
            roll_grant(world, caster, grant, depth);
        }
    }
}

fn roll_grant(world: &mut World, caster: Entity, grant: &CastGrant, depth: u8) {
    let chance = super::script_effect_value(world, caster, grant.talent, grant.chance_effect);
    if crate::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    let amount = super::script_effect_value(world, caster, grant.talent, grant.amount_effect);
    let spell = world
        .resource::<SpellStore>()
        .get(grant.talent)
        .expect("native cast grant retains its source talent")
        .clone();
    let ctx = EffectCtx {
        caster,
        spell: &spell,
        depth,
        dest: None,
        chain_multiplier: 1.0,
    };
    effects::energize_unit(world, &ctx, caster, grant.power, amount, false);
    if !spell_cast::try_cast_triggered(world, caster, grant.buff, Some(caster), depth + 1) {
        warn!(?caster, buff = grant.buff, "Native cast-grant buff refused");
    }
}
