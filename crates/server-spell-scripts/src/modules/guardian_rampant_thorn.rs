//! SimC76f8118 sc_druid.cpp5974–5982 and 11376–11381.
use bevy::prelude::*;

use super::combat;
use combat::auras::{self, ActiveAuras, AuraInstance, scripts::AuraScripts};
use combat::random::seq::SliceRandom;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook, targets};
use combat::spell_info::SpellStore;

const TWO_PIECE: u32 = 1296607;
const SET: u32 = 1296608;
const FORMS: &[u32] = &[102558, 50334];

#[derive(Component)]
struct Extension {
    aura_id: u32,
    milliseconds: u32,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::AfterResolvedHits(after_cast),
    SpellScriptHook::EarlyAuraRegistration(register),
];

fn declared_reach(
    world: &World,
    caster: Entity,
    spell: &combat::spell_info::SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    if !matches!(spell.id, TWO_PIECE | SET | 1310213 | 102558 | 50334)
        || !combat::spell_cast::script_registry::reach_source_selected(world, caster, TWO_PIECE)
    {
        return Default::default();
    }
    let thorn = world
        .resource::<SpellStore>()
        .get(1301157)
        .expect("Rampant Thorn native spell");
    let range = thorn
        .misc
        .as_ref()
        .and_then(|misc| misc.range)
        .expect("Rampant Thorn range")
        .max_hostile;
    combat::spell_cast::script_registry::ScriptReach::caster(range)
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(1310213).or_default().on_periodic = Some(tick);
}

fn tick(world: &mut World, caster: Entity, aura: &AuraInstance, effect: u8) {
    if aura.caster != caster || effect != 0 {
        return;
    }
    let Some(origin) = targets::position(world, caster) else {
        return;
    };
    let spell = world
        .resource::<SpellStore>()
        .get(1301157)
        .expect("Rampant Thorn native spell")
        .clone();
    let range = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.range)
        .expect("Rampant Thorn range")
        .max_hostile;
    let radius = spell
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .and_then(|effect| effect.radius[1])
        .expect("Rampant Thorn secondary radius")
        .radius;
    let cap = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|a| a.spell_id == SET && a.caster == caster)
        })
        .and_then(|a| a.effects.iter().find(|e| e.index == 3))
        .expect("Rampant Thorn target cap")
        .amount
        .max(0.0) as usize;
    let mut candidates =
        targets::area_units(world, caster, origin, range, targets::Relation::Enemy, 0);
    candidates.shuffle(&mut combat::random::thread_rng());
    let Some(&primary) = candidates.first() else {
        return;
    };
    let Some(center) = targets::position(world, primary) else {
        return;
    };
    let nearby = targets::area_units(world, caster, center, radius, targets::Relation::Enemy, 0);
    let selected = candidates
        .into_iter()
        .filter(|target| *target != primary && nearby.contains(target))
        .take(cap);
    for (target, index) in std::iter::once((primary, 0)).chain(selected.map(|target| (target, 1))) {
        let mut child = spell.clone();
        child.effects.retain(|effect| effect.index == index);
        child.effects[0].implicit_targets = [6, 0];
        spell_cast::launch_triggered_spell(world, caster, &child, Some(target), 1);
    }
}

fn roll_gorestained_claws(world: &mut World, caster: Entity, depth: u8) {
    let effect = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == TWO_PIECE && aura.caster == caster)
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0));
    let Some(effect) = effect else { return };
    let (chance, child) = (effect.amount, effect.trigger_spell);
    // SimC76f8118 sc_druid.cpp5968–5972,11447–11449: each Thrash rolls the equipped set.
    if combat::random::random::<f32>() * 100.0 < chance {
        spell_cast::cast_triggered(world, caster, child, Some(caster), depth + 1);
    }
}

fn after_cast(world: &mut World, ctx: &EffectCtx<'_>) {
    let caster = ctx.caster;
    if ctx.spell.id != 77758 {
        return;
    }
    roll_gorestained_claws(world, caster, ctx.depth);
    let Some(auras) = world.get::<ActiveAuras>(caster) else {
        return;
    };
    let Some(set) = auras
        .iter()
        .find(|aura| aura.spell_id == SET && aura.caster == caster)
    else {
        return;
    };
    let value = |index| {
        set.effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("Guardian 4pc extension effect")
            .amount
            .max(0.0) as u32
    };
    let (add, cap) = (value(0), value(1));
    let ticking = auras
        .iter()
        .any(|aura| aura.spell_id == 1310213 && aura.caster == caster);
    if ticking {
        let ticker = world
            .resource::<SpellStore>()
            .get(1310213)
            .expect("Rampant Thorn ticker");
        let duration = spell_cast::duration_ms(world, caster, ticker).max(0) as u32;
        auras::extend_owned_duration(world, caster, caster, &[1310213], duration);
    } else {
        spell_cast::cast_triggered(world, caster, 1310213, Some(caster), 1);
    }
    let Some(auras) = world.get::<ActiveAuras>(caster) else {
        return;
    };
    let Some(form) = auras
        .iter()
        .find(|aura| FORMS.contains(&aura.spell_id) && aura.caster == caster)
    else {
        return;
    };
    let (id, form_spell) = (form.instance_id, form.spell_id);
    let previous = world
        .get::<Extension>(caster)
        .filter(|state| state.aura_id == id)
        .map_or(0, |state| state.milliseconds);
    if previous >= cap {
        return;
    }
    auras::extend_owned_duration(world, caster, caster, &[form_spell], add);
    world.entity_mut(caster).insert(Extension {
        aura_id: id,
        milliseconds: previous + add,
    });
}
