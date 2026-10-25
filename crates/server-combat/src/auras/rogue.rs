//! Missing Rogue aura relationships; native proc admission and payloads stay authoritative.
use bevy::prelude::*;

use super::scripts::{AuraScript, AuraScripts};
use super::{ActiveAuras, AuraInstance};
use crate::spell_cast::{self, targets::Relation};

const DEADLY_POISON: u32 = 2823;
const DEADLY_INSTANT: u32 = 113780;

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        DEADLY_POISON,
        AuraScript {
            on_proc: Some(apply_deadly_poison),
            ..Default::default()
        },
    );
}

fn apply_deadly_poison(
    world: &mut World,
    caster: Entity,
    aura: &AuraInstance,
    target: Option<Entity>,
) {
    let Some(target) = target else {
        return;
    };
    if !spell_cast::targets::relation_matches(world, caster, target, Relation::Enemy) {
        return;
    }
    let dot = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Deadly Poison coating must retain effect0")
        .trigger_spell;
    assert_ne!(dot, 0, "native Deadly Poison effect0 must name its DoT");
    let now = spell_cast::now(world);
    let ticking = world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().any(|active| {
            active.caster == caster
                && active.spell_id == dot
                && active.expires_at.is_none_or(|at| at > now)
        })
    });
    // SimC76f8118 sc_rogue.cpp2981–3005: the first proc applies the native
    // effect0 DoT; a repeat against this caster's ticking DoT also hits113780.
    // Proc masks/chance, spell modifiers, coefficients and duration remain native.
    if ticking {
        spell_cast::cast_triggered(world, caster, DEADLY_INSTANT, Some(target), 1);
    }
    spell_cast::cast_triggered(world, caster, dot, Some(target), 1);
}
