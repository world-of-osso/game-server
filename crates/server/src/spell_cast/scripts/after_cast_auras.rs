//! Source-qualified after-cast passives; payloads come from their native aura42.
//! SimC76f8118 sc_warrior.cpp7229/8427: Shield Wall triggers learned Last Stand.
use bevy::prelude::*;

use super::AfterCastHook;
use crate::auras::{ActiveAuras, handlers::PROC_TRIGGER_SPELL};
use crate::spell_info::SpellInfo;

const SHIELD_WALL: u32 = 871;
const LAST_STAND: u32 = 1243659;
const CONDITIONAL_AURA_TRIGGERS: &[(u32, u32)] = &[(SHIELD_WALL, LAST_STAND)];

pub(super) fn hook(spell: u32) -> Option<AfterCastHook> {
    CONDITIONAL_AURA_TRIGGERS
        .iter()
        .any(|(cast, _)| *cast == spell)
        .then_some(apply)
}

fn read_children(world: &World, caster: Entity, spell: u32) -> Vec<(u32, u32)> {
    let Some(auras) = world.get::<ActiveAuras>(caster) else {
        return Vec::new();
    };
    CONDITIONAL_AURA_TRIGGERS
        .iter()
        .filter(|(cast, _)| *cast == spell)
        .filter_map(|(_, learned)| {
            let aura = auras.iter().find(|aura| aura.spell_id == *learned)?;
            let effect = aura
                .effects
                .iter()
                .find(|effect| effect.aura_type == PROC_TRIGGER_SPELL)
                .expect("source-qualified after-cast passive requires native aura42");
            assert!(
                effect.trigger_spell != 0,
                "native after-cast passive requires a trigger spell"
            );
            Some((*learned, effect.trigger_spell))
        })
        .collect()
}

fn apply(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for (learned, child) in read_children(world, caster, spell.id) {
        if !crate::spell_cast::try_cast_triggered(world, caster, child, Some(caster), 1) {
            warn!(
                ?caster,
                parent = spell.id,
                learned,
                child,
                "Native after-cast aura refused"
            );
        }
    }
}
