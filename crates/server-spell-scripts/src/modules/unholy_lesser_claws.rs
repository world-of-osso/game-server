//! Real lesser ghouls spend and regenerate energy for their native attacks.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::AfterCastHook, summon::SummonedCreature};
use combat::spell_info::SpellInfo;
use combat::stats::CharacterStats;

#[derive(Component)]
struct GhoulEnergy { value: f64, updated: f64 }

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SummonInitialized(initialize),
    SpellScriptHook::AfterCastUpdate(regenerate),
    SpellScriptHook::GuardianActionEligible(eligible),
    SpellScriptHook::AuraRollAfterCast(after_cast),
];

fn initialize(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<SummonedCreature>(actor) else { return; };
    if !matches!(summon.spell_id, 275430 | 1294026) { return; }
    // SimC76f8118 sc_death_knight.cpp3156–3180: lesser ghouls start with
    // 100 Energy and regenerate 10 per second with owner haste.
    let now = combat::auras::aura_now(world);
    world.entity_mut(actor).insert(GhoulEnergy { value: 100.0, updated: now });
}

fn regenerate(world: &mut World) {
    let now = combat::auras::aura_now(world);
    let updates: Vec<_> = world.query::<(Entity, &SummonedCreature, &GhoulEnergy)>().iter(world)
        .map(|(actor, summon, energy)| {
            let haste = world.get::<CharacterStats>(summon.owner).expect("ghoul owner stats").haste_pct;
            (actor, (energy.value + (now - energy.updated).max(0.0) * 10.0 * (1.0 + f64::from(haste) / 100.0)).min(100.0))
        }).collect();
    for (actor, value) in updates {
        let mut energy = world.get_mut::<GhoulEnergy>(actor).unwrap();
        energy.value = value;
        energy.updated = now;
    }
}

fn eligible(world: &World, actor: Entity, spell: u32, _: Entity) -> bool {
    if !matches!(spell, 199373 | 1278150) { return true; }
    let Some(energy) = world.get::<GhoulEnergy>(actor) else { return true; };
    let owner = world.get::<SummonedCreature>(actor).expect("lesser ghoul summon").owner;
    let transformed = combat::auras::has_aura(world, owner, 1235391);
    let outnumber = combat::auras::has_aura(world, owner, 1241705);
    // SimC76f8118 sc_death_knight.cpp3492–3567: attacks cost 40 Energy;
    // selected Outnumber changes claw to sweeping claws during transformation.
    energy.value >= 40.0 && (spell == 1278150) == (outnumber && transformed)
}

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 199373 | 1278150).then_some(spend)
}

fn spend(world: &mut World, actor: Entity, _: &SpellInfo) {
    if let Some(mut energy) = world.get_mut::<GhoulEnergy>(actor) { energy.value -= 40.0; }
}
