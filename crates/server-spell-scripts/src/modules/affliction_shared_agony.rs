//! Shared Agony casts one native twin, selecting by owned remaining duration.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::random::seq::SliceRandom;
use combat::spell_cast::{
    self,
    script_registry::SpellScriptHook,
    scripts::AfterCastHook,
    targets::{self, Relation},
};
use combat::spell_info::SpellInfo;

const AGONY: u32 = 980;
const SHARED_AGONY: u32 = 1259825;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::NativeAfterCast(after_cast),
];

fn declared_reach(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    if spell.id != AGONY {
        return Default::default();
    }
    combat::spell_cast::script_registry::ScriptReach::target(
        spell_cast::scripts::script_effect_value(world, caster, SHARED_AGONY, 1),
    )
}

#[derive(Component)]
struct CastingTwin;

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == AGONY).then_some(share_agony)
}

fn owned_remaining(world: &World, caster: Entity, target: Entity, now: f64) -> f64 {
    world
        .get::<ActiveAuras>(target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == caster && aura.spell_id == AGONY && aura.stacks > 0)
        })
        .map_or(0.0, |aura| {
            aura.expires_at
                .map_or(f64::INFINITY, |expiry| (expiry - now).max(0.0))
        })
}

// SimC76f8118 warlock.cpp1274–1320: randomize first, then prefer the
// shortest owned Agony duration. The primary cannot be selected again.
fn select_twin(world: &mut World, caster: Entity, primary: Entity) -> Option<Entity> {
    let origin = targets::position(world, primary)?;
    let radius = spell_cast::scripts::script_effect_value(world, caster, SHARED_AGONY, 1);
    let mut candidates = targets::area_units(world, caster, origin, radius, Relation::Enemy, 0);
    candidates.retain(|&target| target != primary);
    candidates.shuffle(&mut combat::random::thread_rng());
    let now = combat::auras::aura_now(world);
    candidates.sort_by(|&left, &right| {
        owned_remaining(world, caster, left, now)
            .total_cmp(&owned_remaining(world, caster, right, now))
    });
    candidates.first().copied()
}

// SimC76f8118 actions1419–1450: a real root casts one background twin,
// with no additional payment. The twin cannot itself create another twin.
fn share_agony(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let selected = world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.caster == caster && aura.spell_id == SHARED_AGONY)
    });
    if !selected || world.get::<CastingTwin>(caster).is_some() {
        return;
    }
    let Some(primary) = spell.cast_primary_target else {
        return;
    };
    let Some(twin) = select_twin(world, caster, primary) else {
        return;
    };
    world.entity_mut(caster).insert(CastingTwin);
    let applied = spell_cast::try_cast_triggered(world, caster, AGONY, Some(twin), 1);
    world.entity_mut(caster).remove::<CastingTwin>();
    if !applied {
        warn!(?caster, ?primary, ?twin, "Native Shared Agony twin refused");
    }
}
