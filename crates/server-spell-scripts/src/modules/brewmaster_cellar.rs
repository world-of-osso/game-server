//! SimC76f8118 sc_monk.cpp3390–3467: Exploding Keg earns Empty the Cellar.
use super::{brewmaster_cooldowns::brew_reduction, combat};
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraSource, RemoveReason};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{
    self,
    script_registry::SpellScriptHook,
    scripts::{AfterCastHook, script_effect_value},
};
use combat::spell_info::{SpellInfo, SpellStore};

const TALENT: u32 = 1262329;
const BUFF: u32 = 1262768;
const DRIVER: u32 = 1263438;
const DAMAGE: u32 = 1262765;

struct PendingHit {
    at: f64,
    target: Entity,
}
#[derive(Component, Default)]
struct PendingHits(Vec<PendingHit>);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterResourceValues(cast_hook),
    SpellScriptHook::ActionAvailable(available),
    SpellScriptHook::AfterNativeCastUpdate(update),
];

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    match spell {
        325153 => Some(earn),
        DRIVER => Some(spend),
        _ => None,
    }
}

fn available(world: &World, caster: Entity, spell: u32) -> bool {
    spell != DRIVER || auras::has_aura(world, caster, BUFF)
}

fn earn(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == TALENT)
    }) {
        return;
    }
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: BUFF,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn spend(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == BUFF && aura.caster == caster
    });
    let count = script_effect_value(world, caster, TALENT, 0) as usize;
    let period = world
        .resource::<SpellStore>()
        .get(DRIVER)
        .expect("native Cellar driver")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Cellar period")
        .aura_period_ms;
    let now = spell_cast::now(world);
    let mut entity = world.entity_mut(caster);
    let mut pending = entity.entry::<PendingHits>().or_default().into_mut();
    pending.0.extend((0..count).map(|index| PendingHit {
        at: now + f64::from(period) * index as f64 / 1000.0,
        target,
    }));
}

fn update(world: &mut World) {
    let now = spell_cast::now(world);
    let casters: Vec<_> = world
        .query_filtered::<Entity, With<PendingHits>>()
        .iter(world)
        .collect();
    for caster in casters {
        let due = {
            let mut pending = world.get_mut::<PendingHits>(caster).unwrap();
            let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut pending.0)
                .into_iter()
                .partition(|hit| hit.at <= now);
            pending.0 = later;
            due
        };
        for hit in due {
            if !spell_cast::try_cast_triggered(world, caster, DAMAGE, Some(hit.target), 1) {
                warn!(?caster, target=?hit.target, "Empty the Cellar damage refused");
            }
            let reduction = f64::from(script_effect_value(world, caster, TALENT, 1)) / 1000.0;
            brew_reduction(world, caster, reduction);
        }
    }
}
