//! 70448 Courage of the White Tiger 443087 for Mistweaver (`$?c2`): Tiger Palm, Vivify and
//! Sheilun's Gift roll its RPPM (144: 4/min, hasted); Invoke Chi-Ji or Yu'lon applies 460127,
//! which makes the next activation certain. Xuen claws a nearby enemy (457917) after SimC
//! 76f8118's 2 s travel (sc_monk.cpp3565; WCL claws land 2.02–2.04 s after the cast) and heals
//! one nearby ally (443106) for 443087 s2% of the claw's damage; Mistweaver Monk 1258138
//! effect 22 raises s2 from 100 to 200 through label 3429. 443106 ignores healing modifiers
//! (attr6 0x08000000) and WCL logs no critical heal.
//! WCL `zCXNH4Gf1MPwvcZg` f10 a10 (64 claws) and `6Lg4hCVtj12JNbXc` f41 a730 (83): every heal is
//! 2.600× the claw's post-mitigation damage across 20.5–26.5% armor mitigation, so the basis is
//! damage done after armor (SimC heals from pre-mitigation `result_total`; retail wins).
//! INFERRED (residual): the remaining 1.30× of 2.600/2.00 has no DB2 source and is not applied;
//! retire when a decomp answer or SimC implementation names it.
//! INFERRED: the claw takes the nearest enemy already in combat with the monk (a heal never
//! pulls an idle enemy) and the heal an injured-first random ally (as Way of the Crane); WCL
//! heals went to all five members (18/18/10/10/8). Without such an enemy nothing is rolled or
//! spent. One cast activates at most once: a certain activation spends 460127 instead of rolling.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::spell_cast::{
    self,
    effects::{self, EffectCtx},
    queued_triggers,
    script_registry::SpellScriptHook,
    scripts::{AfterCastHook, script_effect_value},
    targets::{self, Relation},
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::ActiveSpec;

const TALENT: u32 = 443087;
const CERTAIN: u32 = 460127;
const CLAW: u32 = 457917;
const HEAL: u32 = 443106;
const MISTWEAVER: u32 = 270;
/// Tiger Palm, Vivify and the Sheilun's Gift it resolves to in the replay profile.
const SOURCES: [u32; 3] = [100780, 116670, 399491];
/// Invoke Chi-Ji, the Red Crane and Invoke Yu'lon, the Jade Serpent.
const INVOKES: [u32; 2] = [325197, 322118];
/// SimC76f8118 sc_monk.cpp3565 `time_to_travel = 2_s`.
const TRAVEL_MS: u32 = 2000;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterResourceValues(after_cast),
    SpellScriptHook::OwnsPolicyProc(|spell| matches!(spell, TALENT | CERTAIN)),
    SpellScriptHook::ResolvedDamage(heal_from_claw),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    if INVOKES.contains(&spell) {
        Some(make_next_certain)
    } else if SOURCES.contains(&spell) {
        Some(activate)
    } else {
        None
    }
}

fn make_next_certain(world: &mut World, caster: Entity, _: &SpellInfo) {
    if is_selected(world, caster)
        && !spell_cast::try_cast_triggered(world, caster, CERTAIN, Some(caster), 1)
    {
        warn!(?caster, "Courage of the White Tiger certainty failed");
    }
}

fn activate(world: &mut World, caster: Entity, parent: &SpellInfo) {
    if !is_selected(world, caster) {
        return;
    }
    let Some(enemy) = find_engaged_enemy(world, caster) else {
        return;
    };
    if !roll_activation(world, caster) {
        return;
    }
    let ctx = EffectCtx {
        caster,
        spell: parent,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    queued_triggers::schedule_full_trigger(world, &ctx, enemy, CLAW, TRAVEL_MS);
}

/// The nearest enemy within 457917's range that the monk is in combat with.
fn find_engaged_enemy(world: &mut World, caster: Entity) -> Option<Entity> {
    let center = targets::position(world, caster)?;
    let (_, range) = targets::spell_range(
        world
            .resource::<SpellStore>()
            .get(CLAW)
            .expect("Courage claw missing"),
        true,
    );
    let opponents: Vec<_> = combat::combat_refs::active_opponents(world, caster).collect();
    targets::area_units(world, caster, center, range, Relation::Enemy, 0)
        .into_iter()
        .find(|enemy| opponents.contains(enemy))
}

fn roll_activation(world: &mut World, caster: Entity) -> bool {
    if let Some(certain) = own_instance(world, caster, CERTAIN) {
        auras::spend_charge(world, caster, certain);
        return true;
    }
    let talent = own_instance(world, caster, TALENT).expect("selected Courage aura");
    auras::procs::try_native_policy_proc(world, caster, talent)
}

fn heal_from_claw(world: &mut World, ctx: &EffectCtx<'_>, _: Entity, _: bool, damage: f32) {
    if ctx.spell.id != CLAW || damage <= 0.0 {
        return;
    }
    let caster = ctx.caster;
    let spell = world
        .resource::<SpellStore>()
        .get(HEAL)
        .expect("Courage heal missing")
        .clone();
    let radius = targets::radius(&spell.effects[0], 1);
    let Some(ally) = super::mistweaver_way_of_the_crane::select_ally(world, caster, radius) else {
        return;
    };
    let amount = damage * script_effect_value(world, caster, TALENT, 1) / 100.0;
    let heal = EffectCtx {
        caster,
        spell: &spell,
        depth: ctx.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    effects::apply_heal(world, &heal, ally, amount, false, false);
}

fn is_selected(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == MISTWEAVER)
        && own_instance(world, caster, TALENT).is_some()
}

fn own_instance(world: &World, unit: Entity, spell: u32) -> Option<u32> {
    world
        .get::<ActiveAuras>(unit)?
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == unit)
        .map(|aura| aura.instance_id)
}
