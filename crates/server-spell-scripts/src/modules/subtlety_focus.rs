//! SimC76f8118 sc_rogue.cpp7345–7362/7381–7390: selected Shadow Focus
//! lasts while Stealth, Vanish or Shadow Dance remains.
use super::{combat, has_talent};
use bevy::prelude::*;
use combat::auras::scripts::AuraScripts;
use combat::auras::{self, AuraInstance, RemoveReason};
use combat::spell_cast;

const TALENT: u32 = 108209;
const FOCUS: u32 = 112942;
const STEALTH: u32 = 1784;
const STEALTH_CHILD: u32 = 158185;
const VANISH_BUFF: u32 = 11327;
pub(super) const DANCE: u32 = 185313;

pub(super) fn grant(world: &mut World, unit: Entity, aura: &AuraInstance) {
    let stealth_like = matches!(aura.spell_id, STEALTH | VANISH_BUFF | DANCE);
    if stealth_like && unit == aura.caster && has_talent(world, unit, TALENT) {
        spell_cast::cast_triggered(world, unit, FOCUS, Some(unit), 1);
    }
}

pub(super) fn register(scripts: &mut AuraScripts) {
    // Stealth's own removal slot is taken; its native child leaves with it.
    for spell in [STEALTH_CHILD, VANISH_BUFF] {
        let script = scripts.0.entry(spell).or_default();
        assert!(script.on_remove.is_none(), "{spell} removal hook taken");
        script.on_remove = Some(stealth_removed);
    }
}

fn stealth_removed(world: &mut World, unit: Entity, _aura: &AuraInstance, _reason: RemoveReason) {
    expire(world, unit);
}

pub(super) fn expire(world: &mut World, unit: Entity) {
    let remains = [STEALTH, VANISH_BUFF, DANCE]
        .into_iter()
        .any(|spell| auras::has_aura(world, unit, spell));
    if !remains {
        auras::remove_auras(world, unit, RemoveReason::Expired, |aura| {
            aura.spell_id == FOCUS && aura.caster == unit
        });
    }
}
