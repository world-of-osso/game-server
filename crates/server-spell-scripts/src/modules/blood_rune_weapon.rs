//! Insatiable Blade's Bone Shield stacks on Dancing Rune Weapon.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraSource};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{ActiveSpec, Npc, UnitSummonedBy};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterForegroundSpecial(grant_bones),
    SpellScriptHook::AfterForegroundSpecial(copy_weapon_actions),
];

// SimC76f8118 sc_death_knight.cpp9523–9527/13955–13965 summons the
// selected second weapon and mirrors the owner's Bone Shield generators.
fn copy_weapon_actions(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 250)
    {
        return;
    }
    if spell.id == 49028 {
        if world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(377668))
        {
            spell_cast::cast_triggered(world, caster, 1237128, Some(caster), 1);
        }
        return;
    }
    if !matches!(spell.id, 195292 | 195182) {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    let weapons: Vec<Entity> = world
        .query::<(Entity, &Npc, &UnitSummonedBy)>()
        .iter(world)
        .filter(|(_, npc, owner)| npc.template_id == 27893 && owner.0 == caster.to_bits())
        .map(|(entity, _, _)| entity)
        .collect();
    for weapon in weapons {
        spell_cast::cast_triggered(world, weapon, spell.id, Some(target), 1);
    }
}

// SimC76f8118 sc_death_knight.cpp9500–9521 grants Insatiable Blade's
// authored effect2 stacks when Dancing Rune Weapon is cast.
fn grant_bones(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != 49028
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 250)
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(377637))
    {
        return;
    }
    let stacks = world
        .resource::<SpellStore>()
        .get(377637)
        .expect("Insatiable Blade talent")
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("Insatiable Blade stacks")
        .base_points as u32;
    for _ in 0..stacks {
        auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: 195181,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
}
